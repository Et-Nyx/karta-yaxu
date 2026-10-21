# Reusable records and card-content relationships

[ADR0018](../decisions/0018-card-records-and-content-relationships.md) adds flat,
reusable data layouts and named collections to the same-identity card foundation.
Items, Abilities, Resources, Natures/traits and other concepts remain world-defined
cards, not mandatory exclusive global classes. Conditions, timing, adjustment
interpretations and effects are inert prose/data. Engine integration and simulation
remain the absolute final steps.

## The three authoring levels

1. **Record layout:** ordered, stable columns defining a row's shape. Each column
   has a stable layout-scoped ID and readable label. A scalar column embeds an
   exact same-scope reusable field; a reference column declares a nonempty distinct
   allowlist of Card, Stat and Field target kinds. Two columns may use the same
   scalar field without sharing column identity. Layouts are flat, not nested.
2. **Collection definition:** a named, independently versioned definition pinning
   an exact same-scope layout. Resource Costs and Produces may use the same Resource
   Amount layout without becoming the same collection or receiving built-in behavior.
3. **Card records:** ordered assignments to exact collection definitions, with an
   explicit state or ordered entries. Entries have stable IDs scoped to that card
   and collection. Cells identify stable column IDs, not column labels or positions.

Layouts and collection definitions have shared/world scope, immutable IDs, generated
scoped slugs, normalized metadata, archive state and optional exact shared sources.
Choices use the exact scalar field's stable option IDs. References preserve exact
kind, scope, identity and revision. Shared rows reference shared data; local rows
may reference shared or same-world Card/Stat/Field definitions, never another world
or private user/access records. Existing Stats/fields are not converted to wrapper
cards. Repeated targets remain separate row/cell occurrences.

## State distinctions

- The card's collection inventory is unmodeled (`null`) or defined (possibly empty).
- A collection is absent from that inventory, explicitly Missing, Unknown,
  NotApplicable, or Known entries (possibly empty).
- A declared column's cell may be absent, or explicitly Missing/Unknown/NA, or a
  typed Known value. An undeclared column or duplicate cell is structurally invalid.
- Known empty text, false and zero remain values. No default fill, deduplication,
  numeric coercion or first-option selection reinterprets incomplete drafts.

Intrinsic scalar constraints and allowed reference kinds are structural requirements.
Optional cells need not be filled. An authored quantity is not a balance or cost
execution rule; an adjustment does not calculate or mutate the referenced Stat.

## Revision and storage contracts

`CardRelationRecord` is a companion at the existing qualified `CardPin` revision,
not a second card ID or revision counter. Editing rows/cells advances the owning
card revision and preserves scalar content; all existing scalar/native writers
carry records forward. Native/scalar/record seals share the same audit event in one
transaction. Stale native drafts conflict after a record edit. No-ops do not invent
revisions. Shared editing remains administrator-only; world editing requires an
eligible enabled owner/collaborator, checked under `BEGIN IMMEDIATE`, with no global
administrator bypass. Browser actor fields are not authoritative.

Migration **0010_records.sql** adds typed heads/versions/columns/reference allowlists
and card collection/entry/cell tables, with immutable snapshots and seals. It records
immutable legacy boundaries for existing Forms, Variants, Types, Moves and standalone
cards. Pre-boundary record components are unmodeled; existing rows, IDs, heads,
snapshots and audit events are not rewritten. Missing post-boundary companions fail
closed. Current readers/writers require all ten migrations; no missing-table fallback
is used. Back up before deliberately applying `admin migrate`; startup never migrates.
The existing local database was not migrated by this implementation.

## Native browser workflow

Public shared catalogs: `/record-layouts`, `/record-collections`; world catalogs add
`/worlds/{world}/`. Details support `/edit`, `/history`, `/history/{revision}` and
creation uses `/new`. Named catalog searches filter before 25-entry paging. Public
history exposes actor display/system name, action, UTC time and reason, not private
audit/account data. Definition ID and slug GET aliases are scoped; mutation actions
stay ID-bound across rename/reuse.

Card records use `/cards/{namespace}/{card}/records` with the optional world prefix.
Card detail links **Edit record collections** for the current definition, and **Record
collections** links its exact revision. Record detail has separate inventory,
collection, row and cell editor operations:

1. Define an inventory or add an exact named collection definition.
2. Explicitly choose its state. Known entries may be empty; replacing a nonempty
   collection with a non-entry state requires explicit row removal first.
3. Add, remove or move an entry occurrence using its stable identity.
4. In **Edit cell**, choose an attached exact collection, a named column and entry,
   then **Load selected cell**. Choose Absent/Missing/Unknown/NA/Known and edit the
   control dictated by the exact column. Reference targets and choice options have
   named bounded selectors; search scope is shared or this world.
5. Save. Other entries, columns, collections and off-page saved values survive.

One prose value is loaded per operation. Search/choose do not overwrite dirty text.
Saving or **Discard text and load selected cell** is required before switching it.
The loaded marker binds collection, entry, column and owning revision; edited exact
definitions require reconciliation rather than reinterpretation. Raw numeric/text/
selection/reason drafts survive validation, revoked permissions and conflicts; an
explicit reviewed-revision submission is required to rebase. Duplicate/unknown form
and query keys are rejected. ID-bound CSRF forms have the existing 32 KiB encoded
limit; mapping/selection additions receive conservative size checks without truncation.
Text uses LF paragraphs, server Unicode limits and no HTML UTF-16 `maxlength`.

Read views bound rows/columns to 25. Card history supports independent `page` and
`column_page` navigation so large exact records can be inspected without editing.
Changing an exact collection pin is an explicit reconciliation, not a row-edit side effect.

