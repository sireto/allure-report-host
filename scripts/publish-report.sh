#!/usr/bin/env bash
# scripts/publish-report.sh
# Upload Playwright/Allure test reports to the report server

set -euo pipefail

# Configuration
: "${SERVER_URL:=}"
: "${REPORT_API_SECRET:=}"
: "${MAX_FILE_SIZE_MB:=500}"

usage() {
    cat <<EOF
Usage: ./scripts/publish-report.sh [options] [project] [branch] [report_name] [type] [path]
...
EOF
}

dry_run=0
verbose=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --help|-h) usage; exit 0 ;;
        --dry-run) dry_run=1; shift ;;
        --verbose|-v) verbose=1; shift ;;
        --url) SERVER_URL="$2"; shift 2 ;;
        --key) REPORT_API_SECRET="$2"; shift 2 ;;
        *) break ;;
    esac
done

project_name="${1:-$(basename "$(pwd)")}"
branch="${2:-$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'local')}"
report_name="${3:-$(date +'%Y-%m-%d-%H%M')-$(git rev-parse --short HEAD 2>/dev/null || echo '')}"
report_type="${4:-allure}"
input_path="${5:-./allure-results}"

if [[ "$report_type" != "allure" && "$report_type" != "raw" ]]; then
    echo "Error: type must be 'allure' or 'raw'" >&2
    exit 1
fi

# Prepare file
if [[ -d "$input_path" ]]; then
    temp_zip="/tmp/report-$(date +%s).zip"
    echo "→ Zipping folder $input_path ..."
    zip -q -r "$temp_zip" "$input_path" || { echo "Zip failed"; exit 1; }
    upload_file="$temp_zip"
    trap 'rm -f "$temp_zip"' EXIT
elif [[ -f "$input_path" && "$input_path" =~ \.zip$ ]]; then
    upload_file="$input_path"
else
    echo "Error: Provide a folder or .zip file (got: $input_path)" >&2
    exit 1
fi

# File size check
file_size_bytes=$(stat -f%z "$upload_file" 2>/dev/null || stat -c%s "$upload_file" 2>/dev/null || echo "0")
file_size_mb=$((file_size_bytes / 1024 / 1024))

echo "→ File size: ${file_size_mb}MB (limit: ${MAX_FILE_SIZE_MB}MB)"

if (( file_size_mb > MAX_FILE_SIZE_MB )); then
    echo "Error: File too large (${file_size_mb}MB > ${MAX_FILE_SIZE_MB}MB)" >&2
    [[ "$upload_file" == /tmp/report-* ]] && rm -f "$upload_file"
    exit 1
fi

if [[ -z "$SERVER_URL" ]]; then
    echo "Error: SERVER_URL not set. Use --url or env var" >&2
    exit 1
fi

curl_args=(
    -X POST "$SERVER_URL/api/reports/upload"
    -F "project_name=$project_name"
    -F "branch=$branch"
    -F "report_name=$report_name"
    -F "type=$report_type"
    -F "file=@$upload_file"
    --progress-bar
)

[[ -n "$REPORT_API_SECRET" ]] && curl_args+=(-H "X-API-Key: $REPORT_API_SECRET")
[[ $verbose -eq 1 ]] && curl_args+=(-v)

# Summary
echo "Publishing report:"
echo "  URL:           $SERVER_URL"
echo "  Project:       $project_name"
echo "  Branch:        $branch"
echo "  Report name:   $report_name"
echo "  Type:          $report_type"
echo "  File:          $upload_file ($(du -h "$upload_file" | cut -f1))"

echo "→ Starting upload..."

response=$(mktemp)
trap 'rm -f "$response"' EXIT

# Capture ONLY the status code
http_code=$(curl "${curl_args[@]}" \
    -o "$response" \
    -w '%{http_code}' \
    -s -S --fail-with-body) || true   # don't exit on non-2xx

# Clean http_code (remove any whitespace/newlines)
http_code="${http_code//[[:space:]]/}"

if [[ -z "$http_code" || ! "$http_code" =~ ^[0-9]{3}$ ]]; then
    echo "Error: Could not capture valid HTTP code" >&2
    cat "$response" >&2
    exit 1
fi

if (( http_code >= 200 && http_code < 300 )); then
    echo -e "\n→ Upload successful! (HTTP $http_code)"
    if [[ -s "$response" ]]; then
        echo "→ Response:"
        jq . "$response" 2>/dev/null || cat "$response"
    fi

    if [[ -n "${GITHUB_TOKEN:-}" ]]; then
      commit_sha="${GITHUB_SHA:-$(git rev-parse HEAD)}"
      context="Allure Report"
      target_url="${SERVER_URL}/${project_name}/${branch}/${report_name}/index.html"
      description="Allure report for this build"
      state="success"

      curl -s -X POST \
        -H "Authorization: token ${GITHUB_TOKEN}" \
        -H "Content-Type: application/json" \
        -d "{\"state\": \"${state}\", \"target_url\": \"${target_url}\", \"description\": \"${description}\", \"context\": \"${context}\"}" \
        "https://api.github.com/repos/${GITHUB_REPOSITORY}/statuses/${commit_sha}"
    fi
else
    echo -e "\n→ Upload failed (HTTP $http_code)" >&2
    if [[ -s "$response" ]]; then
        echo "→ Error details:"
        jq . "$response" 2>/dev/null || cat "$response"
    fi
    exit 1
fi