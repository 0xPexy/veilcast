#!/usr/bin/env bash
set -euo pipefail

# Initialize Postgres schema for VeilCast using the bundled SQL file.
# Usage: ./backend/scripts/db_init.sh

DB_CONTAINER=${DB_CONTAINER:-veilcast-db}
DB_NAME=${DB_NAME:-veilcast}
DB_USER=${DB_USER:-veilcast}
# Default path inside the repo; will be mounted if you run from host
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SQL_FILE_HOST=${SQL_FILE_HOST:-$SCRIPT_DIR/db_init.sql}
# Path inside container (relative to repo mount)
SQL_FILE_CONTAINER=${SQL_FILE_CONTAINER:-/tmp/db_init.sql}

# Copy SQL into container (to /tmp)
docker cp "$SQL_FILE_HOST" "$DB_CONTAINER:$SQL_FILE_CONTAINER"

docker exec -i "$DB_CONTAINER" psql -U "$DB_USER" -d "$DB_NAME" -f "$SQL_FILE_CONTAINER"

echo "DB init completed."
