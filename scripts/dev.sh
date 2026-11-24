#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

PROJECT="rusty-dev"
ENV_FILE=".env.dev"

COMPOSE=(
  docker compose
  -p "$PROJECT"
  --env-file "$ENV_FILE"
  -f compose.yaml
  -f compose.dev.yaml
)

SERVER_PID=""

start_server() {
  echo "Loading environment from $ENV_FILE..."

  set -a
  # shellcheck source=.env.dev
  source "$ENV_FILE"
  set +a

  echo "Starting Rust server with cargo watch..."

  cargo watch -x run &
  SERVER_PID=$!

  echo "Rust server started. Press Ctrl+C to stop it."
  wait "$SERVER_PID"
}

stop_server() {
  if [[ -n "$SERVER_PID" ]]; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
}

case "${1:-up}" in
  up)
    "${COMPOSE[@]}" up -d postgres redis storage

    echo
    echo "Development infrastructure is running."
    echo "Starting Rust application..."
    echo

    trap stop_server EXIT INT TERM
    start_server
    ;;

  down)
    "${COMPOSE[@]}" down
    ;;

  restart)
    "${COMPOSE[@]}" restart postgres redis storage
    ;;

  logs)
    "${COMPOSE[@]}" logs -f
    ;;

  status)
    "${COMPOSE[@]}" ps
    ;;

  *)
    echo "Usage: ./scripts/dev.sh {up|down|restart|logs|status}"
    exit 1
    ;;
esac