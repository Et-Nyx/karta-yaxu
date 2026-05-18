# ═══════════════════════════════════════════════
# Stage 1: Build
# ═══════════════════════════════════════════════
FROM node:20-bookworm-slim AS build

WORKDIR /app

COPY package.json package-lock.json ./
# We don't need to rebuild or install python/make anymore because
# Debian slim uses glibc, which is perfectly compatible with the 
# pre-built better-sqlite3 binaries downloaded by npm ci.
RUN npm ci

COPY . .
RUN npm run build

# NOTE: We do NOT prune devDeps because drizzle-kit is needed
# at runtime for schema initialization (db:push on first boot).

# ═══════════════════════════════════════════════
# Stage 2: Runtime
# ═══════════════════════════════════════════════
FROM node:20-bookworm-slim AS runtime

# curl needed for healthcheck
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=build /app/build ./build
COPY --from=build /app/node_modules ./node_modules
COPY --from=build /app/package.json ./
# drizzle-kit needs these for db:push at boot
COPY --from=build /app/drizzle.config.ts ./
COPY --from=build /app/src/lib/db ./src/lib/db
COPY docker/entrypoint.sh ./entrypoint.sh

RUN chmod +x ./entrypoint.sh
RUN mkdir -p ./data

ENV PORT=3000
ENV HOST=0.0.0.0
ENV NODE_ENV=production
ENV DATABASE_URL=./data/rom.db

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=10s --start-period=15s CMD curl -f http://127.0.0.1:3000/ || exit 1

VOLUME ["/app/data"]

ENTRYPOINT ["./entrypoint.sh"]
