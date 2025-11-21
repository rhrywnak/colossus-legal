#!/usr/bin/env bash
#
# Bootstrap script for the Colossus-Legal repo.
# Run this from the repo root:
#   ./scripts/bootstrap_colossus_legal.sh
#
# What it does:
#  - Creates backend/.env and frontend/.env from the .env.example files (if they don't exist)
#  - Runs cargo build in backend
#  - Runs npm install in frontend

set -e

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
echo "Repo root: ${ROOT_DIR}"

# Backend env
if [ -f "${ROOT_DIR}/backend/.env" ]; then
  echo "[backend] .env already exists, leaving it as-is."
else
  if [ -f "${ROOT_DIR}/backend/.env.example" ]; then
    echo "[backend] Creating .env from .env.example"
    cp "${ROOT_DIR}/backend/.env.example" "${ROOT_DIR}/backend/.env"
  else
    echo "[backend] WARNING: backend/.env.example not found."
  fi
fi

# Frontend env
if [ -f "${ROOT_DIR}/frontend/.env" ]; then
  echo "[frontend] .env already exists, leaving it as-is."
else
  if [ -f "${ROOT_DIR}/frontend/.env.example" ]; then
    echo "[frontend] Creating .env from .env.example"
    cp "${ROOT_DIR}/frontend/.env.example" "${ROOT_DIR}/frontend/.env"
  else
    echo "[frontend] WARNING: frontend/.env.example not found."
  fi
fi

# Build backend
if [ -d "${ROOT_DIR}/backend" ]; then
  echo "[backend] Running cargo build..."
  cd "${ROOT_DIR}/backend"
  cargo build
else
  echo "[backend] WARNING: backend directory not found."
fi

# Install frontend deps
if [ -d "${ROOT_DIR}/frontend" ]; then
  echo "[frontend] Running npm install..."
  cd "${ROOT_DIR}/frontend"
  npm install
else
  echo "[frontend] WARNING: frontend directory not found."
fi

echo "Bootstrap complete."
