# Karta Yaxu

A creature collector design wiki and future Battle Lab, built with Rust and
server-rendered HTML. The current milestone is a local, read-only creature
browser plus an invite-only authentication foundation.

## Run locally

Requires Nix with flakes enabled on Linux. From the project root:

```bash
nix develop
cargo run -p web-app --bin admin -- migrate
KARTA_DEV_AUTH=1 cargo run -p web-app --bin admin -- invite-user --subject alice --role administrator
KARTA_DEV_AUTH=1 cargo run -p web-app
```

Then open **http://127.0.0.1:3000** and sign in at `/login` as `alice`.
Stop with Ctrl+C. SIGINT and SIGTERM trigger graceful shutdown. The first build
needs network access for pinned packages and crates; normal use and tests do not
fetch external content.

## Authentication (development placeholder)

Authentication is a **development placeholder and is not secure**. It uses no
passwords: anyone who can reach the server can sign in as any invited user. It is
disabled unless `KARTA_DEV_AUTH=1` is set, and it must not be exposed beyond local
development. Accounts are invite-only: an operator invites a subject before first
sign-in, and the account is created on that first sign-in.

The code keeps authentication behind an application-layer identity-provider port.
A future Polycentric identity provider implements this port and adds its own
credential-specific browser sign-in flow. Preserving existing local user ids
requires an explicit identity-mapping procedure; the placeholder adapter test
does not implement that migration. See decisions
`0003-authentication-foundation.md` and `0004-authentication-refinement.md` in
`docs/decisions/`.

Accounts and sessions are persistent. Disabling an account permanently revokes
all its sessions in the same transaction; re-enabling restores none of them.
Sign-in must be repeated. Invitations only authorize first sign-in: re-inviting
an existing account is rejected and does not change its role.

Authentication forms have a **4 KiB** body limit. Login POSTs share **30 attempts
per 60 seconds** across the whole server process, including rejected attempts.
Exhausting the allowance returns `429` with `Retry-After`; wait up to one minute
before retrying. Account pages and logout remain available. The allowance resets
on restart and is deliberately process-wide for the local preview.

Database failures return temporary-unavailability errors rather than pretending
the user signed out. If logout cannot revoke the session, it preserves cookies
and offers a retry button; it does not claim logout succeeded.

### Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `KARTA_BIND` | `127.0.0.1:3000` | Listen address (IPv4 loopback by default). |
| `KARTA_DATABASE_URL` | `sqlite://data/karta.db` | SQLite path; only the explicit `migrate` command creates it. |
| `KARTA_PUBLIC_ORIGIN` | `http://127.0.0.1:3000` | Expected `Origin` for form POSTs. |
| `KARTA_COOKIE_SECURE` | off | Set `1` to add `Secure` to cookies (needed under HTTPS). |
| `KARTA_SESSION_TTL_SECONDS` | `604800` | Session lifetime: 1–2,592,000 seconds (30 days maximum). |
| `KARTA_DEV_AUTH` | off | Set `1` to enable the placeholder provider. |

Keep the database file out of tracked source (`/data/` and `*.db*` are ignored).
Cookies are `HttpOnly` and `SameSite=Strict`; `Secure` is off only for local HTTP.

## Available workflows

- `/`: project overview.
- `/creatures`: catalog and case-insensitive name search.
- `/creatures/{id}`: creature details addressed by stable canonical ID.
- `/login`, `/logout`, `/account`: placeholder sign-in and the signed-in account.
- `/healthz`: process health.
- `cargo run -p tools -- validate-fixtures [path.json]`: validate bundled or
  custom fixtures; errors exit nonzero. Custom files are checked, not imported.
- `cargo run -p web-app --bin admin -- <command>`: operator CLI:
  - `migrate` — apply pending migrations (deliberate; never automatic).
  - `invite-user --subject <name> [--role <viewer|editor|administrator>] [--provider <id>] [--note <text>]` (default role: viewer).
  - `list-users`, `list-invites`.
  - `disable-user --id <immutable-user-id>`, `enable-user --id <immutable-user-id>`.
  - `--help` or `<command> --help`: usage without configuration/database access.

For example, copy the account id printed by `list-users`:

```bash
cargo run -p web-app --bin admin -- list-users
cargo run -p web-app --bin admin -- disable-user --id <immutable-user-id>
cargo run -p web-app --bin admin -- enable-user --id <immutable-user-id>
```

The CLI is trusted local operator tooling: access to the executable and database
authorizes these operations, independently of browser roles. No admin HTTP routes
are exposed. Invalid commands are rejected before opening the database.

The server and non-migration commands require an existing, compatible database.
Startup checks migration versions, success/checksums, and required schema columns
without applying migrations. Missing migrations give the explicit migration
command; incompatible history requires investigation instead of automatic repair.
These refinements need no new migration: existing auth accounts, invites and
sessions remain compatible. Back up an existing database using SQLite's backup
facilities before deliberately applying future migrations.

Fixtures, templates, and CSS are compiled into the executable; rebuild after
editing them. The running app does not require a particular working directory.

## Checks

Inside `nix develop`:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Tests use temporary databases and cover fixture/browser behavior, malformed CLI
commands without side effects, schema compatibility, concurrent invite claims,
transaction rollback, disable/re-enable and session-creation races, persistence
across database reopening, expiry, cookie attributes, Origin/CSRF validation,
form limits, login throttling, and retryable storage/logout failures. A second
provider exercises the port using the existing development credential flow.

Manual keyboard check: Tab from the address bar to the skip link, then to Account;
sign in using the labelled username field and Enter, reach Sign out using Tab,
and activate it with Enter. Verify focus/layout at a narrow viewport and 200%
zoom. Also check that a sign-in error retains the username and that the logout
failure page's retry button is keyboard-accessible.

## Structure and dependency purposes

- `crates/game-data`: typed fixture records and catalog validation.
- `crates/web-app`: catalog use cases, the identity-provider port, accounts and
  sessions, SQLx persistence, Axum routing, Askama HTML, custom CSS.
  - `src/auth/`: the provider port and the placeholder adapter.
  - `src/accounts.rs`: invite-only membership and opaque sessions.
  - `src/persistence.rs` + `migrations/`: SQLite queries and versioned migrations.
  - `src/bin/admin.rs`: migration and invite operator CLI.
- `crates/tools`: fixture validator.
- `src/web/login_limit.rs`: bounded process-local login request budget.
- SQLx provides SQLite access and migrations; SHA-256 hashes session tokens;
  `getrandom` supplies tokens; Serde decodes fixtures. The POC that used a
  self-hosted Polycentric server lives outside this repository (see below).

## Next milestones and boundaries

Editing creatures with attributed revision history and optimistic conflicts is the
next milestone, built on this account and session foundation. PokeAPI imports,
engine export, backups, and combat follow. The fixture format does not settle
creature stats or the export schema.

The independent Polycentric proof of concept was extracted from this repository to
`/home/eter/experiments/polycentric` and its local server stack was removed. It is
deferred, not adopted; decisions 0002 and 0003 record the status and the seam.

See `PROJECT_GUIDELINES.md` for the full project reference and `AGENTS.md` for
contributor instructions.
