# Karta Yaxu

A creature collector design wiki and future Battle Lab, built with Rust and
server-rendered HTML. The local app authors a versioned shared species/form library
and independently owned game worlds, with scoped collaboration, multiple local
variants, versioned Types and ordered typing, explicit source adoption, and
attributed history. Explicit fixture commands provide sample authoring worlds,
not an executable game.

## Run locally

Requires Nix with flakes enabled on Linux. From the project root:

```bash
nix develop
cargo run -p web-app --bin admin -- migrate
cargo run -p web-app --bin admin -- seed-fixtures
KARTA_DEV_AUTH=1 cargo run -p web-app --bin admin -- invite-user --subject alice --role administrator
KARTA_DEV_AUTH=1 cargo run -p web-app --bin admin -- invite-user --subject bob --role editor
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

## Authoring workflow

| Action | Access |
| --- | --- |
| Browse library, worlds and content history | Public local preview |
| Create a world | Editor or administrator; creator becomes its owner |
| Create/edit shared species, forms and Types | Administrator |
| Edit a world, its variants or local Types | Owner or collaborator with editor/administrator eligibility |
| Grant/revoke collaboration and inspect access history | World owner |

Administrator status does not bypass another world's ownership. Accounts remain
invite-only; collaborators are existing enabled accounts, not new identity invites.
Users can find their immutable user id on `/account` and share it with the owner.
Disabling an owner preserves ownership and history. Ownership transfer/recovery
is a future explicit operation; it is not silently performed by account disabling.

1. Sign in as `alice`, open **Creatures**, and edit/create shared species or forms.
   A new species automatically creates its default form. Every successful change
   adds an immutable definition version and audit event in the same transaction.
2. Sign in as `bob`, open **Worlds → Create world**. Add variants from a concrete
   shared form/version, or choose world-exclusive content. Multiple variants may
   use the same form/version; local slugs distinguish them within the world.
3. Edit a variant's effective name/description without changing the library or
   another variant. Each form version also pins its exact species version.
4. After an administrator edits a source form, follow **Review adoption of version
   N** from a variant. Compare old source, local values and new source; explicitly
   confirm the effective fields and new pin. Untouched variants/worlds stay pinned.
5. Follow history links for actor, UTC time, reason, readable field changes and
   complete before/after snapshots. Public world snapshots omit access details.
6. The owner can manage collaborators using their immutable account ids. Access
   history is private and has a separate revision counter from content.

### Types and ordered typing

Use **Types** to author shared definitions. World owners/collaborators add exact
shared Type versions to their world or create a world-exclusive Type. Both library
and local Type edits create immutable versions. Updates do not automatically
retarget world sources or form/variant assignments; adoption and reassignment are
explicit, separate actions.

Shared forms select ordered shared Type versions. World variants select ordered
**same-world** local Type versions. Follow **Edit typing**, choose the displayed
slot count *before editing* (the GET reloads saved values), then choose the typing
state and native slot dropdowns:

- **Unmodeled**: this definition does not describe its typing yet.
- **Defined** with no selections: intentionally typeless.
- **Defined** with selections: exact pins, in displayed order; blank slots are
  skipped and duplicate Type identities are rejected.

The 32-slot software work bound is not Pokémon's two-type rule. Metadata edits
preserve exact typing state and pins. Shared-form adoption shows old/new source
typing but retains local world typing; it never maps by name or picks arbitrary
matching local Types. Type descriptions are inert metadata: effectiveness charts,
damage and transformation execution are not implemented.

### World fixtures

The cached reference subset contains **Fire, Flying, Dragon**, one shared Charizard
species and its base/Mega X forms with ordered typing. These are explicitly marked
temporary Pokémon placeholders, not a complete mechanics dataset. Source caches
under `fixtures/pokeapi/` retain endpoint kind, external id/URL, UTC retrieval time
and transformation version. Internal ids are independent of external ids.

To load just the shared reference subset:

```bash
cargo run -p web-app --bin admin -- seed-reference-fixtures
```

To add the two synthetic worlds, first sign in once as the account that should own
them (an invite alone is not an account). Copy its immutable id from `/account` or
`admin list-users`. From another terminal, inside `nix develop`, using the same
`KARTA_DATABASE_URL` as the server:

```bash
cargo run -p web-app --bin admin -- list-users
cargo run -p web-app --bin admin -- seed-world-fixtures --owner-user-id <immutable-user-id>
```

This also initializes the reference prerequisites atomically if missing. It
creates no accounts and requires an existing enabled editor/administrator owner.
Open **Worlds** to explore:

| World | Examples |
| --- | --- |
| **ClassicLab** (`/worlds/classiclab`) | Charizard Fire/Flying and Mega X Fire/Dragon |
| **VariantLab** (`/worlds/variantlab`) | Independent same-source Charizard variants, a three-Type variant, and world-exclusive Mist/Mistling |

The complete batch contains 3 shared Types, 1 species, 2 forms, 2 worlds, 7 local
Types and 5 variants. Creation history is attributed to `bundled-fixture-loader`,
including owner-only initial access history. Reruns skip 20 matching objects and
preserve later edits, typing and collaborator changes. A different owner, collisions,
drift or partial baselines reject the whole batch; existing worlds are never
repaired or overwritten. These synthetic labs are not official Pokémon editions,
player saves, encounter legality, battle transformations or engine packages.

Forms preserve submitted values after validation/authorization/storage failures.
Stale edits return **409**, display the newer record, and require **Save reviewed
changes**; another competing save produces another conflict. No automatic merging
or silent overwrite occurs. Authoring bodies are capped at **32 KiB**; slug/name/
description limits are 80/100/2,000 characters, with optional 500-character reasons.
Descriptions support plain-text paragraphs with LF line endings. The server is
authoritative for Unicode character limits; HTML's code-unit limits allow valid
non-BMP characters rather than prematurely truncating them.

### Object names, slugs and addresses

Slugs are generated by the server from display names; there is no manual slug
field. Accented and non-Latin names are transliterated with pinned `deunicode`
1.6.2: for example, **Flabébé Forest → `flabebe-forest`**. Separators collapse to
hyphens, addresses are capped at 80 ASCII characters, and an untransliterable name
uses `object`. Collisions get `-2`, `-3`, etc. within the existing uniqueness scope.

Object pages use current readable addresses:

```text
/creatures/emberling
/creatures/emberling/forms/emberling
/worlds/forest-world
/worlds/forest-world/variants/emberling-2
```

Edit, history, adoption and access GET links follow the same coordinates. Forms
are scoped to species; variants are scoped to worlds, so names can be reused in
different scopes without ambiguity. If a generated/stored slug is `new` or looks
exactly like a 32-character lowercase hexadecimal ID, its URL segment is prefixed
with `~` (such as `/worlds/~new`) to distinguish it from creation pages/ID links.
The marker is URL syntax, not stored data.

Renaming an object regenerates its slug atomically with the normal attributed
revision. Description-only edits and pin-only adoption keep the existing slug.
Existing manually assigned slugs also remain until the display name changes.
Parent renames change nested child addresses without rewriting their versions.

Canonical IDs remain immutable and identify all relationships and history. Old
ID GET/HEAD links, including `/forms/{id}`, temporarily redirect **307** to the
current readable address with suffix/query preserved and no-store caching. These
are durable permalinks; old slug paths have no historical alias and can become
404 or identify a new object if that slug is reused.

Save actions intentionally use canonical-ID targets so an open draft cannot edit
a replacement object after a rename/reuse. Successful saves use **303**, then
land on the readable address. Validation/conflict drafts retain their ID POST
address and entered values. Legacy manually submitted slug fields are ignored.
No migration or existing-data backfill is needed. See
`docs/decisions/0007-readable-object-addresses.md`.

### Existing database upgrade and initialization

For an existing development database, stop the old server and back up with
SQLite's backup facility (the example uses the default path; choose an unused
backup filename). Migrate/initialize deliberately, then restart the new binary:

```bash
sqlite3 -cmd ".timeout 5000" data/karta.db ".backup 'data/karta-before-authoring.db'"
cargo run -p web-app --bin admin -- migrate
cargo run -p web-app --bin admin -- seed-fixtures
```

Migration `0002_world_authoring.sql` adds typed content/version/access tables;
`0003_types.sql` adds Type versions, exact ordered bindings, provenance and full
snapshot sidecars. New binaries require all three migrations; older binaries reject
the newer schema. Migrations do not initialize content or alter existing auth
records. The
explicit fixture command preserves existing canonical creature ids as species
ids, creates default-form ids once, and attributes initial history to the named
`bundled-fixture-loader` system actor. It creates no login account. Matching seed
baselines are skipped after edits; drift/collisions roll back the entire seed batch.
An empty uninitialized library is supported.

The Types upgrade preserves old JSON bytes and immutable versions. Old form,
variant and world revisions retain their v1 meaning, including unmodeled typing.
New affected revisions have v2 full snapshots, linked to existing v1 metadata
projections and the same audit event. Missing/inconsistent full snapshots fail
closed rather than silently falling back. History reads both versions, including
v1-before/v2-after transitions. Species, Type and private access snapshots use their
own v1 schemas. Do not edit old migrations or add retrospective typing to history.

## Available workflows

- `/`: project overview.
- `/creatures`: catalog and case-insensitive name search.
- `/creatures/{species-slug}`: shared species details.
- `/creatures/new`, `/creatures/{species-slug}/edit`: administrator authoring.
- `/creatures/{species-slug}/forms/new`, `/creatures/{species-slug}/forms/{form-slug}`:
  shared form creation/detail; form detail paths support `/edit`.
- `/worlds`, `/worlds/new`, `/worlds/{world-slug}`: world catalog and creation/detail.
- `/worlds/{world-slug}/variants/{variant-slug}`: local detail, `/edit` and `/adopt`.
- `/types`, `/types/new`, `/types/{type-slug}`: shared Type catalog, creation and detail.
- `/worlds/{world-slug}/types/new`, `/worlds/{world-slug}/types/{type-slug}`:
  world-local Types; detail supports `/edit`, `/adopt` and history.
- Shared form and world variant detail paths support `/typing` for ordered assignments.
- Content detail URLs have `/history` and `/history/{revision}` views; lists are
  bounded to 50 changes per page.
- `/worlds/{world-slug}/access`: owner-only collaboration and private access history.
- `/login`, `/logout`, `/account`: placeholder sign-in and the signed-in account.
- `/healthz`: process health.
- `cargo run -p tools -- validate-fixtures [path.json]`: validate bundled or
  custom fixtures; errors exit nonzero. Custom files are checked, not imported.
- `cargo run -p web-app --bin admin -- <command>`: operator CLI:
  - `migrate` — apply pending migrations (deliberate; never automatic).
  - `seed-fixtures` — initialize bundled species/forms without overwriting edits.
  - `seed-reference-fixtures` — load the cached Type/Charizard subset.
  - `seed-world-fixtures --owner-user-id <immutable-user-id>` — load the complete
    synthetic lab worlds and reference prerequisites with explicit ownership.
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
Auth accounts, invites and sessions remain compatible through the additive
authoring migration. Back up an existing database using SQLite's backup
facilities before deliberately applying future migrations.

Initialization fixtures, templates and CSS are compiled into the executable;
rebuild after editing them. Live content is read from SQLite. The default database
path is relative to the working directory; set an absolute `KARTA_DATABASE_URL`
when running the executable elsewhere.

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
Authoring tests additionally cover direct-use-case scope/role checks, exact version
references, immutable history, typed snapshot validation, rollback after late
revision/audit faults, initialization baselines, explicit adoption, no-ops and
concurrent permission revocation. Migration 1→2 preserves existing sessions.
Types tests cover populated 2→3 upgrades with byte-preserved history, exact pins,
same-world scope, ordered/empty/unmodeled typing, sealed snapshots/bindings, late
fault rollback, fixture provenance/ownership/idempotence, and SQLite integrity.
URL tests additionally cover scoped addresses, rename/reuse, reserved/ID-shaped
slugs, query-preserving redirects, HEAD behavior, unchanged mutation targets and
source-version display independent of current address coordinates. Automatic slug
allocation is checked under concurrent writes and preserves immutable IDs.

### Headless keyboard browser tests (WSL/Linux)

From the project root:

```bash
nix develop .#browser-tests --command python tests/browser/run.py
```

This separate shell supplies Python Playwright, its matching driver, Chromium,
and fonts from the existing `flake.lock`. It runs headlessly: WSLg, a GUI, Xvfb,
and a display server are not required. No pip/npm or `playwright install` step
is needed. First use may download Nix packages and Rust build dependencies;
browser requests during the tests are restricted to the local test server.

The runner builds current native web/admin binaries with `--locked`, then runs
36 keyboard-driven tests at desktop (1280px) and narrow (320px) widths. It
uses fresh test-owned databases, servers on available loopback ports, and browser
contexts, independent of your running app or `KARTA_*` configuration. Those
processes and databases are cleaned up after each test. Custom `CARGO_TARGET_DIR`
locations are supported through Cargo metadata.

Coverage includes skip-link focus, forward/reverse Tab navigation, sign-in and
sign-out using Enter, retained username and correction after rejection, and a
keyboard-reachable retry after a real logout failure. Authoring coverage includes
two-world scoping, multiple/exclusive variants, library versions and adoption,
collaboration grant/revoke, repeated edit/access conflicts, readable history and
snapshots, Unicode field limits, automatic names/addresses, rename/collision and
ID-bound drafts after slug reuse. Assertions check focused controls are in view
(textareas may scroll the caret rather than the entire box), have a non-transparent
outline, and that pages have no
horizontal overflow. Navigation and typing use only keyboard events; locators
inspect state rather than clicking/focusing/filling controls.
The Types workflows also exercise native ordered dropdowns, 32 displayed slots,
Type version/adoption independence, typing conflicts and revoked grants, and the
two owned fixture labs without external fetching.

Failures save screenshots in a unique ignored `target/browser-tests/run-*`
directory. Screenshots can be opened from Windows for diagnosis. No traces,
session tokens, cookie dumps or browser storage state are recorded. The suite
uses only synthetic accounts and does not replace `cargo test --workspace`.

This is document-level headless Chromium coverage. The 320px viewport checks
reflow, not actual 200% browser zoom; human readability/contrast assessment,
screen-reader behavior, address-bar navigation and other browsers still need
separate verification. See `docs/decisions/0005-headless-browser-tests.md`.

Manual keyboard check: Tab from the address bar to the skip link, then to Account;
sign in using the labelled username field and Enter, reach Sign out using Tab,
and activate it with Enter. Verify focus/layout at a narrow viewport and 200%
zoom. Also check that a sign-in error retains the username and that the logout
failure page's retry button is keyboard-accessible.

## Structure and dependency purposes

- `crates/game-data`: typed fixture records and catalog validation.
  `deunicode` supplies deterministic name-to-ASCII transliteration for automatic
  slugs; its version is pinned and existing addresses are not recomputed on reads.
- `crates/web-app`: catalog use cases, the identity-provider port, accounts and
  sessions, SQLx persistence, Axum routing, Askama HTML, custom CSS.
  - `src/auth/`: the provider port and the placeholder adapter.
  - `src/accounts.rs`: invite-only membership and opaque sessions.
  - `src/persistence.rs` + `migrations/`: SQLite queries and versioned migrations.
  - `src/application/authoring.rs`: scoped policy, transactions, versions and adoption.
  - `src/persistence/authoring.rs`: typed rows/queries and strict snapshot readers.
  - `src/web/authoring.rs`: native authoring/history forms and error rendering.
  - `src/web/addresses.rs`: pre-routing readable GET/HEAD addresses and ID compatibility.
  - `src/application/types.rs` + `src/persistence/types.rs`: versioned Type policy,
    exact typing, compatible full snapshots, cached reference/world initialization.
  - `src/web/types.rs`: native Type, adoption and ordered-typing forms.
  - `src/bin/admin.rs`: migration and invite operator CLI.
- `crates/tools`: fixture validator.
- `tests/browser`: Python `unittest` + Playwright for isolated real-browser
  keyboard workflows; test-only dependencies live in the optional Nix shell.
- `src/web/login_limit.rs`: bounded process-local login request budget.
- SQLx provides SQLite access and migrations; SHA-256 hashes session tokens;
  `getrandom` supplies tokens; Serde decodes fixtures. The POC that used a
  self-hosted Polycentric server lives outside this repository (see below).

## Next milestones and boundaries

The world-aware foundation authors name/description with automatic slugs, Type
definitions and exact ordered typing. Type effectiveness charts, stats,
moves/learnsets, items, abilities, natures, evolution/transformation rules,
maps/encounters, PokeAPI adapters and engine exports follow in separate domain
slices. The reference benchmark is documented in
`docs/architecture/reference-coverage.md`; Pokémon-specific limits belong to
versioned rulesets, not universal database constraints. Only a small cached Pokémon
metadata/typing subset is supplied; a general import/update-preview adapter and
complete mechanics model remain future work. World edit revisions are not yet
immutable complete-world packages. Archive/revert and ownership transfer/recovery
also require explicit future workflows. See decision `0006-world-aware-authoring`.

The independent Polycentric proof of concept was extracted from this repository to
`/home/eter/experiments/polycentric` and its local server stack was removed. It is
deferred, not adopted; decisions 0002 and 0003 record the status and the seam.

See `PROJECT_GUIDELINES.md` for the full project reference and `AGENTS.md` for
contributor instructions.
