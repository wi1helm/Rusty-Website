#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

COMPOSE=(
  docker compose
  -p rusty-staging
  --env-file .env.staging
  -f compose.yaml
  -f compose.staging.yaml
)

case "${1:-}" in
  up)
    npm test
    "${COMPOSE[@]}" up -d --build
    echo "Staging started."
    ;;

  down)
    "${COMPOSE[@]}" down
    echo "Staging stopped."
    ;;

  rebuild)
    npm test
    "${COMPOSE[@]}" up -d --build --force-recreate
    echo "Staging rebuilt."
    ;;

  reset)
    npm test
    "${COMPOSE[@]}" down -v
    "${COMPOSE[@]}" up -d --build
    echo "Staging recreated with fresh data."
    ;;

  logs)
    "${COMPOSE[@]}" logs -f
    ;;

  *)
    echo "Usage: ./scripts/staging.sh {up|down|rebuild|reset|logs}"
    exit 1
    ;;
esac