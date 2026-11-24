#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

PROJECT="rusty-dev"

COMPOSE=(
  docker compose
  -p "$PROJECT"
  --env-file .env.dev
  -f compose.yaml
  -f compose.dev.yaml
)

echo "Starting development database..."
"${COMPOSE[@]}" up -d postgres

echo "Preparing SQLx..."
"${COMPOSE[@]}" run --rm sqlx-tools bash -c '
  set -e

  cargo install sqlx-cli \
    --version 0.9.0 \
    --no-default-features \
    --features postgres,rustls

  sqlx --version

  echo "Resetting development database..."
  sqlx database reset --source ./migrations -y

  echo "Generating SQLx offline metadata..."
  SQLX_OFFLINE=false cargo sqlx prepare
'

echo "SQLx metadata updated successfully!"