## Adoption, reconciliation and source copying

Layout and collection source adoption keep local metadata, columns and exact layout
by default; only an explicitly selected newer source pin changes. Existing consumers
retain their exact definitions. Archival and adoption require preview/confirmation.

Reconcile a card collection to another revision of the same collection identity by
explicitly mapping old to new stable columns. Source copying maps a shared card's
exact collection identities to local collection definitions, then shared columns to
local columns. Local collections and layouts must explicitly source the corresponding
shared identities. Names never infer mappings, targets never silently become local,
and missing columns never receive fabricated values.

Draft mappings retain the complete compact set across pages and named searches.
Preview displays old/source/proposed values and paginated removed/unmapped collection
and cell evidence. Confirmation binds exact inputs, complete mappings and proposal,
then recomputes under the writer lock. Copies preserve source order and stable entry
IDs in the destination scope. Unrelated exact archived pins remain retainable; new
or retargeted references require active definitions/targets. Reverse Granted by and
Used by are derived through inspection, not independently edited mirror lists.

## Capture, usage and export

The existing world-first read transaction now captures all current local layouts and
collection definitions, including archived/unused, every reached same-revision record
component, and exact sources, embedded scalar definitions and row targets. Preflight
accounts snapshot bytes, child occurrences and verification companions before checked
reads. Card/record reference cycles use bounded visited work queues. End the transaction
before fingerprinting, analysis/rendering. Privacy, progress-handler cleanup, cancellation
lease disposal and two-job admission follow the existing inspection/export contracts.

`/worlds/{world}/inspection/records` and its `/where-used` suffix share card inspection's
captured evidence and mixed graph. Targets include card, field, stat, layout, collection
and legacy typed data. Repeated rows/cells have explicit occurrence sites. Membership/
ownership is not use; all-version means all captured versions, not all history. Partial
coverage has gaps and no fingerprint/Unused claim. Complete capture fingerprint **v3**
covers record roots, exact definitions/components and observed heads, separately from
package checksums. Legacy usage routes direct record-aware worlds to the mixed view;
old typed-only analysis does not claim complete usage coverage for record data.

New downloads use [RecordPackage schema 3](design-package-v3.md), including a schema-2
card projection plus complete record closure. Old v1/v2 readers/files stay supported.
Offline `tools validate-package [path|-]` dispatches 1/2/3 with the same 32 MiB bound.
Counts include each evidence component, not distinct gameplay identities. Structural
acceptance permits well-formed drafts and is not engine/gameplay readiness.

## Original opt-in examples

```bash
cargo run -p web-app --bin admin -- list-users
cargo run -p web-app --bin admin -- seed-card-relations --owner-user-id <immutable-user-id>
```

The separate **Card Relations Workshop** (`/worlds/card-relations-workshop`) requires
an existing enabled editor/admin and creates no accounts. One transaction creates
**28 objects / 29 initial audit events**: one world, ten cards, six fields, four
layouts, six collection definitions and one Stat. Audit actor is
`bundled-fixture-loader`, reason `Original Card Relations Workshop v1 (ADR0018)`.
No old seed, startup or migration calls it. Complete reruns validate original immutable
creation evidence, skip all 28, and preserve later edits/grants/sessions. Wrong owners,
partial/colliding/corrupt batches and late failures reject/roll back without repair.

- Fernstrider (creature Variant) has Patient and Restless trait options.
- Ember (Type) also illustrates resource/attachment design intent; Focus is a Resource.
- Second Wind (Ability) has Resource Costs: Ember x2 and Focus x1.
- Field Satchel (Item) grants Second Wind.
- Patient (Nature/trait) has personality prose and exact Resolve Stat / Quantity field
  influence targets; adjustment interpretation is an inert authored choice.
- Produces reuses Resource Amount independently of Resource Costs, preserving repeated
  Focus occurrences under different descriptive conditions and explicit incomplete states.
- Shared Ember, Focus and Second Wind, two shared fields, a shared Resource Amount
  layout and Resource Costs collection demonstrate explicit local source context.

The old labs retain 65 objects/69 events and Card Workshop retains 15/16. This is original
illustrative content, not final lore, inventory, executable costs/effects or simulation.
Runtime instances/balances/consumption, formula/condition execution and engine integration
remain later work, with engine and simulator last.

Implementation: `game-data::relations`, `record_inspection`, `record_package`;
`web-app` application/persistence `relations`, record search, `web/relations` native
editors, the combined inspection capture, and `application/relation_workshop.rs`.

## Verification

Formatting, strict workspace Clippy, **501 Rust tests** and the complete **114-case
keyboard browser suite** passed. Two ordinary-run ignored Rust tests provision
browser-owned fixtures. New coverage includes same-event rollback/carry-forward,
populated migration preservation, allowed reference kinds, exact typed closure,
repeated rows, state distinctions, >25 rows/columns, corruption/partial capture,
no-write reads, confirmation freshness, source mapping and example rerun isolation.

Five new record workflows run at desktop and 320px: typed amount editing and exact
history; dirty Unicode conditions and explicit discard; Stat/field targets and mixed
usage; native layout/collection/entry creation; and explicit source-column mappings
with independent checksum verification of a schema-3 download. Tests use temporary
databases, synthetic accounts and test-owned downloads/servers. The existing local
database was not migrated or seeded. Single-job builds disabled debug symbols on
the constrained host. Human visual, screen-reader and actual zoom assessment were
not performed; 320px reflow and document keyboard checks are separate evidence.
