# 0006 — World-aware, versioned content authoring

## Context and confirmed requirements

The maintainer selected authored world definitions rather than runtime player
saves. Worlds explicitly adopt shared definition updates. Editors and
administrators can create worlds; creation assigns ownership to the authenticated
creator. Only administrators edit the shared library. World editing requires
ownership or collaboration even for administrators. Multiple variants of the same
species or form can coexist in a world. The local catalog and content history stay
public. Pokémon is a reference-model stress fixture, not the final game's schema;
authoring correctness precedes executable battle fidelity.

## Alternatives

- A global creature editor postpones world ownership and version scoping until
  after content exists. Rejected: these are foundational contracts.
- Live shared defaults and JSON override patches complicate reads, validation,
  and historical meaning. Rejected: store effective world values in typed columns
  and retain an exact source pin.
- One universal entity/edge JSON store loses meaningful foreign keys and query
  semantics. Rejected: concrete entities/relationships plus shared audit helpers.
- Whole-world edit locking causes unrelated variant edits to conflict. Prefer
  per-aggregate revisions with world membership and access tracked separately.

## Decision: first vertical slice

Implement species, forms, worlds, collaboration, world variants, editing,
version history, and explicit form-version adoption end to end. Reuse the existing
content fields (slug, display name, description), with plain-text paragraph
descriptions normalized to LF. Do not invent stats, effects, maps, or battle rules
in this migration. A reference-coverage document records their later acceptance
scenarios.

### Identities and typed state

- Existing fixture creature ids become shared species ids, preserving existing
  `/creatures/{id}` links. Species create an explicitly identified default form.
  Additional forms have separate random canonical ids and an immutable species
  parent. Species, form and world-entry identities never derive from names or
  external service ids.
- Species and forms have typed current rows plus immutable typed version rows.
  A form version pins an exact species version through a composite foreign key.
  Each successful edit immediately creates a new definition version; a separate
  publishing workflow can follow later.
- Worlds own independently identified variants. A variant stores effective typed
  content and either an exact `(form_id, form_version)` source pin or no source
  for world-exclusive content. Both source components must be present or absent.
  There is no unique `(world, species)` or `(world, form)` constraint. Slugs are
  unique within the world; variant `(world_id, id)` references are scope-safe.
- Archive state is explicit. This slice has no history deletion or archive UI.
  Referenced definitions and versions cannot cascade-delete.

### Authorization

- Viewer accounts are read-only. Editor/administrator accounts are eligible for
  world authoring, but a current owner/collaborator grant is also necessary.
- Ownership and its initial world record commit together. The owner alone manages
  collaborators; collaborators must be existing enabled editor/administrator
  accounts. Identity invitations remain trusted operator tooling.
- Ownership is retained when an account is disabled. Ownership transfer/recovery
  requires a later explicit use case; there is no silent administrative bypass.
- Validate the active session, role and scope inside each write transaction, using
  the same connection. Logout, disabling, and grant revocation therefore cannot
  race past a successful authorization check.
- Content history is public in this local milestone. Collaboration/access history
  is owner-only and separate from public content snapshots.

### Transactions, revisions and audit

- Use SQLx-managed `BEGIN IMMEDIATE`. Each mutation writes current state, its
  revision/version, and an attributed audit event in one transaction.
- Audit metadata owns actor, UTC timestamp, action and optional reason. Human
  actors reference immutable users; the constrained `bundled-fixture-loader`
  system key handles fixture initialization without a synthetic login account.
  Typed per-aggregate version/revision rows link to audit events and own
  schema-versioned before/after JSON snapshots. Live content is not JSON.
- Species/form edits advance their own revisions; variant edits advance only the
  variant. Adding a variant also changes world membership and records a world
  revision atomically. World metadata/membership has a content revision, while
  collaborator changes use a separate access revision and private audit history.
- Every update carries its starting revision. A stale command fails with no writes.
  Browser conflicts retain the submitted values, show current state, and permit an
  explicit reviewed resubmission; they never auto-merge or silently overwrite.
- No-op commands create no additional history. Reasons are capped at 500
  characters. Snapshots use version 1, independent of database and engine schemas.
- Public history is bounded and supports inspecting individual revisions.

### Initialization and adoption

- Only an explicit `admin seed-fixtures` initializes bundled content. It preserves
  species ids and creates default-form identities once, with attributed initial
  versions. Matching creation baselines are skipped even after local edits;
  inconsistent ids/slug collisions/fixture drift fail the entire batch.
- No automatic content initialization at startup or during migration. Browser
  catalog reads become SQLite-backed; an empty library is a supported state.
- A variant chooses a concrete form version when created. Library edits do not
  alter existing pins or effective values.
- Adoption previews the old source, local effective values and a selected newer
  version of the same form. The editor confirms effective fields and the new pin
  against the variant's starting revision. Retained local fields remain explicitly
  local; changing the baseline is not described as copying every source value.

## Implementation boundaries and compatibility

Add migration `0002_world_authoring.sql`; leave `0001_auth.sql` unchanged. Existing
accounts/sessions remain compatible. New binaries require both migrations and old
binaries reject the newer schema under existing readiness rules. Operators back up
existing databases with SQLite facilities before deliberate migrations. Tests use
only temporary databases.

Extend `game-data` with shared validation and typed content values. Application
use cases own policy/transactions; persistence modules own SQL; HTTP owns normal
links/forms, Origin/CSRF checks, and 32 KiB content-form limits. Reuse the already
pinned workspace `serde_json` library for revision serialization in `web-app`;
this introduces no new external crate/version or hosting service.

World edit revisions are not complete immutable world packages. Engine export,
PokeAPI import, curated reference fixtures, supported mechanics, and deterministic
simulation each follow their existing separate contracts. Future owned relationship
edits must advance their owning aggregate and snapshot the changed relationships.
PokeAPI source contexts are not application definition versions. The development
identity provider remains local-only and insecure.

The requested Plan/Build agent types are unavailable; read-only architectural
exploration and the maintainer's explicit choices preceded this decision. Build
work can use the available general agents with bounded module ownership.
