# 0017 — Card Workshop and named, draft-preserving authoring

## Context and alternatives

The maintainer approved authoring refinement together with an original example
workshop after ADR0016. Stable exact references work, but routine editing still
requires copying opaque IDs/revisions, selecting value kinds manually, and typing
source mappings. Improve the existing native workflow using concrete examples.
Engine integration, simulation and player-instance execution remain the final
product steps. This milestone is design authoring, not gameplay execution.

A dedicated Plan agent is unavailable; a read-only architectural assessment used
the available exploration agent before this record. Arbitrary frontend frameworks,
implicit latest-head retargeting, name-inferred mappings, mandatory global card
classes, server-side draft storage and automatic workshop creation were considered
and rejected. Reuse the current typed model, revision services and native forms.

## Decision

### Search and selection

- Add purpose-specific bounded card/field search APIs with public summaries. Apply
  name, exact label, technical origin/kind and archive filters in SQL before stable
  pagination. Display at most 25 candidates, fetching one extra for `has_more`.
  Names use literal substring matching (SQLite ASCII case folding); wildcard-like
  input is literal, not SQL syntax. Queries are parameterized and validated.
- A catalog is shared or one world. Assignment fields are same-scope. Reference
  candidates are shared or the edited card's world. Creation sources are shared
  definitions of the appropriate origin. Adoption chooses exact revisions of the
  already pinned source identity. Local mapping candidates explicitly source the
  chosen shared field identity; matching names never establish a mapping.
- Results identify readable names, scope, revision and archival status. Choosing a
  result materializes its exact pin immediately. Retain selected historical or
  archived pins visibly outside candidate pages; never replace them with a head.
  Existing transaction-local selection and retention validation remains authoritative.
- Choice options are selected by name from the exact field definition, paginated
  at 25, retaining stable IDs and off-page selection. No new 25-option game cap.

### Draft-safe editor transitions

- Use existing ID-bound CSRF-protected editor POSTs for non-saving selector search,
  paging, choosing and loading. These transitions render the editor without writes
  or redirects. GET remains the initial editor/catalog/read interface.
- Keep one raw operation draft, selector navigation state and resolved exact context
  distinct from the final mutation. Search/page must not parse or erase malformed
  numbers, text, reason, state, selected pins or the starting revision. Do not reload
  saved defaults over unsaved drafts. Permission failures preserve submitted input.
- Existing expected-revision-first conflict handling remains explicit. Changing an
  exact field/revision must not coerce the value. Loading saved text over a dirty
  draft requires saving or explicit discard-and-load. Loaded markers include exact
  field and owning revision, not merely a field identity.
- Known-value controls derive from the checked exact field kind: numeric input with
  constraints, one bounded textarea, explicit boolean, named choice, or card picker.
  Missing/Unknown/NA are explicit. Known empty text/false/zero remain distinct.
  Advanced exact-coordinate controls may remain as a fallback, not a required path.
- The 32 KiB URL-encoded body bound and one-loaded-prose rule remain. Do not repeat
  candidate descriptions/prose in hidden fields. Keep compact selected pins once.
  Selector/mapping additions must not create an unusable oversized next form: reject
  the addition with the prior draft and pending input retained, never truncate.
- Named source-copy mapping edits build one compact complete draft (IDs/pins only)
  across pages. Search/page does not discard pending or off-page mappings. Preview
  and confirmation bind the whole mapping set, exact source/proposal, removals and
  unmapped assignments. The existing locked equality check still protects commit.
  Stateless mapping size is a transport limit, not a new cardinality game rule.

### Browsing and readable cards

- Add native search/filter forms to card/field catalogs; preserve filters in paging.
  Use explicit same-world/shared scopes, stable ordering and allowlisted queries.
- Lead the card reading view with name, description, labels, inert rules text and
  named ordered properties. Render choices using their embedded exact option labels,
  references and sources using checked exact names and exact-history links. Keep
  qualified identity, origin, revision, archive state and history available as
  secondary context. Resolve only displayed references, deduplicated per request.
- Historical views use exact names/values rather than current-head overlays. These
  presentation refinements do not introduce another metadata authority, visual
  template system, global Equipment/Ability/Item classes or mechanics.

### Original, opt-in workshop

- Add a separate trusted operator use case/command:
  `admin seed-card-workshop --owner-user-id <immutable-id>`. Require an existing
  enabled editor/administrator owner. No account creation/promotion, implicit owner
  selection, world transfer, startup/migration seed or inclusion in old seeds.
- Original illustrative world **Card Workshop** has six local cards: **Mossrunner**
  (native creature Variant), **Ember** (native Type), **Steady Ascent** (native Move),
  **Trail Kit**, **Focused Effort**, and **Field Wrap** (standalone cards). A shared
  Trail Kit example and shared Roll Bonus field support explicit local source
  adoption/copy; these are original design examples, not approved final game lore.
- Six local reusable fields illustrate every kind: Roll Bonus (integer Number,
  shared source), Target Number (integer Number), Usage Notes (Text), Optional
  Marker (Boolean), Use Context (Choice with stable option IDs), Associated Card
  (CardReference). Shared Roll Bonus adds one shared definition. Expected batch:
  one world, seven cards and seven field definitions = 15 authored objects; world
  content/access plus each definition creation share 16 initial audit events.
- Tool +3, ability +5, a roll of 10 and target 15 appear as authored values/prose:
  `10 + 3 + 5 = 18`. No arithmetic, dice, resource spending, equipment effects or
  success resolution is executed. Type labels describe possible attachment and
  resource uses under this example world's conventions. Include explicit false,
  Unknown/Missing/NA and exact relationships without inventing mandatory fields.
- One `BEGIN IMMEDIATE` transaction validates owner and all deterministic creation
  baselines, creates/stages all roots/children/components, seals world membership
  and access and commits once. Reuse native writers and same-event card seals;
  initial card components belong to revision 1, not artificial follow-up edits.
- A trusted CLI import is attributed to the existing explicit system actor
  `bundled-fixture-loader`, with Workshop-specific reason/provenance. An owner ID
  does not prove that person initiated the command. Ordinary browser writes remain
  session-attributed; no browser-editable actor/source-of-permission fields.
- Reruns verify immutable creation baselines and original audit provenance, not
  edited heads; skip the whole complete batch without writes. Partial batches,
  conflicting scoped slugs/IDs, wrong owner, inconsistent baselines or final seal
  failure reject/roll back the invocation. Preserve later edits, archive state,
  pins, collaboration and account/session data. Old labs remain 65 objects/69
  initial events with their existing creation baselines and seed scopes intact.

## Consequences and verification

Prefer no schema/package-version change or new external dependency. Existing
inspection/where-used and schema-2 packages include workshop data through their
normal exact-closure contracts; v1 compatibility remains unchanged.

Test filter-before-page and literal search, eligibility/scope/archived pins, exact
old revision selection, >25 choices/fields/mappings, malformed/dirty/raw drafts
through selector transitions and 403/conflicts, encoded Unicode body bounds,
same-page single prose, changed-proposal confirmation, partial/collision/wrong
owner/late-failure seed rollback, edited-baseline reruns, system attribution and
unchanged old seeds. Keyboard workflows at desktop/320px must create/edit/select
by names, inspect history/usage and export/read the workshop offline. Run fmt,
strict Clippy, workspace and headless browser checks; stop test servers afterward.
