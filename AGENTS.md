# AGENTS.md

Instructions for AI coding agents (OpenCode) working in this repository.

This file is a condensed operating guide. The full reference is `PROJECT_GUIDELINES.md`. Read the relevant section of it before any architectural change, and when this file says "see §N".

## Project

Creature Collector Design Wiki and Battle Lab: an educational web app that is the design database and wiki for an indie creature collector game. Designers browse and edit game objects in the browser, every change is attributed to a user, validated data is exported to the game engine, and combat is tested in PvE and PvP simulations.

- Game engine: Rust + Lua. Development OS: NixOS.
- Maintainer values control, learning, fine tuning, and reliability over speed or convenience.
- Apply Unix and suckless principles pragmatically. Aim for zero hosting cost, without promising it.
- Karta Yaxu means Monster Cards in Globasa: a creature collector with deck-building.
  Existing definitions ARE cards; player-owned objects would be instances with
  dynamic state. Worlds determine uses (including Type cards as attachments or
  resources), not fixed global gameplay classes. Engine integration and the battle
  simulator are the absolute FINAL product steps; authoring/fine-tuning comes first.

## Confirmed vs. proposed

Only the items under "Confirmed" are settled. Everything in the stack below is a proposed default (§1, §3).

- Confirmed: Rust + Lua engine, NixOS dev, simplicity, PokeAPI as temporary fixtures only, team editing with per-user history, engine export, eventual PvE/PvP testing.
- Proposed: Axum, Tokio, SQLx, Askama, HTMX, mlua, hosting approach, repo layout.
- Open (§21): library versions, auth provider, creature stat model, Lua dialect, export permissions, wiki visibility, hosting target, battle timeouts, backup cadence, export schema policy.

Do not treat a proposed default as agreed when it blocks you. Say so, choose the smallest reversible option, and record it in `docs/decisions/`. Do not block the initial skeleton on future battle-system details.

## Stack (proposed)

| Concern | Choice |
| --- | --- |
| Backend | Rust, Axum, Tokio |
| Database | SQLite via SQLx (migrations, transactions) |
| HTML | Askama server-rendered templates, normal links and forms |
| Browser | Small JS modules; HTMX if useful; TypeScript optional |
| Styling | Semantic HTML + small CSS token/component system |
| Serialization | Serde, versioned JSON |
| Combat | Shared deterministic Rust library |
| Scripting | Lua via mlua, only where mechanics need it |
| Dev env | Nix flake, locked Rust toolchain |
| Packaging | Native executable first; container only if the host requires it |

Commit `Cargo.lock`, `flake.lock`, and `rust-toolchain.toml`. Check current official docs and pick compatible versions at implementation time. Never guess a library's API from memory.

## Commands

Run inside `nix develop`. These become required checks once the workspace exists:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For authentication or authoring keyboard/focus/layout changes, also run from the project root:

```bash
nix develop .#browser-tests --command python tests/browser/run.py
```

The optional shell supplies pinned Python Playwright, matching Chromium and fonts.
The suite builds native binaries and runs 120 keyboard-only tests (58 earlier
workflows, 6 Moves, 8 learning/preset, 6 inspection and 6 export tests: three
inspection/export workflows each at both widths, plus five card workflows at both
widths including public card inspection/usage, plus five Workshop workflows at
both widths, plus five record-relationship workflows at both widths, two formula
workflows and one formula-export workflow at both widths) at desktop
and 320px widths against temporary test-owned databases/servers. No GUI or browser
download command is needed. Screenshots on failure go under ignored
`target/browser-tests/`; browser traces/cookie dumps are not recorded. This checks
document-level keyboard behavior and reflow, not actual browser zoom or human
visual/readability assessment. See `docs/decisions/0005-headless-browser-tests.md`.

Available project commands (inside `nix develop`):

```bash
cargo run -p web-app --bin admin -- migrate
KARTA_DEV_AUTH=1 cargo run -p web-app --bin admin -- seed-fixtures
cargo run -p web-app --bin admin -- seed-creature-fixtures
cargo run -p web-app --bin admin -- seed-reference-fixtures
cargo run -p web-app --bin admin -- seed-world-fixtures --owner-user-id <immutable-user-id>
cargo run -p web-app --bin admin -- seed-card-workshop --owner-user-id <immutable-user-id>
cargo run -p web-app --bin admin -- seed-card-relations --owner-user-id <immutable-user-id>
KARTA_DEV_AUTH=1 cargo run -p web-app --bin admin -- invite-user --subject alice --role administrator
KARTA_DEV_AUTH=1 cargo run -p web-app
cargo run -p web-app --bin admin -- list-users
cargo run -p web-app --bin admin -- disable-user --id <immutable-user-id>
cargo run -p web-app --bin admin -- enable-user --id <immutable-user-id>
cargo run -p tools -- validate-fixtures
cargo run -p tools -- validate-fixtures fixtures/creatures.json
cargo run -p tools -- validate-package path/to/package.json
```

