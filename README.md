# Karta Yaxu

A creature collector design wiki and future Battle Lab, built with Rust and
server-rendered HTML. This first milestone is a local, read-only creature browser.

## Run locally

Requires Nix with flakes enabled on Linux. From the project root:

```bash
nix develop
cargo run -p tools -- validate-fixtures
cargo run -p web-app
```

Open **http://127.0.0.1:3000**. Stop with Ctrl+C. The server binds to loopback;
port 3000 must be available. SIGINT and SIGTERM trigger graceful shutdown.
The first build needs network access to download pinned packages and crates.
Normal app use and tests do not fetch external content.

The Nix shell supplies Rust 1.98.1, Cargo, rustfmt, Clippy, SQLite CLI, and curl.
`flake.lock` pins Nixpkgs; `rust-toolchain.toml` records the matching Rust version
for rustup users. Update these together. `Cargo.lock` pins Rust dependencies.

## Available workflows

- `/`: project overview.
- `/creatures`: catalog and case-insensitive name search.
- `/creatures/{id}`: creature details addressed by stable canonical ID.
- `/healthz`: process health; startup validates the bundled catalog.
- `cargo run -p tools -- validate-fixtures [path.json]`: validate bundled or custom
  local fixtures; errors exit nonzero. Custom files are checked, not imported.

Fixtures, HTML templates, and CSS are compiled into the executable; rebuild after
editing them. The running app does not require a particular working directory.

## Checks

Inside `nix develop`:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Tests cover fixture invariants, stable IDs on rename, browser routes, detail
links, search, empty and missing results, rejected mutations, and HTML escaping.
For a manual accessibility check: use Tab from the address bar to the skip link,
navigate to the catalog, search and clear it using the keyboard, open a detail
link, and check focus/readability at a narrow viewport and 200% zoom.

## Structure and dependency purposes

- `crates/game-data`: typed fixture records and catalog validation;
  Serde/serde_json decode JSON.
- `crates/web-app`: catalog use cases, Axum routing, Tokio runtime/shutdown,
  Askama HTML, and custom CSS. Tower is used only for HTTP workflow tests.
- `crates/tools`: fixture validator using the same game-data library.
- `fixtures`: three invented development placeholders; no final game assets.
- `docs/decisions/0001-local-foundation.md`: scope, choices, and tradeoffs.

## Next milestones and boundaries

Accounts, SQLite persistence, attributed edits, revision conflicts, backups,
PokeAPI imports, engine export, and combat are future work. The fixture format
does not settle creature stats or the engine export schema. No migrations are
needed for this milestone. This unauthenticated local fixture preview is not a
team deployment; wiki visibility and authentication remain open decisions.

The full project reference is currently named `PROJECT_GUIDEINES.md` (existing
spelling). See `AGENTS.md` for contributor instructions.
