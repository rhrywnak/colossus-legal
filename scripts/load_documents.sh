#!/usr/bin/env bash
set -euo pipefail

# Simple loader for Colossus-Legal Documents via POST /documents
#
# Usage:
#   ./load_documents.sh case_manifest.txt
#
# Manifest format (pipe-delimited):
#   file_path | title | doc_type | created_at | description
#
# Example:
#   /data/cases/example_case_1/complaint.pdf | Complaint – Smith v. Jones | complaint | 2023-01-15T09:00:00Z | Initial complaint...

API_BASE_URL="${API_BASE_URL:-http://localhost:3403}"
DOCUMENTS_ENDPOINT="$API_BASE_URL/documents"

MANIFEST_FILE="${1:-}"

if [[ -z "$MANIFEST_FILE" ]]; then
  echo "Usage: $0 <manifest_file>" >&2
  exit 1
fi

if [[ ! -f "$MANIFEST_FILE" ]]; then
  echo "Manifest file not found: $MANIFEST_FILE" >&2
  exit 1
fi

# Check for required tools
if ! command -v curl >/dev/null 2>&1; then
  echo "Error: curl is required but not installed." >&2
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "Error: jq is required but not installed." >&2
  echo "On Ubuntu: sudo apt-get install jq" >&2
  exit 1
fi

echo "Using API_BASE_URL=${API_BASE_URL}"
echo "Posting documents from manifest: ${MANIFEST_FILE}"
echo

line_no=0

while IFS='|' read -r raw_path raw_title raw_type raw_created_at raw_desc; do
  line_no=$((line_no + 1))

  # Trim leading/trailing whitespace for each field
  file_path="$(echo "${raw_path:-}" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
  title="$(echo "${raw_title:-}" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
  doc_type="$(echo "${raw_type:-}" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
  created_at="$(echo "${raw_created_at:-}" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
  description="$(echo "${raw_desc:-}" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"

  # Skip empty lines and comments
  if [[ -z "$file_path" ]] || [[ "$file_path" =~ ^# ]]; then
    continue
  fi

  # Basic validation
  if [[ -z "$title" || -z "$doc_type" || -z "$created_at" ]]; then
    echo "Line ${line_no}: missing required fields (title/doc_type/created_at). Skipping." >&2
    continue
  fi

  # Optionally check file existence
  if [[ ! -f "$file_path" ]]; then
    echo "Line ${line_no}: WARNING - file does not exist: $file_path" >&2
    # Not fatal; we still store file_path as metadata.
  fi

  echo "Line ${line_no}: Creating document:"
  echo "  title      = $title"
  echo "  doc_type   = $doc_type"
  echo "  created_at = $created_at"
  echo "  file_path  = $file_path"
  echo

  # Build JSON body with jq to handle escaping safely
  json_body="$(jq -n \
    --arg title "$title" \
    --arg doc_type "$doc_type" \
    --arg created_at "$created_at" \
    --arg file_path "$file_path" \
    --arg description "$description" \
    '{
      title: $title,
      doc_type: $doc_type,
      created_at: $created_at,
      file_path: $file_path,
      description: $description
    }'
  )"

  # POST to /documents
  http_status=$(curl -s -o /tmp/doc_post_response.json -w "%{http_code}" \
    -X POST "$DOCUMENTS_ENDPOINT" \
    -H "Content-Type: application/json" \
    -d "$json_body")

  if [[ "$http_status" != "200" && "$http_status" != "201" ]]; then
    echo "Line ${line_no}: ERROR - HTTP status $http_status when creating document." >&2
    echo "Response:" >&2
    cat /tmp/doc_post_response.json >&2
    echo >&2
  else
    echo "Line ${line_no}: Document created successfully."
    echo "Response:"
    cat /tmp/doc_post_response.json
    echo
  fi

  echo "------------------------------------------------------------"
  echo

done < "$MANIFEST_FILE"

echo "Done."

