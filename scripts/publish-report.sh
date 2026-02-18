#!/usr/bin/env bash
# scripts/publish-report.sh
# Upload Playwright/Allure test reports to the report server

set -euo pipefail

# Configuration
: "${REPORT_API_URL:=}"
: "${REPORT_API_KEY:=}"

usage() {
    cat <<EOF
Usage: ./scripts/publish-report.sh [options] [project] [report_name] [type] [path]
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
        --url) REPORT_API_URL="$2"; shift 2 ;;
        --key) REPORT_API_KEY="$2"; shift 2 ;;
        *) break ;;
    esac
done

# Fixed variable assignments
project_name="${1:-$(basename "$(pwd)")}"
report_name="${2:-$(date +'%Y-%m-%d-%H%M')-$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'local')-$(git rev-parse --short HEAD 2>/dev/null || echo '')}"
report_type="${3:-allure}"
input_path="${4:-./allure-results}"

# Validate type
if [[ "$report_type" != "allure" && "$report_type" != "raw" ]]; then
    echo "Error: type must be 'allure' or 'raw'" >&2
    exit 1
fi

# Prepare file to upload
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

# Build curl command
curl_args=(
    curl
    -X POST "$REPORT_API_URL"
    -F "project_name=$project_name"
    -F "report_name=$report_name"
    -F "type=$report_type"
    -F "file=@$upload_file"
    --progress-bar
)

[[ -n "$REPORT_API_KEY" ]] && curl_args+=(-H "X-API-Key: $REPORT_API_KEY")
[[ $verbose -eq 1 ]] && curl_args+=(-v)

# Show summary
echo "Publishing report:"
echo "  URL:          $REPORT_API_URL"
echo "  Project:      $project_name"
echo "  Report name:  $report_name"
echo "  Type:         $report_type"
echo "  File:         $upload_file ($(du -h "$upload_file" | cut -f1))"

# Execute upload
echo "→ Starting upload..."

if [[ $dry_run -eq 1 ]]; then
    echo -e "\nDRY RUN MODE — would execute:"
    printf '  %q ' "${curl_args[@]}"
    echo -e " -w '\\n→ HTTP %{http_code}\\n' -o /dev/null\n"
    exit 0
fi

# Add the remaining flags directly to the array (safest)
curl_args+=(
    -w "\n→ HTTP %{http_code}\n"
    -o /dev/null
)

if "${curl_args[@]}"; then
    echo -e "\n→ Upload successful"
else
    echo -e "\n→ Upload failed (HTTP code above or network error)" >&2
    exit 1
fi