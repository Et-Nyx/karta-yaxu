#!/bin/sh
set -e

echo "[entrypoint] Ensuring data directory exists..."
mkdir -p /app/data

echo "[entrypoint] Applying database schema (drizzle-kit push --force)..."
npx drizzle-kit push --force

echo "🎮 Starting Karta Yaxu 2.0..."
exec node build