The server is a local preview at http://127.0.0.1:3000, branded **Karta Yaxu**.
It has a SQLite-backed shared species/form library, owned worlds and multiple
world-local variants with attributed versions, public content history and explicit
source adoption. Editors/administrators create worlds; library editing is
administrator-only; world edits require ownership/collaboration. Access history
is owner-only. Names generate scoped slugs automatically on creation/rename via
pinned deunicode. Object GET/HEAD pages use readable scoped slug addresses; old ID
links temporarily redirect, while mutation forms stay ID-bound across rename/reuse.
Authentication is a development-only placeholder, disabled unless
`KARTA_DEV_AUTH=1`, and is not secure. Types and ordered exact form/variant typing
are versioned, world-scoped and explicitly adopted. Shared/local effectiveness
charts have independent revisions, exact rational factors and explicit source
mappings/adoption. Each world has at most one chart. Pair lookup uses canonical
Type identity across metadata revisions; Unknown, neutral, zero and outside-roster
are distinct. Multi-Type composition/damage remains unimplemented.
Old v1 history remains readable;
new form/variant revisions have authoritative v3 stat snapshots plus v1 metadata
and v2 typing projections; world revisions retain v2 full snapshots. Independent
shared/local Stats have exact signed decimals (64-bit coefficient, scale 0–6),
optional inclusive bounds and whole-number constraints. Each world explicitly
defines a variable-length ordered roster. Variant values pin a roster revision;
reconciliation and source mappings are explicit, missing differs from zero, and
definition edits never retarget consumers. Native editors save at most 25 displayed
entries per page, preserving off-page saved values; save before dirty navigation.
Confirmations bind reviewed proposals and history has readable paginated stats.
Independent shared/local numeric move attributes have descriptive units and exact
constraints, separate from creature Stats and without hardcoded names/counts.
Moves own exact attribute pins with Known/Unknown/NotApplicable states; unmodeled,
empty, absent and zero differ. Explicit injective source mappings and previewed
whole-collection replacement show removed/unmapped entries; no name-based inference.
Each world has at most one independent ruleset configuration: exact local attribute
expectations, required/NA/additional constraints, optional exact roster/chart pins,
optional maximum, unspecified/allow/forbid repeats and inert notes. Policy-2 reports
identify exact inputs, contradictions, incomplete and not_evaluated checks, never
combat readiness. Well-formed diagnostic drafts are saveable; structurally invalid
references/numbers or intrinsic violations are errors. One independent learnset
belongs to each configured shared form/local variant; stable scoped acquisition
IDs may repeat the same Move. Optional method/context/condition text is inert and
Known/Unknown/NA thresholds have no implicit level meaning. Multiple named variant
presets pin exact same-owner learnsets and same-world rulesets; selections are
ordered acquisition IDs, with canonical Move identity used for forbid-repeat checks.
Context reconciliation is explicit, never automatic. Form versions do not capture
learnsets, variant versions do not capture presets and world revisions do not
capture rulesets; no creature v4 snapshots were added. New selections require
unarchived definitions/Moves; retained exact archived pins permit unrelated edits.
Preset retention counts exact acquisition/Move pins including multiplicity, not
just identity membership. Retained exact chart pins allow unrelated ruleset edits.
New presets/ad-hoc reports use policy 2: impossible required numeric intersections
are contradictions, no maximum means no cap and unspecified repeats are unevaluated.
Saved policy-1 presets/fixtures retain historical semantics on ordinary edits;
current-policy revalidation is an explicit preview/confirmed attributed revision.
Native editors paginate at 25 entries, keep raw drafts including 403 failures and
bind preview confirmations to reviewed inputs/proposals. Long text uses one loaded
field per operation (`text_field`, `text`, `next_text_field`, `load_text`): load before
editing; save or explicitly discard dirty text before switching. Other texts stay
saved, without hidden-field duplication or truncation, within the 32 KiB form bound.
The complete `seed-fixtures` command atomically initializes all cached content plus
three synthetic chart examples and,
only when no users exist and dev authentication is enabled, a development admin
(default `admin`, configurable `--admin-subject`). Existing worlds retain their
eligible owner; otherwise a sole enabled admin is selected, or `--owner-user-id`
selects one explicitly. No implicit promotion, re-enable or ownership transfer.
The targeted creature/reference/world seeds create no accounts; targeted world
seeding requires an existing enabled editor/administrator owner.
General network imports, CLI package creation, and simulation commands do
not exist yet. Browser frozen design-package export and offline `validate-package`
are delivered (ADR0015). Only `admin migrate` creates a database; the server and other
commands require an existing, compatible schema (migrations 0001–0010).
Chart snapshots use their own v1 schema; existing world v1/v2 JSON bytes and
membership semantics remain unchanged. Only complete setup seeds charts, not
the targeted creature/reference/world commands. Complete setup also adds synthetic
Stats, three-/four-entry rosters and sample form/variants. The ADR0012 batch uses
those existing StatsSample owners: 6 move attributes, 2 shared/4 local Moves,
2 rulesets, 3 learnsets and 4 presets add 21 objects without changing the previous
44 creation baselines or old object heads/history. Complete setup totals 65 objects
and 69 initial audit events; reruns skip 65 objects. Targeted old seeds create no
move/learning objects. Partial batches, custom singleton conflicts or last-preset
entry/seal failures reject/roll back the entire setup, including bootstrap changes.
Migrations and server startup never seed content. Derived stats, formulas,
modifiers, executable acquisition/effect rules and battle calculation remain future work.
Only explicit fixture commands initialize content, never overwriting edits.
Authoring forms have a 32 KiB limit and optimistic conflicts require explicit review.
Disabling an account atomically
revokes all its sessions; re-enabling requires a new sign-in. Authentication forms
are limited to 4 KiB and login POSTs share 30 attempts per 60 seconds per process.
The independent Polycentric proof of concept was moved out of this
repository to `/home/eter/experiments/polycentric`. The full reference currently
exists as `PROJECT_GUIDELINES.md`. See
`docs/decisions/0014-world-inspection-and-dependency-visibility.md`,
`docs/decisions/0013-authoring-validation-policy-and-archive-selection.md`,
`0012-moves-learning-and-world-rulesets.md`,
`0011-independent-stats-and-world-rosters.md`, `0010-type-effectiveness-charts.md`, `0009-complete-fixture-bootstrap.md`, `0008-types-and-world-fixtures.md`, `0007-readable-object-addresses.md`, `0006-world-aware-authoring.md`, `0005-headless-browser-tests.md`,
`0004-authentication-refinement.md`, and `0003-authentication-foundation.md`
for approved choices, README for the current workflow, and
[the pinned roadmap](docs/architecture/roadmap.md) for all deferred directions.

