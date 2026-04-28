# Contributing to Karta Yaxu

Thank you for your interest in contributing to Karta Yaxu! This guide will help you get started.

## Prerequisites

You need **one** of the following setups:

### Option A: Nix (recommended)
- [Nix package manager](https://nixos.org/download.html) with flakes enabled
- Everything else is handled by `nix develop`

### Option B: Manual setup
- Rust stable toolchain with `wasm32-unknown-unknown` target
- `wasm-pack` for building the frontend WASM module
- `cargo-watch` for live reload during development
- `sqlx-cli` for database migrations
- `pkg-config` + `openssl` development headers
- Python 3.10+ (for data fetching scripts)

## Getting Started

```bash
# 1. Clone the repository
git clone <repo-url> && cd karta-yaxu

# 2. Enter the development environment
nix develop          # Option A
# or manually install dependencies listed above

# 3. Seed the database (creates data/rom.db if missing)
make seed-db

# 4. Build WASM + start the dev server
make dev

# 5. Open http://localhost:3000
```

## Project Structure

```
karta-yaxu/
├── crates/
│   ├── engine/      # Shared models + battle logic (compiles to WASM and native)
│   ├── frontend/    # WASM module for client-side interactivity
│   └── backend/     # Axum web server, templates, auth, editor
├── data/template/   # TOML seed files (source of truth for static data)
├── static/          # CSS, sprites, icons, WASM output
├── scripts/         # Python scripts to fetch data from PokeAPI
└── Makefile         # Common development commands
```

## Development Workflow

### Common Commands

| Command | Description |
|---------|-------------|
| `make dev` | Build WASM + start backend with live reload |
| `make build` | Production build (WASM + backend binary) |
| `make seed-db` | Populate the database from TOML templates |
| `make fmt` | Format all Rust code |
| `make lint` | Run clippy with warnings as errors |
| `make test` | Run all tests |
| `make check-all` | Format + lint + test (run before pushing) |

### Code Style

- **Always run `make check-all` before pushing.** CI will reject unformatted code or clippy warnings.
- Formatting is enforced by `rustfmt.toml` — just run `make fmt`.
- Clippy is configured at the workspace level in `Cargo.toml`.

### SQLx Offline Mode

The project uses `SQLX_OFFLINE=true` so builds don't require a live database. If you change any SQL queries or add migrations, regenerate the cache:

```bash
make sqlx-prepare
```

This updates the `.sqlx/` directory which must be committed.

## Adding New Data

To add new Pokémon, moves, abilities, or other entities:

1. Edit the corresponding TOML file in `data/template/` (e.g., `pokemon.toml`, `move.toml`)
2. Run `make seed-db` to apply changes to the database
3. Verify the data appears correctly on the wiki pages

For bulk data from PokeAPI, use the scripts in order:
```bash
python3 scripts/fetch_types.py
python3 scripts/fetch_pokeapi.py
python3 scripts/fetch_extras.py
make seed-db
```

## Adding a New Entity Route

The wiki and editor follow a consistent pattern. To add a new entity (e.g., "Items"):

1. **Model** — Add the struct to `crates/engine/src/models.rs`
2. **Migration** — Create a new SQL migration in `crates/backend/migrations/`
3. **Seed** — Add the TOML parser + insert logic to `crates/backend/src/seed.rs`
4. **Wiki handler** — Add the list/detail handlers in `crates/backend/src/wiki/`
5. **Editor handler** — Add CRUD handlers in `crates/backend/src/editor/`
6. **Templates** — Create Askama templates in `crates/backend/templates/wiki/` and `templates/editor/`
7. **Routes** — Register routes in the module's `routes()` function
8. **SQLx cache** — Run `make sqlx-prepare` and commit `.sqlx/` changes

## Environment Variables

See `.env.example` for all required variables. Key ones:

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | `sqlite:./data/rom.db` | SQLite database path |
| `SEED_DATA_PATH` | `data/template` | Path to TOML seed files |
| `RUST_LOG` | `debug` | Log level |
| `PORT` | `3000` | Server port |
| `SECURE_COOKIE` | `false` | Set `true` in production (HTTPS) |

## Branch Workflow

1. Create a feature branch from `master`: `git checkout -b feature/your-feature`
2. Make your changes, run `make check-all`
3. Push and open a merge request
4. CI must pass before merging
