#!/usr/bin/env bash
#
# push-templates.sh — sync extraction template(s) from the git repo to DEV.
#
# Automates the sacred deployment process of
# DOCUMENT_TYPE_ONBOARDING_RUNBOOK_v1 Step 8 (scp to /tmp, sudo cp into the
# root-owned mount, md5 verify both sides) as ONE command, so the prompt
# tuning loop is: edit in repo -> ./push-templates.sh -> click scan.
#
# The engine re-reads template files at every scan start — no restart,
# no rebuild, no deploy. The repo remains the single source of truth;
# this script only moves bytes.
#
# Usage:
#   ./push-templates.sh                       # pushes theme_scan_prompt_v3.md
#   ./push-templates.sh file1.md file2.md     # pushes named files
#
# Files are named relative to backend/extraction_templates/ in the repo.
#
# Created 2026-08-07 (task 2.15, scan-quality tuning loop).

set -euo pipefail

REPO_DIR="$HOME/Projects/colossus-legal/backend/extraction_templates"
DEV_HOST="core@10.10.100.220"
DEV_DIR="/mnt/data/legal-docs/extraction_templates"

# Default file: the scan prompt under tuning.
FILES=("$@")
if [ ${#FILES[@]} -eq 0 ]; then
  FILES=("theme_scan_prompt_v3.md")
fi

FAIL=0

for f in "${FILES[@]}"; do
  LOCAL="$REPO_DIR/$f"

  if [ ! -f "$LOCAL" ]; then
    echo "MISSING  $f — not found at $LOCAL"
    FAIL=1
    continue
  fi

  # Hop 1: repo -> DEV /tmp
  scp -q "$LOCAL" "$DEV_HOST:/tmp/$f"

  # Hop 2: /tmp -> root-owned mount, then clean up /tmp
  ssh "$DEV_HOST" "sudo cp /tmp/$f $DEV_DIR/$f && rm -f /tmp/$f"

  # Verify: md5 both sides (macOS md5 locally, GNU md5sum remotely)
  LOCAL_MD5=$(md5 -q "$LOCAL")
  REMOTE_MD5=$(ssh "$DEV_HOST" "md5sum $DEV_DIR/$f" | awk '{print $1}')

  if [ "$LOCAL_MD5" = "$REMOTE_MD5" ]; then
    echo "VERIFIED $f  ($LOCAL_MD5)"
  else
    echo "MISMATCH $f  local=$LOCAL_MD5 remote=$REMOTE_MD5"
    FAIL=1
  fi
done

exit $FAIL