If a command cannot run in your environment, say so in your report instead of skipping it silently.

### Delivered world inspection contracts

- Public read-only GET/HEAD `/worlds/{world}/inspection` and
  `/worlds/{world}/inspection/where-used`; world detail links **Authoring inspection**.
  Native typed/allowlisted GET filters only: no POST, CSRF, new auth policy, cached
  report or writes. No schema, dependencies or fixture objects were added; existing
  labs remain 65 objects/69 initial audit events.
- Capture resolves the world as the first read in one ordinary SQLite read
  transaction, enumerates all independent current roots including archived/unused
  local objects and explicit absences, then expands exact dependencies. Shared
  sources enter only through explicit edges; no implicit source-form learnset.
  Older pins and every acquisition/preset occurrence survive. Current catalog/
  ownership observations are not historical world-revision pins. Finish the read
  transaction before fingerprinting, analysis, usage traversal or rendering.
- Only explicit public DTOs cross capture boundaries: private WorldRecord owner/
  access data, actors, grants, sessions, raw audits and raw snapshot JSON are never
  reported or hashed. Immutable kind/scope/id/revision keys and captured data build
  exact-history links/labels; no post-capture database lookups for rendering.
- Current Move/ruleset diagnostics use policy 2. Saved presets retain policy 1/2;
  optional same-input policy-2 comparison saves nothing. Contradiction, Incomplete,
  ContextDifference, notEvaluated and separate coverage gaps never imply combat/
  export readiness. Reached archived dependencies/old pins are informational, not
  automatically invalid. Keep numeric states and scoped canonical Type identity
  across metadata revisions, including chart neutral/zero/Unknown/outside distinctions.
