#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

PROJECT="rusty-dev"
ENV_FILE=".env.dev"
RUN_DIR="$ROOT/.run"
SERVER_PID_FILE="$RUN_DIR/dev-server.pid"
WATCH_COMMAND='npm run test:js && cargo test && cargo run'

COMPOSE=(
  docker compose
  -p "$PROJECT"
  --env-file "$ENV_FILE"
  -f compose.yaml
  -f compose.dev.yaml
)

SERVER_PID=""

load_environment() {
  echo "Loading environment from $ENV_FILE..."

  set -a
  # shellcheck source=.env.dev
  source "$ENV_FILE"
  set +a
}

check_dependencies() {
  local missing=()

  for command in docker cargo cargo-watch npm setsid; do
    if ! command -v "$command" >/dev/null 2>&1; then missing+=("$command"); fi
  done

  if ((${#missing[@]} > 0)); then
    printf 'Missing required commands: %s\n' "${missing[*]}" >&2
    return 1
  fi
}

check_http_port() {
  if [[ -z "${APP_HOST:-}" || ! "${APP_PORT:-}" =~ ^[0-9]+$ ]] || ! command -v ss >/dev/null 2>&1; then return 0; fi

  local listeners
  listeners="$(ss -H -ltn "sport = :$APP_PORT" 2>/dev/null || true)"

  if [[ -n "$listeners" ]]; then
    printf 'HTTP bind unavailable: %s:%s already has a TCP listener.\n' "$APP_HOST" "$APP_PORT" >&2
    printf 'Find the process with: sudo lsof -i :%s\n' "$APP_PORT" >&2
    return 1
  fi
}

is_managed_server() {
  local pid="$1"
  local pgid sid cwd

  [[ "$pid" =~ ^[0-9]+$ && -d "/proc/$pid" ]] || return 1

  read -r pgid sid < <(ps -o pgid=,sid= -p "$pid" 2>/dev/null) || return 1
  [[ "$pgid" == "$pid" && "$sid" == "$pid" ]] || return 1

  cwd="$(readlink "/proc/$pid/cwd" 2>/dev/null)" || return 1
  [[ "$cwd" == "$ROOT" ]] || return 1

  tr '\0' '\n' < "/proc/$pid/environ" 2>/dev/null | grep -qx 'RUSTY_DEV_MANAGED=1'
}

stop_server() {
  local pid="${1:-$SERVER_PID}"

  if [[ -z "$pid" ]]; then return 0; fi

  if ! is_managed_server "$pid"; then
    if [[ -f "$SERVER_PID_FILE" ]]; then rm -f "$SERVER_PID_FILE"; fi
    return 0
  fi

  echo "Stopping Rust development server (PID $pid)..."

  kill -TERM -- "-$pid" 2>/dev/null || true

  for _ in {1..50}; do
    if ! kill -0 -- "-$pid" 2>/dev/null; then break; fi
    sleep 0.1
  done

  if kill -0 -- "-$pid" 2>/dev/null; then
    echo "Server did not stop gracefully. Forcing shutdown..."
    kill -KILL -- "-$pid" 2>/dev/null || true
  fi

  if [[ -f "$SERVER_PID_FILE" ]]; then
    rm -f "$SERVER_PID_FILE"
  fi

  SERVER_PID=""
}

start_server() {
  mkdir -p "$RUN_DIR"

  if [[ -f "$SERVER_PID_FILE" ]]; then
    local existing_pid
    existing_pid="$(cat "$SERVER_PID_FILE")"

    if is_managed_server "$existing_pid"; then
      echo "Development server is already running (PID $existing_pid)." >&2
      return 1
    fi

    rm -f "$SERVER_PID_FILE"
  fi

  echo "Starting development watcher..."
  echo "JavaScript tests -> Rust tests -> Rust server"
  echo

  setsid env RUSTY_DEV_MANAGED=1 bash -c 'cd "$1"; exec cargo watch -w static/js -w package.json -s "$2"' _ "$ROOT" "$WATCH_COMMAND" &

  SERVER_PID=$!
  echo "$SERVER_PID" > "$SERVER_PID_FILE"

  echo "Development watcher running (PID $SERVER_PID)."
  echo "Press Ctrl+C to stop the server."

  wait "$SERVER_PID"
}

stop_existing_server() {
  if [[ ! -f "$SERVER_PID_FILE" ]]; then return 0; fi

  local pid
  pid="$(cat "$SERVER_PID_FILE")"

  if is_managed_server "$pid"; then
    stop_server "$pid"
  else
    echo "Removing stale development server PID file."
    rm -f "$SERVER_PID_FILE"
  fi
}

cleanup() {
  stop_server
}

case "${1:-up}" in
  up)
    load_environment
    check_dependencies
    stop_existing_server
    check_http_port

    "${COMPOSE[@]}" up -d postgres redis storage

    echo
    echo "Development infrastructure is running."
    echo

    trap cleanup EXIT
    trap 'exit 130' INT
    trap 'exit 143' TERM
    trap 'exit 129' HUP

    start_server
    ;;

  down)
    stop_existing_server
    "${COMPOSE[@]}" down
    echo "Development environment stopped."
    ;;

  restart)
    "${COMPOSE[@]}" restart postgres redis storage
    ;;

  logs)
    "${COMPOSE[@]}" logs -f
    ;;

  status)
    "${COMPOSE[@]}" ps

    if [[ -f "$SERVER_PID_FILE" ]] && is_managed_server "$(cat "$SERVER_PID_FILE")"; then
      echo "Rust development server: running (PID $(cat "$SERVER_PID_FILE"))"
    else
      echo "Rust development server: stopped"
    fi
    ;;

  test)
    check_dependencies
    npm run test:js
    ;;

  *)
    echo "Usage: ./scripts/dev.sh {up|down|restart|logs|status|test}"
    exit 1
    ;;
esac