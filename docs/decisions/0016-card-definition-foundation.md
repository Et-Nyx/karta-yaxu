# 0016 — Same-identity card definitions and reusable typed fields

## Context and alternatives

The maintainer clarified Karta Yaxu (Monster Cards) as a creature collector with
deck-building: creatures, Moves, Types, Abilities and Items can be cards. A world
decides whether a Type card is an attachment, a resource or something else. The
existing shared/world creature is a definition; a player-owned creature would be
an instance with its own dynamic state. Engine integration and simulation are the
absolute final product steps, not the next milestone.

The maintainer selected **definition is card**, **typed custom fields**, and
**reusable field definitions**. Independently identified wrappers, fixed global
Equipment/Consumable classes, universal arbitrary entity JSON, and rewriting old
objects/history were considered and rejected. A read-only architectural assessment
used the available exploration agent because a dedicated Plan agent is unavailable.

## Decision

### Identity and revision

- Concrete shared Forms and world Variants, shared/local Types and Moves are card
  definitions. Species remains a grouping. New standalone shared/local cards use
  the same authoring contract without a required specialized gameplay class.
- Preserve immutable IDs and qualified identities: technical origin namespace,
  shared/world scope, and ID. Existing tables do not guarantee cross-kind ID
  uniqueness. Technical origins impose no gameplay meanings. Preserve scoped slugs.
- The complete existing card revision is its existing owning definition revision.
  Custom-only edits advance that revision; every existing writer carries its card
  component forward. Stage/seal the component and specialized content in one
  transaction with the same audit event. No second mutable metadata authority or
  independent card counter. Complete card reads verify component/owner/revision
  agreement and exact field/reference dependencies.
- Add a schema/compatibility boundary for legacy revisions: existing bytes, IDs,
  audits and heads remain unchanged; pre-boundary custom fields/rules text are
  unmodeled, not fabricated empty authored data. Migration bookkeeping must not
  invent card-creation actors or events. New owning revisions require complete
  sealed card components. Missing post-boundary components fail closed.
- Learnsets, presets, rulesets, charts and rosters remain independent. A creature
  card pin does not invent historical pins to these independent current objects.

### Reusable field data

- Shared/local field definitions have immutable IDs, scoped generated slugs,
  normalized metadata, revision/archive state and optional exact shared sources.
  Field kinds: exact number with constraints, bounded plain text, boolean, single
  choice with stable option IDs, and exact qualified card reference. Fields are
  reusable definitions, not required classes/layout templates or executable rules.
- Cards select ordered distinct field identities at exact same-scope revisions.
  Embed verified field definitions in immutable component snapshots. Values have
  Missing, Unknown, NotApplicable and typed Known states; absent field, unmodeled
  collection, defined-empty, zero, false and empty text are distinct. Value shape,
  constraints, stable choice IDs and references are structurally checked.
- Shared cards reference shared cards; local card reference values may explicitly
  name shared or same-world cards, never another world's content. Exact pins are
  retained, not automatically advanced. Read cycles with bounded visited queues.
- Rules text is optional inert plain text distinct from description; optional
  normalized labels organize cards without automatically imposing behavior.
- Existing Stats and Move attributes retain their identities and exact semantics;
  do not merge them by names into the new field system. Fields named Cost or
  Equipment have no built-in mechanics. Changing a field type/options creates a
  new revision and never reinterprets old values; selecting that revision requires
  explicit compatible values. Retained archived exact pins permit unrelated edits;
  new/retargeted selections require selectable unarchived definitions/cards.
- Source adoption defaults to keep-local. Source-field replacement is explicit,
  previewed and confirmed, with injective field identity mappings whose local
  definitions explicitly source the shared identity. No name-based inference.
  Existing specialized adoption operations preserve card components. Copying
  fields from an already pinned source is separate from silently retargeting it.

### Authoring, persistence and capture

- Concrete typed columns/child tables and validated references, immutable revision
  snapshots, atomic revision/audit, expected-revision-first conflicts and no-op
  suppression follow current patterns. No new dependencies or hosted resources.
- Shared editing remains administrator-only; local editing requires eligible
  owner/collaborator, checked with the session under BEGIN IMMEDIATE. No global
  administrator bypass or browser-supplied actor. Old editors cannot bypass a
  newer custom-field revision or erase fields they did not edit.
- Native Cards/Fields catalogs, public detail/history, rules-text and operation-
  scoped field editors use bounded allowlisted forms, keep raw drafts on failure,
  show exact contexts and explicit conflicts. Lists paginate at 25, long prose is
  loaded/submitted per operation within 32 KiB, not duplicated in hidden fields.
- Inspection must include standalone cards, reusable local fields, existing card
  components and exact reference closure. Public-only capture finishes one ordinary
  world-first read transaction before CPU analysis/rendering, with the existing
  budget, interruption, cancellation and admission guarantees. Partial capture is
  explicit and never treated as authored Unknown data.

### Frozen packages and compatibility

- Keep the v1 reader, wire fields/enum ordering and old files readable unchanged.
  New card-aware v2 exports include complete components, standalone cards, field
  definitions and exact reference closure. Do not silently export only a legacy
  projection as the complete card. Version package roots/checksums/reader contracts
  separately from DB migrations, typed creature snapshots and validator policies.
- Reuse authorized preview/confirmed download, same-snapshot permission, reviewed
  digest and final shared-reader acceptance. No engine integration is involved.
  Existing exact typed dependencies remain checked; no private access/user/audit
  information crosses package/capture boundaries. Unknown diagnostic drafts remain
  structurally representable. Operational budgets are not new gameplay caps.

## Consequences and verification

All existing owning write paths (creation, metadata, typing, Stats, adoption,
reconciliation and fixture setup) participate in the same-revision bridge. Current
typed views remain available. Technical schema changes do not seed new authored
objects or rewrite earlier creation baselines. Standalone cards support designing
Items/Abilities now without inventing fixed gameplay classes.

Test populated upgrades preserving every prior table value; cross-kind/scope ID
collisions; complete new seals and corrupt companions; atomic late rollback and
attribution; every legacy writer preserving custom fields; stale and no-op behavior;
shared/local authorization/revocation; typed value distinctions/constraints/choice
identity; explicit source mappings/adoption; exact cyclic reference capture;
inspection privacy/budgets/freshness; old v1 and new v2 offline acceptance; keyboard
authoring at desktop/320px. Run fmt, Clippy, workspace and browser checks.

Player-owned instances, decks/hands/discards, attachment execution, resource
spending, dice rules, formulas, Lua, engine integration and simulation are later
work. This foundation records definitions and intentional relationships only.