- SHA-256 fingerprint only complete captures over deterministic public semantics:
  current roots/absences, exact nodes/edge occurrences and observed heads. Exclude
  time, private/access data, filters and pages. Every request captures anew;
  navigation carries the prior fingerprint, changed input announces refresh and
  resets page 1. This is not frozen history or a package checksum. Partial captures
  have no fingerprint: failed/unsupported/budget reads are gaps, not Unknown data.
- Where-used covers all CAPTURED versions or an exact revision in this world, with
  direct consumers and representative bounded shortest root paths. Catalog/owner
  membership is not usage; acquisitions differ from actual preset selections.
  No global/all-history/safe-delete claim; never label partial coverage “Unused.”
- Operational defaults: roots 2048, nodes 8192, edge occurrences 65536, per-snapshot
  preflight 1 MiB, total accounted snapshot bytes 16 MiB, deadline 10 seconds across
  acquire/anchor/capture. Preflight and per-request caches bound outer work, but
  nested existing-reader verification is not a perfect cost meter. Safe SQLite
  progress callback checks about every 1000 VM operations; remove handlers and
  roll back on cleanup, with close-on-drop service leases for cancellation safety.
  Five-second busy waits, filesystem/non-VM work can outlast the cooperative deadline;
  do not call it hard real-time. At most two public inspection jobs per service,
  admission held through render; fingerprint/analysis/path work uses spawn_blocking.
  Shared Arc exact evidence avoids quadratic clones; rendered lists and finding
  evidence previews show at most 25 with remaining counts, retaining full typed data.
  Paths: 256 results, depth 32, work 65536, explicit truncation. No new game caps.
- Helpers: `crates/game-data/src/inspection.rs` (typed capture/edge/limit contracts),
  `inspection_analysis.rs` (pure findings/usage); `crates/web-app/src/` contains
  `application/inspection.rs` (admission, capture, fingerprint, blocking work),
  `persistence/inspection.rs` (bounded checked readers, preflight, interruption,
  public capture) and `web/inspection.rs` (typed GET filters and bounded rendering).
  Preserve legacy companion/predecessor typed metadata/header verification and
  SQLite cancellation cleanup regression coverage.
- ADR0014 records the approved inspection foundation; ADR0015 separately delivers
  frozen design packages. Inspection itself does not implement export. See README
  and the pinned roadmap.

### Delivered frozen design-package contracts

- Authorized native GET/HEAD `/worlds/{world}/export` entry, ID-bound CSRF-protected
  POST preview/confirmed attachment. Eligible enabled owner/collaborator only, no
  global administrator bypass. Permission checked in the capture transaction and
  rechecked before release; confirmation binds package-specific content digest.
- Active world catalog roots: unarchived local definitions/variants/chart,
  configured roster/ruleset, learnsets and unarchived presets of active variants.
  Exact closure includes needed archived/older revisions. Identity-only owners and
  default Forms use explicit same-snapshot current-head bindings, not invented pins.
  No membership closure or implicit source Form learnset. Preserve independent
  Learnset source adoption and repeated acquisition/preset occurrences.
- `game_data::package` provides a DB/network-free v1 reader. Dedicated envelope
  locks reused public payload wire representations: strict fields/order/encodings,
  sorted keyed vectors, explicit nulls, same-scope exact closure/embedded equality,
  required bindings, no unrelated extras. JSON whitespace allowed. Manifest SHA256
  uses its own domain over canonical content, never inspection fingerprint.
- Structural acceptance permits well-formed drafts/contradictions/Unknown; separate
  diagnostics use current policy2 and retained preset policy1/2. Invalid numbers,
  broken references, unsupported formats/policies and capture gaps block export.
  No actual engine currently consumes the package; no gameplay readiness claim.
- World-first ordinary read capture reuses checked readers/preflight/cancellation
  cleanup. End transaction before CPU work. 32 MiB input/output limit, capture
  defaults as inspection, at most two export jobs/service held through rendering;
  blocking workers retain admission on cancellation. Fully serialized bytes pass
  actual shared reader before private/no-store attachment. No retained artifact.
- `tools validate-package [path|-]` defaults to stdin, bounded reads and independent
  offline diagnostics. No migration/content/audit/fixture changes (65 objects/69
  events); reuse existing workspace sha2/hex only. See ADR0015 and
  `docs/architecture/design-package-v1.md` for exact protocol/compatibility rules.

## Repository layout

### Derived values foundation (ADR0019)

