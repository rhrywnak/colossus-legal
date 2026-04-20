#!/usr/bin/env bash
# new-migration.sh — Create a new migration file with a timestamp-based version.
#
# sqlx uses the numeric prefix of each migration filename as the version
# number (i64). Date-based prefixes (YYYYMMDD) collide when multiple
# migrations are created on the same day. Timestamp-based prefixes
# (YYYYMMDDHHMMSS) are unique to the second.
#
# This script is the ONLY way to create migration files. Never create
# them manually — the filename format matters.
#
# Usage:
#   ./scripts/new-migration.sh pipeline "add user preferences"
#   ./scripts/new-migration.sh main "create feedback table"
#
# Arguments:
#   $1 — Target database: "pipeline" or "main"
#   $2 — Description (lowercase, spaces OK — converted to underscores)
#
# Output:
#   Creates backend/pipeline_migrations/YYYYMMDDHHMMSS_description.sql
#   or     backend/migrations/YYYYMMDDHHMMSS_description.sql
#
# Examples:
#   ./scripts/new-migration.sh pipeline "add user preferences"
#   → backend/pipeline_migrations/20260420153045_add_user_preferences.sql
#
#   ./scripts/new-migration.sh main "create feedback table"
#   → backend/migrations/20260420153046_create_feedback_table.sql

set -euo pipefail

if [[ $# -lt 2 ]]; then
    echo "Usage: $0 <pipeline|main> <description>"
    echo ""
    echo "Examples:"
    echo "  $0 pipeline \"add user preferences\""
    echo "  $0 main \"create feedback table\""
    exit 1
fi

TARGET="$1"
shift
DESCRIPTION="$*"

case "$TARGET" in
    pipeline)
        DIR="backend/pipeline_migrations"
        ;;
    main)
        DIR="backend/migrations"
        ;;
    *)
        echo "ERROR: First argument must be 'pipeline' or 'main', got '${TARGET}'"
        exit 1
        ;;
esac

if [[ ! -d "$DIR" ]]; then
    echo "ERROR: Migration directory '$DIR' does not exist."
    echo "Are you running this from the repo root?"
    exit 1
fi

# Generate timestamp-based version (YYYYMMDDHHMMSS)
VERSION=$(date +"%Y%m%d%H%M%S")

# Convert description to snake_case filename
SLUG=$(echo "$DESCRIPTION" | tr '[:upper:]' '[:lower:]' | tr ' ' '_' | tr -cd 'a-z0-9_')

if [[ -z "$SLUG" ]]; then
    echo "ERROR: Description cannot be empty."
    exit 1
fi

FILENAME="${VERSION}_${SLUG}.sql"
FILEPATH="${DIR}/${FILENAME}"

# Check for collision (extremely unlikely with second-level timestamps)
if [[ -f "$FILEPATH" ]]; then
    echo "ERROR: File already exists: $FILEPATH"
    echo "Wait one second and try again."
    exit 1
fi

# Create the file with a header comment
cat > "$FILEPATH" << EOF
-- ${SLUG}: $(echo "$DESCRIPTION" | sed 's/^./\U&/')
--
-- Created: $(date +"%Y-%m-%d %H:%M:%S")
-- Target: ${TARGET} database
--
-- TODO: Add your SQL here.

EOF

echo "Created: $FILEPATH"
echo ""
echo "Next steps:"
echo "  1. Edit the file and add your SQL"
echo "  2. Run ./scripts/check-migrations.sh to validate"
echo "  3. Test locally before deploying"
