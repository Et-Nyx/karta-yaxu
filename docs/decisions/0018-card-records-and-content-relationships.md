# 0018 — Reusable records and world-defined content relationships

## Context and approvals

After ADR0017 the maintainer approved Items, Abilities, Natures/traits, Resources
and other content through reusable records, exact references and inert prose.
Up-front choices: reusable named collection definitions separate from row layouts;
absent cells distinct from Missing/Unknown/NotApplicable; reference columns declare
one or more allowed public target kinds; and a separate opt-in example world.
Natures are world-defined traits, not mandatory stat-modifier classes. Engine and
simulator integration remain the absolute final product steps.

The dedicated Plan agent is unavailable. This record follows read-only review of
the current field/card/package/capture contracts and explicit maintainer decisions.
Fixed global gameplay classes, nested record languages, automatic name mapping,
live arbitrary JSON entities, runtime balances and executable effects were rejected.

## Decision

### Definition and value model

- Add independently versioned shared/world **record layouts**, with immutable IDs,
  scoped generated slugs, normalized metadata/archive state and optional exact
  shared sources. Each layout owns ordered columns with stable scoped column IDs,
  readable labels and a declared kind. Existing scalar field definitions remain
  unchanged; columns may reuse them at exact same-scope revisions, including the
  same field in differently named columns. Column identity is not its label/field ID.
- A column is either an exact reusable scalar Field definition, or a reference
  declaring a nonempty distinct allowlist of Card, Stat and reusable Field target
  kinds. Known references always retain their explicit kind, scope, ID and revision.
  Shared data may reference shared definitions; local rows may reference shared or
  same-world definitions, never foreign-world/private content. Existing Stat and
  field identities are not converted into wrapper cards or merged by names.
- Add independently versioned shared/world **collection definitions**, also with
  immutable IDs, metadata/archive state and optional exact shared sources. Each
  pins an exact same-scope layout. Costs and Produces can use the same Resource
  Amount layout without sharing collection identity or acquiring implicit behavior.
- A card selects distinct ordered exact collection definitions and authors their
  values. A collection value is Missing, Unknown, NotApplicable or an ordered list
  of known entries. Known empty differs from all those states. The card's collection
  inventory can itself be unmodeled or defined-empty; an absent collection differs
  from an explicitly modeled state.
- Each entry has a stable ID scoped to its owning card/collection. Repeated targets
  are allowed and never deduplicated. Cells identify stable layout column IDs;
  absent cells are allowed, separately from explicit Missing/Unknown/NA. Known
  numbers/text/booleans/choices use the exact scalar field constraints; a scalar
  CardReference column accepts Card targets only. Reference columns enforce their
  declared allowlist. Reject duplicate/undeclared cells, wrong value kinds, invalid
  numbers/options and broken references. Preserve empty text, false and zero.
- Initial layouts are flat, with no nested records, required-cell policy, arithmetic,
  triggers or condition execution. Timing, applicability and effects are prose.
  No name, label, resource amount or adjustment choice has built-in mechanics.

### Ownership, history and adoption

- Collections are a component of the existing qualified card and owning revision,
  not a second card ID/revision stream. Entry/cell/collection edits advance that
  same revision. All existing native/scalar writers carry records forward, sharing
  the owning audit event atomically. Readers verify complete same-event evidence.
  Expected revision and permission checks precede submitted dependency validation;
  failures roll back everything and no-ops create no attributed revision.
- Add additive migration 0010 with typed layout/column, collection-definition,
  card-component, assignment, entry and cell tables. JSON is for immutable snapshots
  only. Record explicit legacy boundaries for already existing card revisions;
  their collections are unmodeled. Preserve old rows, IDs, bytes, heads, actors and
  audit events. Missing new record seals fail closed; no silent missing-table skip.
- Public exact histories include author/action/UTC/reason. Shared editing remains
  administrator-only; world editing requires eligible owner/collaborator, checked
  in the writer transaction, without global administrator bypass.
- Layout/collection source adoption defaults to keep-local. Consumer retargeting
  and source copying use explicit injective collection/column mappings, previewed
  removals/unmapped cells and confirmation bound to exact inputs/proposals. No name
  inference, number coercion, automatic filling, target retargeting or row dedup.
  Preserve stable entry occurrence IDs across reviewed copies/reconciliation in
  their destination scope. Existing exact archived dependencies permit unrelated
  edits; new/retargeted selections require active definitions/targets.
