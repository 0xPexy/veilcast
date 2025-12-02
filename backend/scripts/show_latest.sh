#!/usr/bin/env bash
set -euo pipefail

# Show latest 10 records per key table in the Postgres container.
# Usage: ./backend/scripts/show_latest.sh

DB_CONTAINER=${DB_CONTAINER:-veilcast-db}
DB_NAME=${DB_NAME:-veilcast}
DB_USER=${DB_USER:-veilcast}

psql_cmd() {
  docker exec -i "$DB_CONTAINER" psql -U "$DB_USER" -d "$DB_NAME" -c "$1"
}

echo "== polls (latest 10) =="
psql_cmd "SELECT id, question, category, commit_phase_end, reveal_phase_end, commit_sync_completed, created_at FROM polls ORDER BY created_at DESC LIMIT 10;"

echo "== poll_members (latest 10) =="
psql_cmd "SELECT poll_id, identity_secret, created_at FROM poll_members ORDER BY created_at DESC LIMIT 10;"

echo "== members (latest 10) =="
psql_cmd "SELECT id, identity_secret, created_at FROM members ORDER BY created_at DESC LIMIT 10;"

echo "== commitments (latest 10) =="
psql_cmd "SELECT id, poll_id, commitment, identity_secret, onchain_submitted, recorded_at FROM commitments ORDER BY recorded_at DESC LIMIT 10;"

echo "== votes (latest 10) =="
psql_cmd "SELECT poll_id, nullifier, recorded_at FROM votes ORDER BY recorded_at DESC LIMIT 10;"
