#!/bin/bash
# bump-version.sh — Update version in both frontend and backend
# Usage: ./bump-version.sh 0.7.3
#   (do NOT include the 'v' prefix — just the semver number)

set -euo pipefail
cd "$(git rev-parse --show-toplevel)" || exit 1

if [ $# -ne 1 ]; then
  echo "Usage: $0 <version>"
  echo "  Example: $0 0.7.3"
  exit 1
fi

VERSION="$1"

# Strip leading 'v' if accidentally provided
VERSION="${VERSION#v}"

# Validate semver format (basic check)
if ! echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "ERROR: Version must be semver format (e.g., 0.7.3)"
  exit 1
fi

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
FRONTEND_PKG="$REPO_ROOT/frontend/package.json"
BACKEND_CARGO="$REPO_ROOT/backend/Cargo.toml"

# --- Frontend (package.json) ---
if [ -f "$FRONTEND_PKG" ]; then
  OLD_FE=$(grep -oP '"version":\s*"\K[^"]+' "$FRONTEND_PKG")
  sed -i "s/\"version\": \"$OLD_FE\"/\"version\": \"$VERSION\"/" "$FRONTEND_PKG"
  echo "Frontend: $OLD_FE → $VERSION  ($FRONTEND_PKG)"
else
  echo "WARNING: $FRONTEND_PKG not found"
fi

# --- Backend (Cargo.toml) ---
if [ -f "$BACKEND_CARGO" ]; then
  OLD_BE=$(grep -oP '^version\s*=\s*"\K[^"]+' "$BACKEND_CARGO")
  sed -i "s/^version = \"$OLD_BE\"/version = \"$VERSION\"/" "$BACKEND_CARGO"
  echo "Backend:  $OLD_BE → $VERSION  ($BACKEND_CARGO)"
else
  echo "WARNING: $BACKEND_CARGO not found"
fi

echo ""
echo "Done. Run:"
echo "  git add frontend/package.json backend/Cargo.toml"
echo "  git commit -m \"chore: bump version to v$VERSION\""
echo "  git push origin \$(git branch --show-current)"