- Independent shared/local formulas, not Card wrappers or base Stat mutations.
  Stable named inputs and ordered steps reference constants, inputs or strictly
  earlier steps; explicit output. Semantics1 supports arithmetic/min/max/clamp and
  floor/ceiling/toward-zero/nearest-ties-even rounding to scale0..6. Reduced signed
  i64 numerator/positive i64 denominator, checked i128 intermediates, cancellation
  before range checks; no implicit rounding or floating point.
- Known exact Decimal/Missing/Unknown/NA inputs; omitted is Missing. Constraints
  validate, never clamp. Blocked provenance differs from division-by-zero/overflow/
  invalid-clamp errors; zero times Unknown stays blocked. Each step runs once.
  Valid definitions can fail for particular samples and remain saveable/exportable.
- Typed migration0011 adds heads, immutable versions/inputs/steps/operands, checked
  snapshot/audit evidence and atomic attributed revisions. Shared admin/local enabled
  owner-collaborator writes, expected-first/no-op semantics. No old seed changes or
  automatic migration. Native /formulas and world-prefixed catalogs/history/editors,
  25 input entries/five steps per editor page,32KiB drafts, explicit conflict rebase,
  raw errors and strict operation/query/form allowlists.
- Public exact-history /calculate GET/HEAD/POST is read-only, no login/CSRF, private/
  no-store, no saved samples/results/audits. Exact revision bound; two calculator
  jobs held by blocking workers including cancellation. Trace/reasons bounded25.
  Format limits100inputs/200steps are operational, not global game caps.
- Same-snapshot public local formula inventory (including archives), combined
  preflight/deadline budgets, fingerprintv4 and /inspection/formulas. No external
  formula dependencies or invented usage. Schema4 FormulaPackage wraps unchanged
  records-v3/cards-v2/typed-v1 plus exact active-local formula closure; hash domain
  karta-design-package-content-sha256-v4 newline. Frozen readers1–3 unchanged;
  tools accepts1/2/3/4; authorized final-byte reader/recheck and32MiB bound remain.
- No live bindings, formula calls, source adoption, modifier execution, scripts or
  battle semantics. Examples are manual docs, no new seed. Verification:538 Rust
  cases full-workspace plus one subsequent targeted populated10→11 upgrade case;
  all120 browser cases passed across a timed-out full attempt and remaining-module
  run, not one completed full run. See ADR0019, derived-values.md and package-v4.

### Card foundation (ADR0016)

### Card records and content relationships (ADR0018)

- Reusable shared/local flat record layouts own stable ordered column IDs: exact
  same-scope scalar fields or declared Card/Stat/Field reference-kind allowlists.
  Reusable named collection definitions independently pin exact same-scope layouts.
  No global Item/Ability/Nature/Resource classes, nested records or execution.
- Card record companions use the same qualified identity/owning revision and audit
  as scalar/native components. Every writer carries them forward atomically. Migration
  0010 preserves all old bytes/heads/audits, including standalone-card record legacy
  boundaries; pre-boundary records are unmodeled, missing new seals fail closed.
- Collection inventory unmodeled/empty/absent, collection Missing/Unknown/NA/Known
  entries, absent cells and cell Missing/Unknown/NA/typed Known are distinct. Repeated
  targets preserve stable row/cell occurrence sites; false/zero/empty text are values.
  Shared references stay shared, local targets may be shared/same-world; never foreign
  world/private refs. Stat and reusable-field identities are not wrapper cards.
- Named native /record-layouts, /record-collections and qualified /cards/.../records
  editors/history use 32 KiB ID-bound CSRF forms, one loaded prose, exact constraints,
  retained raw errors/revocation/conflicts and explicit reviewed rebase. Reconciliation
  and source-copy use injective collection/column mappings, complete bound previews,
  removed/unmapped evidence and locked equality checks; no name inference/coercion.
- World-first inspection adds all current local layout/collection roots and exact
  records/targets with combined budgets; fingerprint v3 hashes public evidence only.
  /inspection/records and /where-used alias the mixed bounded usage view, preserving
  repeated cell sites. Partial has no fingerprint/Unused claim. Typed-only usage
  reports do not claim complete coverage of record edges.
- New exports use record_package::RecordPackage schema3, with schema2 card projection
  plus full layout/collection/relation closure. Frozen v1/v2 public reader behavior
  remains unchanged; tools validate-package dispatches 1/2/3. Canonical v3 SHA256
  domain, 32 MiB bound, authorized recapture/review/recheck and actual shared-reader
  acceptance remain required. No silent projection-only download.