- One authority owns a relationship: reverse Granted by/Used by views are derived
  usage, not separately editable mirrored rows. Labels remain organizational;
  Items/Abilities/Resources/Natures do not become universal exclusive namespaces.

### Native authoring and capture

- Public scoped layout/collection catalogs, details and exact histories, with named
  selectors for existing scalar fields and exact Card/Stat/Field targets. Lists
  paginate at 25 after typed SQL filtering. Layouts/collections show source context.
- On cards, edit one collection/entry/cell operation at a time, including adding,
  removing and reordering rows, with raw-error/conflict/revoked drafts retained.
  One loaded prose per operation, explicit save/discard before switching dirty text,
  UTF-16-safe server character validation, ID-bound CSRF POSTs and 32 KiB body
  bounds follow ADR0017. Off-page saved values and repeated occurrences survive.
- Extend world-first single read-snapshot inspection with all current local layouts
  and collection definitions (including archived/unused), card record components
  and exact dependencies. Include repeated row/cell sites and mixed bounded shortest
  usage paths. Public DTOs only; no owner/access/users/raw audit/snapshot data in
  reports/hashes. Fingerprint contract advances separately to include new evidence.
- Preflight snapshots/children, caches, cooperative SQLite progress interruption,
  cleanup, close-on-drop cancellation, two-job admission and off-handler CPU work
  remain. Combine accounting across old and new evidence. Partial captures have
  explicit gaps, no fingerprint and no global-unused or readiness claim.

### Frozen packages and compatibility

- New complete exports use schema 3, with a frozen schema-2 card projection plus
  exact layout/collection/record closure. The projection may have dependencies
  reached only through records, so is not necessarily independently closed v2.
  Do not promote archived/old dependency pins into active catalog roots.
- Preserve existing v1/v2 readers, wire tokens/order and interpretation unchanged.
  Shared structural validators may take explicit external seeds internally, while
  historical entry points supply none. The v3 reader independently derives and
  proves the combined root closure, exact embedded equality, allowed targets,
  scopes, stable cells/options and occurrence preservation; reject extras as well
  as missing/duplicate/invalid evidence, even with recomputed checksums.
- Deterministic canonical keyed vectors, authored array order, explicit states and
  a package-specific SHA-256 domain remain. Reuse workspace crypto libraries only.
  At most 32 MiB input/output and existing combined capture/work budgets. Preview
  confirmation binds v3 content digest; final bytes pass the actual shared reader
  and permission is rechecked before private/no-store release. CLI validation
  dispatches formats 1/2/3 offline. Well-formed diagnostic drafts remain saveable/
  exportable, without claiming effect, engine or combat readiness.

### Original relationship workshop

- Add a separate opt-in original world, through trusted
  `admin seed-card-relations --owner-user-id <immutable-id>`. Require an existing
  enabled editor/admin; create no accounts and do not modify old workshop cards or
  old seed scopes/baselines. One transaction, explicit system attribution, exact
  immutable creation baseline verification, partial/collision/wrong-owner rejection,
  rollback on late failures and edited rerun preservation follow ADR0017.
- Demonstrate an Ability costing Ember x2 and Focus x1, an Item granting it, a
  creature with several trait options, a Nature with personality prose and optional
  exact Stat/Field influences, and a Type used in resource/attachment design notes.
  Shared/local layout/collection source context demonstrates explicit adoption.
  Names/labels/values are original illustrative data, not final lore or executable
  rules. Derive and test exact batch counts from the delivered manifest.

## Verification and boundaries

Test populated upgrades preserving all previous data/bytes; missing/forged seals;
native writer carry-forward; atomic attribution/rollback/no-ops; role/scope/archived
retention; ordered/repeated entries and absent/Missing/Unknown/NA/empty distinctions;
exact field/options/target kinds; explicit source and reconciliation mappings;
raw named editor paging/dirty prose/body bounds; public privacy, cooperative limits
and freshness; exact mixed usage; old v1/v2 and v3 acceptance/tampering/offline
reproducibility; workshop idempotency and partial/collision/late-failure rejection.
Run fmt, Clippy with warnings denied, workspace and keyboard desktop/320px suites
in Nix. Use single-job builds with debug symbols disabled on this constrained host.
Verification uses temporary databases, not the existing local database.

Player instances, balances, charges, inventory, deck execution, attachment/resource
execution, dice/formulas, executable conditions/effects, engine integration and
simulation remain future work, with engine/simulation last.
