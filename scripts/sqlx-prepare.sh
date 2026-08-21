#!/usr/bin/env bash
set -euo pipefail
docker compose up -d postgres
docker compose run --rm sqlx-tools bash -c \
  "cargo install sqlx-cli --no-default-features --features postgres,rustls --quiet 2>/dev/null || true; \
   sqlx migrate run --source ./migrations --database-url postgres://admin:password@postgres:5432/database; \
   DATABASE_URL=postgres://admin:password@postgres:5432/database cargo sqlx prepare"