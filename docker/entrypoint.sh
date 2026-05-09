#!/bin/sh
set -e

# Ensure data directory exists
mkdir -p /app/data

echo "🎮 Starting Karta Yaxu..."
exec node build
