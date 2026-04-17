#!/bin/sh
set -e

echo "[entrypoint] Syncing static assets from image to volume..."
mkdir -p /app/static
cp -rn /app/static-reference/* /app/static/
chown -R app:app /app/static

echo "[entrypoint] Ensuring data directory exists..."
mkdir -p /app/data
chown -R app:app /app/data

echo "[entrypoint] Executing: $*"
exec su app -c "$*"