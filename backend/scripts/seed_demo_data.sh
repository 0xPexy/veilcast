#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
INFRA_DIR="$(cd "$PROJECT_ROOT/../infra" && pwd)"
SQL_FILE="$SCRIPT_DIR/demo_seed.sql"

DEFAULT_URL="postgres://veilcast:veilcast@localhost:5432/veilcast"
DATABASE_URL="${DATABASE_URL:-$DEFAULT_URL}"
DB_USER="${DB_USER:-veilcast}"
DB_NAME="${DB_NAME:-veilcast}"

echo "[demo-seed] Preparing demo dataset..."

if command -v psql >/dev/null 2>&1; then
  echo "[demo-seed] Using host psql (DATABASE_URL=${DATABASE_URL})"
  PGPASSWORD="${PGPASSWORD:-}" psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f "$SQL_FILE"
else
  if ! command -v docker >/dev/null 2>&1; then
    echo "[demo-seed] Neither psql nor docker is available. Install PostgreSQL client tools or ensure Docker is present." >&2
    exit 1
  fi
  echo "[demo-seed] psql not found locally. Using docker compose exec (user=${DB_USER}, db=${DB_NAME})."
  (
    cd "$INFRA_DIR"
    docker compose up -d db >/dev/null
    cat "$SQL_FILE" | docker compose exec -T db psql -U "$DB_USER" -d "$DB_NAME" -v ON_ERROR_STOP=1 -f -
  )
fi

echo "[demo-seed] Demo dataset loaded."
