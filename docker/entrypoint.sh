#!/bin/sh
set -e

echo "[entrypoint] Ensuring data directory exists..."
mkdir -p /app/data

if [ -f /app/data/rom.db ]; then
  ts=$(date +%Y%m%d-%H%M%S)
  echo "[entrypoint] Snapshotting rom.db -> rom.db.bak.$ts"
  cp /app/data/rom.db "/app/data/rom.db.bak.$ts"
  # Keep only the 5 most recent snapshots
  ls -1t /app/data/rom.db.bak.* 2>/dev/null | tail -n +6 | xargs -r rm
fi

echo "[entrypoint] Applying pending migrations (drizzle-kit migrate)..."
npx drizzle-kit migrate

echo "🎮 Starting Karta Yaxu 2.0..."
exec node build