- Optional seed-card-relations --owner-user-id creates a separate original Card
  Relations Workshop: 28 objects/29 initial system audits, owner already enabled
  editor/admin, no accounts or old seed changes. Exact immutable baselines, partial/
  collision/wrong-owner rejection, rollback and edited reruns preserve old65/69 and
  Workshop15/16. Prose/values illustrate costs/grants/traits/influences, not mechanics.
- Source: game-data relations.rs/record_inspection.rs/record_package.rs; web-app
  application/persistence relations.rs, inspection/record_capture.rs, web/relations
  and application/relation_workshop.rs. See ADR0018, card-records.md and design-package-v3.md.

### Card Workshop refinement (ADR0017)

- Named selectors/search APIs filter typed SQL before 25-result paging, preserving
  exact field/card/choice/source pins and current/historical archival context.
  Catalogs support literal name, exact label, origin/kind and archive filters.
  Technical origins and example labels impose no gameplay classes.
- Selector search/page/choose/load/mapping operations POST to the same ID-bound
  CSRF editor, preserving raw drafts without writes. Known controls derive from
  the exact field; Missing/Unknown/NA and zero/false/empty remain distinct.
  Dirty text requires explicit save/discard; loaded markers bind the exact field
  and owning card revision. One prose value per request, no hidden duplication.
  Do not use HTML maxlength for Unicode text (UTF-16 can truncate valid input).
  Advanced coordinates/inactive values remain available behind native disclosure.
- Mapping drafts contain the whole explicit injective collection across pages;
  preview/confirmation still bind exact inputs, removals and unmapped fields.
  Conservative encoded-size checks preserve prior/pending drafts rather than
  truncating additions that would exceed the 32 KiB transport bound.
- Optional `seed-card-workshop --owner-user-id` requires an existing enabled
  editor/admin, creates no accounts, and is separate from every old seed/startup/
  migration. One transaction creates 15 objects/16 initial audit events under
  bundled-fixture-loader with reason Original Card Workshop example v1 (ADR0017).
  Baseline verification, partial/collision rejection, rollback and edited reruns
  preserve original evidence. Old lab seeds retain 65 objects/69 events.
- Source: application/card_workshop.rs (typed immutable original manifest),
  application/cards/search.rs, persistence/card_search.rs, web/cards/pickers.rs.
  No new migration/package format/dependency. Engine/simulator remain last.

### Card foundation contracts

- Forms, world Variants, Types and Moves ARE cards at qualified native identity
  (namespace/scope/id) and the same owning revision. No wrapper IDs or gameplay
  classes. Species remains a grouping; standalone cards have no required family.
  Player-owned instances and dynamic state are later work.
- Reusable shared/local field definitions support constrained exact numbers, LF
  plain text (0–2000 characters), booleans, choices with stable option IDs and exact
  qualified card references. Fields pin exact same-scope definitions; Missing,
  Unknown, NA, absent, unmodeled/empty, false, zero and empty text remain distinct.
  Local references may name shared/same-world cards; no foreign world reference.
- Rules text and labels are inert. Existing Stats/Move attributes remain separate.
  Every native writer carries components forward and shares its audit atomically;
  custom-only edits advance the owning revision, so old native drafts conflict.
  Same-event immutable companions are checked; missing new seals fail closed.
- Migration 0009 records immutable legacy boundaries without rewriting existing
  values/heads/snapshots/audits. Pre-boundary card data is unmodeled. Fixtures keep
  65 objects/69 audits; no automatic standalone-card/field seed. Only deliberate
  `admin migrate` upgrades; backup first. Never silently skip missing card tables.
- Public Cards/Fields and attributed exact histories; operation-scoped native forms
  load one prose field and at most 25 displayed entries, preserve raw errors, use
  ID-bound CSRF POSTs and explicit conflict/archival review. Source adoption keeps
  local values; whole-field replacement needs explicit injective mappings and
  bound previews. Textarea CRLF is transport-normalized to LF, never truncated.
- Public `/worlds/{world}/inspection/cards` and `/cards/where-used` suffix extend
  the same capture with card components/fields and mixed bounded usage. Fingerprint
  v2 covers complete card evidence, roots and head observations; partial has none.
- ADR0016 introduced schema-2 `card_package::CardPackage`, with a typed schema-1
  projection plus complete card/field closure. Never silently omit components.
  Frozen v1 reader/wire remains unchanged. ADR0018 now extends downloads to schema3;
  validate-package accepts all three formats.
  See `docs/architecture/card-foundation.md` and `design-package-v2.md`.

Create crates and directories only when they have code or docs to hold. Start with `game-data`, `web-app`, and `tools`; add `battle-core` when simulation work begins (§5).

