# 0008 — Versioned Types, ordered typing and owned world fixtures

Decision [0009](0009-complete-fixture-bootstrap.md) supersedes the no-bootstrap
restriction for complete development setup. The targeted commands below retain
their original existing-owner/no-account-creation behavior.

## Context

The maintainer approved the next small gameplay-data slice: Types and ordered
form/world-variant assignments, with world fixtures. Effectiveness charts and
executable battle rules remain later slices. Existing immutable form, variant and
world histories explicitly accept snapshot v1, so extending DTOs alone would
break history or silently reinterpret old definitions.

## Alternatives

- Rebuild immutable version tables to relax their JSON CHECKs. Viable but risky:
  current-head FK cycles and retention triggers complicate ordinary transactional
  migration. Prefer additive structures for this feature.
- Keep typing as independent mutable rows or resolve latest types on read.
  Rejected: source pins, histories and world variants would change implicitly.
- Generic attribute/edge JSON tables. Rejected: these relationships have concrete
  ownership, ordering and foreign-key semantics.
- Synthetic fixture owner accounts. Rejected: actors and ownership are distinct;
  fixture worlds must have an explicitly selected real existing owner.

## Decision

### Shared and world-local Types

- Shared Types have canonical ids, typed current/immutable version rows, explicit
  archive state, automatic scoped slugs and administrator-only editing.
- World-local Types have independent ids, effective typed fields, world scope and
  optional exact shared-Type source pins. World owners/collaborators with current
  editor/administrator eligibility can create/edit them or explicitly adopt a
  newer shared version. Neither library nor local type edits retarget assignments.
- A form owns an ordered list of exact shared-Type versions. A world variant owns
  an ordered list of exact same-world local-Type versions. Duplicate Type identities
  are rejected and positions are contiguous from 1. A 32-slot software work bound
  is not Pokémon's two-type rule; a future ruleset may impose its own limits.
- Typing is explicitly `Unmodeled` or `Defined` (possibly an empty ordered list).
  The latter means an intentionally typeless authored definition; old v1 records
  remain unmodeled. No inferred typing from names or descriptions.
- Editing typing advances its owning form/variant revision and records before/
  after state and audit in the same short `BEGIN IMMEDIATE` transaction. Ordinary
  metadata edits preserve the exact typing state/pins. Stale revisions are checked
  before dependency validation. No-op commands create no extra history.
- Shared-form adoption does not map Types by name or arbitrary matching source.
  World typing remains local; the source's old/new assignments are shown for
  comparison, and editors explicitly select existing world-Type versions.

### Compatible full snapshots

- Leave migrations 0001/0002 and existing JSON bytes unchanged. Migration 0003 adds
  per-aggregate full-snapshot sidecars, a version marker defaulting to 1 on existing
  legacy rows, and typed immutable revision-keyed Type bindings.
- New form/variant/world content revisions are marker 2. Their existing version
  row retains a v1 metadata projection, while the required sidecar stores a strict
  v2 complete content snapshot. The same existing audit event attributes both.
  Species and private access snapshots retain their v1 schema.
- Before snapshots are the predecessor's authoritative stored after snapshot,
  which may be v1 or v2. Full history reads dispatch on the marker; a missing v2
  sidecar is an integrity error, never a fallback to metadata-only v1.
- Deferred exact-owner/revision FKs and insert guards require a v2 sidecar for
  every new affected revision, and forbid orphan/mismatched sidecars. Bindings are
  inserted before the sidecar/version seals them; late inserts as well as updates
  or deletes are rejected. Full snapshots and typed rows must agree on reads.
- World v2 membership snapshots include sorted Type ids alongside variant ids,
  excluding owner/collaborator/access data. These are still not whole-world engine
  packages. Adding a local Type advances world membership; editing it advances
  only its own revision.

### Explicit fixtures

- Add a bounded cached reference fixture: a few Types and Charizard base/Mega X
  form typing. Allocate internal ids once, separate from endpoint-specific external
  ids, and record source URL/id, UTC retrieval time and transformation version.
  Only known name/form/type data is projected; battle-only/reference limitations
  are described, not claimed as executable mechanics. No artwork is fetched.
- Add two synthetic world labs with multiple same-source variants, different
  typing, and world-exclusive entries/Types. These are demonstrations, not complete
  Pokémon editions or player saves.
- `admin seed-reference-fixtures` explicitly initializes the cached library data.
  `admin seed-world-fixtures --owner-user-id <id>` checks an existing enabled
  editor/administrator owner and initializes prerequisite reference data and the
  complete owned worlds in one transaction. No synthetic users or identity invites.
  Changes are attributed to `bundled-fixture-loader`, including private ownership
  history; private reads must include system actors without weakening access policy.
- Fixed fixture identities, complete creation snapshots and system provenance prove
  idempotence. Reruns compare creation baselines, skip matching complete worlds and
  preserve later edits/grants. Owner mismatch, drift, collisions, missing prerequisites
  or partial baselines fail the whole invocation. No automatic startup/migration
  seeding, implicit overwrite, or repair of an existing world.

## Boundaries, compatibility and checks

Application use cases own authorization/transactions; persistence owns SQL; shared
data definitions own validation; web uses native forms, ID-bound mutation targets,
readable slugs and 32 KiB limits. New Type/history pages follow existing scope rules.
No new external library or hosting service is needed.

Upgrade tests must preserve populated 0002 data, old snapshot bytes, pins, audit,
users and sessions; exercise v1→v2 history continuity, missing/invalid sidecars,
wrong/cross-world pins, sealed bindings, late-failure rollback, fixture ownership
and idempotence, foreign-key checks, and keyboard workflows at desktop/320px.
Back up existing databases with SQLite facilities before explicit migration;
older binaries reject the new schema. Normal tests use only local fixtures and
temporary test-owned databases/servers, with external requests disabled.

The requested Plan/Build types are unavailable. A read-only schema assessment
preceded this record; available general agents can build bounded modules.
