#!/usr/bin/env bash
set -euo pipefail

# List tables in the Postgres container used by docker-compose.
# Usage: ./backend/scripts/list_tables.sh

DB_CONTAINER=${DB_CONTAINER:-veilcast-db}
DB_NAME=${DB_NAME:-veilcast}
DB_USER=${DB_USER:-veilcast}

docker exec -it "$DB_CONTAINER" psql -U "$DB_USER" -d "$DB_NAME" -c "\dt"
