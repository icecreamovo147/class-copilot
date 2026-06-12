#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DB_PATH="${1:-$HOME/Library/Application Support/com.class-copilot/class_management.db}"
SQL_PATH="${2:-$ROOT_DIR/scripts/test_data.sql}"

if [[ ! -f "$SQL_PATH" ]]; then
  echo "SQL file not found: $SQL_PATH" >&2
  exit 1
fi

mkdir -p "$(dirname "$DB_PATH")"
rm -f "$DB_PATH" "$DB_PATH-shm" "$DB_PATH-wal"

cargo run --manifest-path "$ROOT_DIR/src-tauri/Cargo.toml" --bin init_db_at_path -- "$DB_PATH"

attempt=1
while true; do
  if sqlite3 -bail "$DB_PATH" < "$SQL_PATH"; then
    break
  fi
  if [[ "$attempt" -ge 10 ]]; then
    echo "failed to seed database after $attempt attempts" >&2
    exit 1
  fi
  sleep 0.2
  attempt=$((attempt + 1))
done

echo "Reset test database:"
echo "  db : $DB_PATH"
echo "  sql: $SQL_PATH"