```text
crates/game-data      shared typed definitions + validation
crates/battle-core    pure mechanics; no HTTP, no SQL
crates/game-scripting Lua integration (only if needed)
crates/web-app        main.rs, config.rs, domain/, application/, persistence/, web/, migrations/, templates/, static/
crates/tools          import/export + simulation CLI
schemas/ rules/ fixtures/ tests/ docs/{architecture,decisions,operations}/ deploy/
```

Keep runtime data, database files, credentials, and generated exports out of tracked source.

## Architecture rules (modular monolith, §4)

| Layer | Owns | Must not own |
| --- | --- | --- |
| Domain | object types, invariants, battle transitions | HTTP, templates, DB connections |
| Application | use cases, authorization policy, transactions | page layout, scattered SQL |
| Persistence | queries, migrations, durable state | combat rules, browser logic |
| Web | routing, parsing, sessions, forms, rendering | a second copy of game mechanics |
| Adapters | PokeAPI import, engine export, script glue | core rules tied to an external service |

- Handlers translate input into application commands; services check policy and coordinate repositories; repositories translate to SQL. Keep routes thin.
- Add interfaces only at real boundaries. No generic repository framework, no trait per function.
- `battle-core` must build and run with no web server and no database. No mutable globals in the domain.

## Hard invariants

Never break these without an explicit instruction from the user. If a request conflicts with them, explain the conflict and propose the smallest coherent alternative.

### Data and schema (§6)
- Use immutable object ids, separate from display names and slugs.
- Never use a PokeAPI id as a canonical id.
- Use typed columns for queried fields. No universal entity table of arbitrary JSON. JSON only for revision snapshots or schema-validated effect parameters.
- Use foreign keys or validated typed references. Add indexes only for real queries.
- Store UTC timestamps. Mark archived/deleted state explicitly.
- Version migrations, export schemas, rulesets, and battle protocols separately.
- SQLite: enable foreign keys on every connection, consider WAL, use a bounded busy timeout, keep write transactions short, bound the pool, and never run blocking DB or CPU work on async handlers. Never put a WAL database on a network filesystem.

### Editing and audit (§7)
- Every game-data mutation commits the object update, the revision, and the audit event in one transaction. A failed write leaves none of them.
- Record actor id, timestamp, action, object type/id, previous and new revision, and before/after snapshots. A change reason and request correlation id are optional.
- Never take the actor id from a browser-editable field. Imports are attributed to the initiating user or an explicit system actor.
- Use optimistic concurrency: an edit carries its starting revision. On conflict, show the newer record and let the editor resubmit. Never silently overwrite.
- A revert creates a new attributed revision. Never delete history.
- Do not call audit logs tamper-proof.

### Auth and security (§8)
- Invite-only, individual accounts. Enforce authorization inside every use case, including JSON endpoints and WebSocket commands.
- Passwords: an established implementation such as Argon2id. No homemade crypto.
- Server-side sessions with opaque random tokens, expiry, revocation, and Secure/HttpOnly/SameSite cookies. CSRF protection on cookie-authenticated mutations.
- Escape rendered text, parameterize all SQL, cap payload sizes, rate-limit login and simulation.
- Never log passwords, tokens, secrets, or sensitive raw requests.
- Editing numbers must not imply permission to run Lua. Export and script-publish rights are separate (open decision).

### Export (§10)
- The engine consumes a versioned package, never the live DB or an HTTP dependency.
- The package has `schema_version`, ruleset/content ids, canonical ids, validated collections, a manifest, and checksums.
- Serialize deterministically. Validate through the real engine importer or the shared parsing library. Publish atomically, only after validation succeeds.
- Never export sessions, credentials, user profile data, or admin history.

### PokeAPI (§11)
- It is an import adapter, never a runtime dependency for browsing or combat.
- Imports are idempotent, previewed before overwriting local edits, and labeled as placeholder. Record source, external id/URL, retrieval time, and transformation version.
- Use timeouts and bounded retries. Disable external fetching in normal tests and use local fixtures.
- Do not assume permission to ship Pokémon names or artwork in the final game.

### Combat (§12)
- Core shape: `step(rules, content_snapshot, battle_state, validated_command, rng_state) -> updated_state + ordered_events + updated_rng_state`.
- No DB calls, network access, wall-clock reads, or ambient randomness inside mechanics.
- Use a versioned RNG with recorded seed/state. Prefer integer or fixed-point math. Define tie ordering. Never depend on hash-map iteration order.
- Validate every command. Emit structured events. Bound simulation work.
- Store initial state, commands, rules/data versions, and RNG info for replay. A seed alone does not guarantee replay across code changes.
- PvP: the server owns state and clients send intent only, never damage or results. Pin a rules/content snapshot at match creation. Reveal only permitted information.
- Build order is CLI simulation, then browser PvE, then turn-based PvP.

