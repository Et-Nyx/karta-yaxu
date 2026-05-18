# ═══════════════════════════════════════════════
# Stage 1: Build
# ═══════════════════════════════════════════════
FROM node:20-alpine AS build

# better-sqlite3 requires native compilation tools for Alpine (musl)
RUN apk add --no-cache python3 make g++

WORKDIR /app

COPY package.json package-lock.json ./
RUN npm ci
# Rebuild native modules for Alpine's musl libc
RUN npm rebuild better-sqlite3

COPY . .
RUN npm run build

# NOTE: We do NOT prune devDeps because drizzle-kit is needed
# at runtime for schema initialization (db:push on first boot).

# ═══════════════════════════════════════════════
# Stage 2: Runtime
# ═══════════════════════════════════════════════
FROM node:20-alpine AS runtime

# python3 + make + g++ needed for better-sqlite3 if rebuild needed
# curl needed for healthcheck
RUN apk add --no-cache curl

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

HEALTHCHECK --interval=30s --timeout=5s CMD curl -f http://localhost:3000/ || exit 1

VOLUME ["/app/data"]

ENTRYPOINT ["./entrypoint.sh"]
