#!/bin/bash
# bump-version.sh — Update version in both frontend and backend
# Usage: ./bump-version.sh 0.7.3
set -euo pipefail
cd "$(git rev-parse --show-toplevel)" || exit 1

if [ $# -ne 1 ]; then
  echo "Usage: $0 <version>"
  exit 1
fi

VERSION="${1#v}"

# Frontend: use jq to set version
jq --arg v "$VERSION" '.version = $v' frontend/package.json > frontend/package.json.tmp
mv frontend/package.json.tmp frontend/package.json

# Backend: use toml-aware sed (match the first version line only)
sed -i "0,/^version = \".*\"/s//version = \"$VERSION\"/" backend/Cargo.toml

# Verify
FE=$(jq -r .version frontend/package.json)
BE=$(grep -m1 '^version' backend/Cargo.toml | cut -d'"' -f2)

echo "Frontend: $FE"
echo "Backend:  $BE"

if [ "$FE" != "$VERSION" ] || [ "$BE" != "$VERSION" ]; then
  echo "ERROR: Version mismatch! Expected $VERSION"
  exit 1
fi

echo "Both set to $VERSION"
