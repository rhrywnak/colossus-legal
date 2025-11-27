 #!/usr/bin/env bash
#
# start_task_branch.sh
#
# Usage:
#   ./start_task_branch.sh T2.3-claims-integration-l1
#
# This script:
#   1. Ensures you're on main
#   2. Pulls the latest changes
#   3. Creates a new task branch (feature/<TASK>)
#   4. Shows branch status + reminder about Codex workflow
#

set -e

TASK_ID="$1"

if [ -z "$TASK_ID" ]; then
  echo "❌  ERROR: No task ID provided."
  echo "Usage: $0 <task-id>"
  echo "Example: $0 T2.3-claims-integration-l1"
  exit 1
fi

BRANCH="feature/${TASK_ID}"

echo "🔍 Checking current branch..."
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)

if [ "$CURRENT_BRANCH" != "main" ]; then
  echo "➡️  Switching to main..."
  git switch main
fi

echo "⬇️  Pulling latest changes..."
git pull

echo "🌱 Creating new task branch: $BRANCH"
git switch -c "$BRANCH"

echo ""
echo "🎉 Branch created: $BRANCH"
echo ""
echo "Next steps:"
echo "  1. Open Codex CLI"
echo "  2. Start a new session"
echo "  3. Paste the T2.x planning prompt for this task:"
echo "        Task ID: $TASK_ID"
echo "        Branch:  $BRANCH"
echo ""
echo "Codex workflow reminder:"
echo "  ✔ 1 Task → 1 Branch"
echo "  ✔ Never work on main"
echo "  ✔ Codex must confirm Task ID / Persona / Layer"
echo "  ✔ Codex must read required files BEFORE editing"
echo ""
echo "You're ready to start Codex for task: $TASK_ID"