### Lua (§13)
- Match the engine's exact Lua dialect. Lua, LuaJIT, and Luau are not interchangeable.
- Only trusted, reviewed, version-controlled scripts. Defer browser script uploads.
- No filesystem, shell, network, module loading, secrets, time, or ambient RNG for scripts. Provide host-controlled RNG and bounded host functions.
- Limit instructions, memory, recursion, and event count. Pin script hashes.
- Commit a battle transition only after all script work succeeds, or follow a documented failure policy.
- mlua does not make arbitrary Lua safe. Its documented sandbox is Luau-specific.

## UI and accessibility (§9)

Primary users include nontechnical designers.

- Native controls and semantic HTML first. Visible focus, keyboard access, sufficient contrast, responsive layout.
- Never convey validation or combat state by color alone.
- Labeled forms with help text and units. Show errors beside fields and keep entered values on failure.
- Clear archive/delete wording with confirmation for destructive actions.
- Explicit loading, failure, reconnecting, and conflict states.
- Do not adopt a large frontend framework just to render CRUD forms.

## Dependencies and infrastructure

- Add a dependency only for a concrete problem, and explain it in the change. Track why each one exists.
- A well-maintained library can reduce total complexity (auth, password hashing, crypto). Prefer one over homemade code.
- Docker is optional. If a container is required: small image, non-root user, durable data mounted separately, no secrets and no live SQLite file in the image.
- Do not pick a hosting provider. That decision needs verified current terms (§16).
- Reject ephemeral-only storage for the live database. Ask before changing the SQLite architecture, for example moving to a managed remote database.

## Operations (§16, §17)

- Back up with SQLite's backup facilities or a quiesced procedure. Never `cp` the main DB file during live WAL writes.
- Back up before migrations. Migrations are deliberate and never auto-destructive. Prefer compatible, staged migrations.
- Keep dev, test, and prod databases separate. Keep secrets in an ignored local config or environment variables, and commit only a safe example.
- If SQLx compile-time checked queries are adopted, define the offline metadata workflow so builds never need production access.

## Testing (§18)

Test invariants and user workflows, not implementation details. Use temporary databases and local fixtures. Keep network-dependent checks out of normal runs.

Cover these as the relevant area is built: domain validation, migrations and transactional rollback, audit attribution (and no partial history on failure), per-role authorization tested server-side, stale-revision rejection, export consistency and importer acceptance, deterministic combat replay, Lua resource limits and errors, PvP duplicate/reordered/wrong-user commands, backup restore, and keyboard-only browse/edit.

## Working in OpenCode

- Use the Plan agent for architecture, schema, or contract changes. Present the options, and write a short decision record (context, alternatives, decision, consequences) in `docs/decisions/` before the Build agent implements.
- Work in small, reviewable changes with one clear user-visible goal. Read the files you will change before editing.
- Ask before any of the following, unless the current task explicitly authorizes it:
  - touching production data or credentials
  - running destructive migrations or deleting databases or backups
  - publishing or deploying
  - creating paid or external resources
  - adding infrastructure or new dependencies
- Do not edit `PROJECT_GUIDELINES.md` unless asked. If it is wrong or stale, say so and propose the edit.
- Keep secrets and personal account data out of source, fixtures, exports, and logs.
- Treat the user's explicit current instruction as taking precedence over these guidelines. Flag the conflict first, then comply.

Load detailed sections on demand rather than all at once:

- Schema or migration work: read `PROJECT_GUIDELINES.md` §6–7.
- Auth or sessions: §8.
- Export or importer: §10–11.
- Battle-core or Lua: §12–13.
- Deploy or backup: §15–17.

## Definition of done

- The intended workflow works end to end.
- Invariants hold and the relevant checks pass: fmt, clippy with `-D warnings`, and tests.
- Error paths are understandable to a nontechnical user.
- Schema, runbook, README, or decision record is updated when behavior changes.
- Compatibility and migration effects are documented.
- No unexplained dependency or deployment cost was introduced.

## Final report format

End each task with:

1. What changed and why (one short paragraph).
2. Checks run, with pass/fail. List checks not run, and why.
3. Remaining limitations, open decisions touched, and any guideline conflicts.
