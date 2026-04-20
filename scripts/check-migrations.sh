#!/usr/bin/env bash
# check-migrations.sh — Validate migration files have unique version prefixes.
#
# sqlx parses the numeric prefix before the first underscore as the
# migration version (i64). Two files with the same prefix cause a
# VersionMismatch panic at startup — the backend crashes in a restart
# loop and cannot serve traffic.
#
# This script catches duplicates before they reach production.
# It runs automatically as part of build-release.sh.
#
# Usage:
#   ./scripts/check-migrations.sh    # Check and report

set -euo pipefail

ERRORS=0

check_dir() {
    local dir="$1"
    local label="$2"

    if [[ ! -d "$dir" ]]; then
        return
    fi

    local count=0
    local prefixes=""

    for f in "$dir"/*.sql; do
        [[ -e "$f" ]] || continue
        local base
        base=$(basename "$f")
        local version
        version=$(echo "$base" | sed 's/_.*//')
        prefixes="${prefixes}${version}"$'\n'
        count=$((count + 1))
    done

    if [[ $count -eq 0 ]]; then
        echo "OK: $label — no migration files found"
        return
    fi

    local dupes
    dupes=$(echo "$prefixes" | sort | uniq -d | grep -v '^$' || true)

    if [[ -n "$dupes" ]]; then
        echo "ERROR: Duplicate migration versions in $label:"
        for v in $dupes; do
            echo "  Version $v:"
            ls "$dir"/${v}_*.sql 2>/dev/null | sed 's/^/    /'
        done
        ERRORS=$((ERRORS + 1))
    else
        echo "OK: $label — $count migrations, no duplicates"
    fi
}

echo "============================================"
echo " Migration Version Check"
echo "============================================"
echo ""

check_dir "backend/migrations" "Main DB (backend/migrations)"
check_dir "backend/pipeline_migrations" "Pipeline DB (backend/pipeline_migrations)"

echo ""
if [[ $ERRORS -gt 0 ]]; then
    echo "FAILED: $ERRORS directory(ies) have duplicate versions."
    echo ""
    echo "Fix: Use ./scripts/new-migration.sh to create migrations."
    echo "It generates timestamp-based filenames (YYYYMMDDHHMMSS) that"
    echo "are unique to the second. Never create migration files manually."
    echo ""
    echo "sqlx uses the numeric prefix as the version — two files with"
    echo "the same prefix cause VersionMismatch at startup."
    exit 1
else
    echo "All migration versions unique."
fi
