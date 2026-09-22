#!/usr/bin/env bash
#
# push-templates.sh — sync extraction template(s) from the git repo to DEV or PROD.
#
# Automates the sacred deployment process of
# DOCUMENT_TYPE_ONBOARDING_RUNBOOK_v1 Step 8 (scp to /tmp, sudo cp into the
# root-owned mount, chmod 644, md5 verify both sides) as ONE command, so the
# prompt tuning loop is: edit in repo -> ./push-templates.sh -> click scan.
#
# The engine re-reads template files at every scan start — no restart,
# no rebuild, no deploy. The repo remains the single source of truth;
# this script only moves bytes.
#
# Usage:
#   ./push-templates.sh                            # DEV: pushes theme_scan_prompt_v3.md
#   ./push-templates.sh file1.md file2.md          # DEV: pushes named files
#   ./push-templates.sh PROD file1.md file2.md     # PROD: pushes named files
#   ./push-templates.sh DEV file1.md               # DEV, said explicitly
#
# The target is DEV unless the FIRST argument is the word PROD (or DEV). PROD
# is never a default: pushing a prompt to the witness-facing server is a
# decision, so it has to be typed. Roman runs the PROD push himself.
#
# Hosts come from the environment when set, and default to the inventory's
# (colossus-ansible/inventory/hosts.yml: colossus-dev-app1, colossus-prod-app1):
#   PUSH_TEMPLATES_DEV_HOST   default core@10.10.100.220
#   PUSH_TEMPLATES_PROD_HOST  default core@10.10.100.120
#   PUSH_TEMPLATES_DIR        default /mnt/data/legal-docs/extraction_templates
#
# Every pushed file is set to mode 644 after the copy. `sudo cp` into a fresh
# path creates the file with the umask of root's shell, and the backend reads
# the mount as a non-root user — a prompt file it cannot read is a backend that
# refuses to boot (v2.2.0's DEV deploy, 2026-09-21).
#
# Files are named relative to backend/extraction_templates/ in the repo.
#
# Created 2026-08-07 (task 2.15, scan-quality tuning loop).
# PROD target and chmod 644: 2026-09-22 (CC_TASK_PRACTICE_FIXES_v2.2.1, GO §7 (b)).

set -euo pipefail

REPO_DIR="$HOME/Projects/colossus-legal/backend/extraction_templates"
DEV_HOST="${PUSH_TEMPLATES_DEV_HOST:-core@10.10.100.220}"
PROD_HOST="${PUSH_TEMPLATES_PROD_HOST:-core@10.10.100.120}"
REMOTE_DIR="${PUSH_TEMPLATES_DIR:-/mnt/data/legal-docs/extraction_templates}"

# The target: an explicit first argument, or DEV.
TARGET="DEV"
if [ $# -gt 0 ] && { [ "$1" = "PROD" ] || [ "$1" = "DEV" ]; }; then
  TARGET="$1"
  shift
fi
case "$TARGET" in
  PROD) HOST="$PROD_HOST" ;;
  DEV)  HOST="$DEV_HOST" ;;
esac
echo "TARGET   $TARGET ($HOST:$REMOTE_DIR)"

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

  # Hop 1: repo -> host /tmp
  scp -q "$LOCAL" "$HOST:/tmp/$f"

  # Hop 2: /tmp -> root-owned mount, readable by the backend (644), then clean up /tmp
  ssh "$HOST" "sudo cp /tmp/$f $REMOTE_DIR/$f && sudo chmod 644 $REMOTE_DIR/$f && rm -f /tmp/$f"

  # Verify: md5 both sides (macOS md5 locally, GNU md5sum remotely), and the mode
  LOCAL_MD5=$(md5 -q "$LOCAL")
  REMOTE_MD5=$(ssh "$HOST" "md5sum $REMOTE_DIR/$f" | awk '{print $1}')
  REMOTE_MODE=$(ssh "$HOST" "stat -c %a $REMOTE_DIR/$f")

  if [ "$LOCAL_MD5" = "$REMOTE_MD5" ] && [ "$REMOTE_MODE" = "644" ]; then
    echo "VERIFIED $f  ($LOCAL_MD5, mode $REMOTE_MODE)"
  else
    echo "MISMATCH $f  local=$LOCAL_MD5 remote=$REMOTE_MD5 mode=$REMOTE_MODE"
    FAIL=1
  fi
done

exit $FAIL
