#!/bin/sh
set -e

echo "[entrypoint] Ensuring data directory exists..."
mkdir -p /app/data

echo "[entrypoint] Applying database schema (drizzle-kit push)..."
npx drizzle-kit push --force 2>&1 || echo "[entrypoint] Warning: schema push encountered issues"

echo "🎮 Starting Karta Yaxu 2.0..."
exec node build
