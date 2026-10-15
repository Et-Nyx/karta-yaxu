# 0011 — Independent stats, exact decimals and world rosters

## Context and alternatives

The maintainer approved independent stat definitions, an explicit ordered world
roster, signed decimal base values, configurable constraints and explicit adoption.
Neither names nor cardinality come from Pokémon. Fixed creature columns, generic
JSON attributes, floating point, live shared overlays and automatic creature-wide
rewrites are rejected. Base-value authoring does not define battle calculations.

The existing v1 metadata and v2 typing contracts are immutable. Their SQL CHECKs
cannot accept a v3 marker. Rebuilding populated version tables would introduce
unnecessary migration risk. Add an authoritative extension instead.

## Decision

### Values and definitions

- Use a dedicated exact decimal with signed 64-bit coefficient and scale 0–6.
  Normalize trailing zeroes and negative zero. Checked integer comparisons only;
  reject overflow, excessive precision, exponent notation and non-finite input.
  Snapshot decimals are canonical strings; typed live values use integer columns.
- Shared and world-local Stat definitions have immutable IDs, generated scoped
  slugs, effective metadata, optional inclusive minimum/maximum, whole-number-only
  flag, explicit archive state and independently attributed immutable revisions.
  Decimal and negative values are permitted unless authored constraints exclude
  them. Local definitions optionally pin an exact shared definition revision.
- Constraint changes never retarget consumers. History validates against exact
  pinned constraints, not current heads. A mechanically different stat needs a
  distinct identity; display text itself has no executable meaning.

### Rosters and creature values

- A world has at most one independently versioned ordered stat roster. No roster
  means unconfigured; an empty roster is intentional. Entries pin exact same-world
  definitions with distinct identities. Creating/editing definitions does not add
  or retarget entries. There is no fixed stat count or reuse of the Type limit.
- Shared forms may select shared definitions and optional reusable values. A
  variant's defined values pin an exact world roster revision and include every
  expected entry, with an explicit missing value where incomplete. Unmodeled,
  defined-empty, missing and explicit zero remain distinct.
- Definition, roster and creature edits advance only their owning aggregate.
  Stat catalogs/rosters have independent histories, like charts; existing world
  snapshots keep their documented Type/variant membership semantics.
- Roster reconciliation is explicit: retain values by identity, leave new entries
  missing, show removed values and changed constraints, and confirm the proposed
  state. Never round/clamp or discard values silently. Invalid carried values must
  be corrected or explicitly made missing before saving the new context. Older
  context is an authoring state, not storage corruption.
- Stat writes check both creature and current roster revisions, before dependency
  validation. Ordinary metadata/typing edits retain stat context exactly. No-op
  normalization creates no history.

### Adoption and policy

- Administrators author shared definitions/form values. World edits require an
  eligible owner/collaborator, checked with the session on the locked transaction;
  administrators have no all-world bypass.
- Local definition adoption compares old source, local and new source, and changes
  only that definition. Roster selection remains separate.
- Form-value adoption keeps local values by default. Copying requires explicit
  injective shared-to-local mappings whose local definitions have the same shared
  source identity. Never infer by name, position or an arbitrary duplicate source.
  Unmapped local values remain local. Missing source values clear local values
  only when explicitly selected. Revalidate selected values against exact local
  constraints. Source-pin/value changes commit in one creature revision.
- All mutations atomically seal current data, revision, complete snapshots and
  audit under BEGIN IMMEDIATE. Preserve drafts, conflict review, session-derived
  attribution, CSRF/Origin and existing request limits.

### Additive compatibility

- Use two additive migrations: 0005 for independent Stat/roster storage and 0006
  for creature snapshot integration. This separates the two integrity boundaries;
  deployment applies both deliberately. Migrations 0001–0004 remain byte-identical.
- New form/variant revisions retain required v1 metadata and v2 typing projections
  and require an authoritative v3 sidecar, selected by an additive discriminator.
  New typed stat bindings are sealed before their owner revision. Empty and
  unmodeled states require a seal too. Legacy revisions cannot acquire extensions.
- Legacy before chains retain their original projection contract. New v3 before
  snapshots equal the predecessor's actual authoritative v1/v2/v3 after bytes.
  Readers verify projections, typed bindings, exact dependencies, scope and seals;
  missing/inconsistent required data fails closed without fallback.
- Existing v1/v2 records remain stat-unmodeled, with original JSON bytes, pins,
  audit, users, sessions and charts preserved. Every creature writer (including
  fixtures, metadata, typing and adoption) uses the new sealing path.
- Only explicit migrations change schema and explicit fixture commands load
  content. Back up with SQLite facilities before upgrading a developer database.

### UI, fixtures and verification

- Native semantic pages cover definitions, roster, values, reconciliation,
  explicit mapping/adoption and public history. Readable GET/HEAD addresses and
  canonical-ID POSTs follow existing rules. Large catalogs/rosters use bounded
  pages/requests, not a fixed game cardinality. Display exact constraints, missing
  values and old roster context in text; retain raw invalid decimal input.
- Extend complete atomic setup with synthetic definitions, different-sized lab
  rosters and new sample forms/variants. Never retrofit old fixture creation
  snapshots or overwrite edits. Targeted old seed scopes remain unchanged.
- Test signed decimals/normalization/bounds; variable roster sizes; pins/scope;
  reconciliation/adoption; locked authorization; stale/no-op handling; immutable
  seals and rollback; v1/v2/v3 continuity and populated upgrade preservation;
  atomic fixture reruns; and keyboard authoring at desktop/320px.
- Run formatting, Clippy with warnings denied, workspace tests and the full pinned
  browser suite. Test servers/databases are temporary and servers are shut down.

## Consequences and boundaries

No new dependency or infrastructure is required. Snapshot projections duplicate
some data deliberately to preserve old contracts. Definitions and roster changes
can leave creatures on older baselines; reconciliation is an explicit workflow.
Derived values, level scaling, modifiers, formulas, rounding, battle execution and
engine packages remain later mechanics/export work.

The dedicated Plan/Build agent types are unavailable. A read-only architectural
assessment preceded maintainer approval. Available general agents implement
bounded modules against the agreed contract.
