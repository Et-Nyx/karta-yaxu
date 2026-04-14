#!/bin/sh
set -e

echo "[entrypoint] Initializing static assets volume..."

mkdir -p /app/static

if [ -z "$(ls -A /app/static 2>/dev/null)" ]; then
    echo "[entrypoint] Static volume is empty — copying defaults from image..."
    cp -r /app/static-reference/* /app/static/
else
    echo "[entrypoint] Static volume has content — skipping copy."
fi

echo "[entrypoint] Ensuring data directory exists..."
mkdir -p /app/data

echo "[entrypoint] Executing: $*"
exec "$@"