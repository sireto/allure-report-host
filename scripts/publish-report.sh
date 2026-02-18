#!/usr/bin/env bash
# scripts/publish-report.sh
# Upload Playwright/Allure test reports to the report server

set -euo pipefail

# Configuration
: "${REPORT_API_URL:=}"
: "${REPORT_API_KEY:=}"

usage() {
    cat <<EOF
Usage: ./scripts/publish-report.sh [options] [project] [branch] [report_name] [type] [path]

Arguments:
  project       Project name (default: current directory name)
  branch        Git branch (default: current git branch or 'local')
  report_name   Report name (default: auto-generated from date/git commit)
  type          Report type: 'allure' or 'raw' (default: allure)
  path          Path to report folder or .zip file (default: ./allure-results)

Options:
  --url URL     API endpoint URL
  --key KEY     API key for authentication
  --dry-run     Show what would be executed without uploading
  --verbose,-v  Show verbose curl output
  --help,-h     Show this help message
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

# Parameter assignments
project_name="${1:-$(basename "$(pwd)")}"
branch="${2:-$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'local')}"
report_name="${3:-$(date +'%Y-%m-%d-%H%M')-$(git rev-parse --short HEAD 2>/dev/null || echo '')}"
report_type="${4:-allure}"
input_path="${5:-./allure-results}"

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
    -F "branch=$branch"
    -F "report_name=$report_name"
    -F "type=$report_type"
    -F "file=@$upload_file"
    --progress-bar
)

[[ -n "$REPORT_API_KEY" ]] && curl_args+=(-H "X-API-Key: $REPORT_API_KEY")
[[ $verbose -eq 1 ]] && curl_args+=(-v)

# Show summary
echo "Publishing report:"
echo "  URL:           $REPORT_API_URL"
echo "  Project:       $project_name"
echo "  Branch:        $branch"
echo "  Report name:   $report_name"
echo "  Type:          $report_type"
echo "  File:          $upload_file ($(du -h "$upload_file" | cut -f1))"

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