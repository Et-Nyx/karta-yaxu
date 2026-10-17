# Card definition foundation

**Delivered.** This document describes
[ADR0016](../decisions/0016-card-definition-foundation.md).

Karta Yaxu means **Monster Cards** in Globasa: a creature collector with
deck-building. Creatures, Moves, Types, Abilities and Items can be cards. Each
world interprets their roles: a Type might be an attachment, a resource, or
something else. These are design **definitions**, not eventual player-owned
instances with changing state. **Engine integration and the battle simulator are
the absolute final steps**, after further content/world authoring and fine-tuning.

## Identity, revisions and authority

- Concrete shared Forms and world Variants, shared/local Types and Moves are cards
  at their existing identity. Species is a grouping, not another creature card.
- A qualified identity is `(namespace, world_id, id)`: `form`, `variant`, `type`,
  `move` or `standalone`; `world_id=null` means shared. Forms are shared, Variants
  local. Equal opaque IDs across namespaces/scopes are not equal identities.
  Technical origins do not prescribe gameplay classes.
- Standalone shared/local cards support, for example, Item or Ability design
  without mandatory global Equipment/Consumable classes or a specialized table.
- An exact card pin adds positive `version`. For native cards this is the **same
  owning definition revision**, not an independent wrapper ID or card counter.
  A custom-only change advances the native revision, so stale native editors
  conflict too. Native typed metadata, archive state and source remain the single
  authority; the card view cannot maintain a competing name/description/source.
- Every owning writer carries the component forward: creation (including default
  Forms), metadata, typing, Stats, adoption, reconciliation and fixture setup.
  A staged custom proposal or carried component is sealed after the native version
  is inserted, using that version's **same audit event**, in one transaction.
  A late seal/audit failure rolls back the whole write; no second event is invented.
  No-op saves do not add revisions.
- Stats and Move attributes remain separate exact typed definitions; matching
  names do not merge them into custom fields. Learnsets, presets, rosters, charts
  and rulesets keep independent revisions. A card pin invents no pins to their
  independent current heads.

## Reusable fields and inert prose

Shared/local field definitions have immutable IDs, generated scoped slugs, metadata,
revision/archive state and optional exact shared sources. Cards select an ordered
list of distinct field identities at **exact same-scope revisions**, embedding the
verified definition in each immutable component. They are reusable fields, not
required classes, layouts or executable rules.

| Kind | Authored value and constraints |
| --- | --- |
| Number | Exact signed decimal: normalized i64 coefficient, scale 0–6; optional inclusive minimum/maximum and whole-only constraint. No rounding or exponent notation. |
| Text | Plain text with LF paragraphs; selected maximum 0–2000 Unicode characters, no other control characters. Empty known text is valid. |
| Boolean | Explicit `true` or `false`. |
| Choice | One stable opaque option ID from that exact definition revision; ordered options have distinct IDs and normalized labels of 1–120 characters. Renaming a label does not change its ID. |
| CardReference | Exact qualified card identity and revision. Shared cards reference shared cards; local cards may reference shared or same-world cards, never another world's content. |

Every kind also permits **Missing**, **Unknown** and **NotApplicable**. A missing
value on a selected field differs from an absent assignment; an unmodeled field
collection differs from defined-empty. Known empty text, false and zero are not
Missing/Unknown/NA. Definition changes never reinterpret earlier values: choosing
a new revision requires compatible explicit values. Retained exact archived pins
permit unrelated edits; new/retargeted selections require active definitions/cards.
References can cycle; closure uses bounded visited traversal, not recursive reads.

`rules_text` is optional inert plain text, separate from description, at most 2000
Unicode characters with LF paragraphs. Labels are optional ordered distinct trimmed
strings, 1–120 characters each without controls. Labels such as Equipment and fields
such as Cost impose **no mechanics**. No effects, spending or formulas execute.

## Native browser workflow and history

Public GET/HEAD catalogs are `/cards`, `/card-fields` and their
`/worlds/{world}/cards`, `/worlds/{world}/card-fields` counterparts. Native card
links qualify the origin: `/cards/form/{id}`, `/cards/type/{id}`,
`/cards/move/{id}` and world-local `/cards/variant/{id}`, `/cards/type/{id}`,
`/cards/move/{id}` under the world prefix. Standalone cards use
`/cards/standalone/{slug-or-id}`; fields use `/card-fields/{slug-or-id}`.
World GET/HEAD addresses accept readable world slugs. Standalone/field aliases use
generated scoped slugs, with `~` escaping reserved/ID-shaped slugs. Form slugs
require their species-qualified native route. Immutable origin-qualified IDs are
the durable coordinates, and mutation forms remain ID-bound across rename/reuse.

Catalogs support `/new`; details support `/edit`, `/history` and
`/history/{revision}`. Lists, field assignments, labels, choice options and history
pages show at most **25** entries; history uses a descending exclusive `before`
revision cursor. Exact history reconstructs the owning definition and component,
with actor display/system name, action, UTC time and reason. Missing attribution
or corrupt required seals fail closed rather than silently hiding revisions.
Native cards link **Specialized metadata editor** and **Native full history**;
specialized fields stay in their native editors.

Editors use operation-scoped native controls: load a single assignment, option,
label or prose operation before editing. They retain raw drafts on invalid input,
authorization/storage failures and conflicts, with explicit reviewed resubmission.
Forms are CSRF protected and bounded to **32 KiB**; other long text is not repeated
in hidden fields. Standalone-card and reusable-field archive changes require a
preview and explicit confirmation. Shared editing requires an administrator;
world editing requires an eligible enabled owner/collaborator, checked with the
session under `BEGIN IMMEDIATE`, without a global administrator bypass.

Local field source adoption previews old/local/new/proposed definitions and keeps
local metadata/kind by default. Native specialized adoption preserves components;
standalone source adoption also keeps local data. Copying fields from the already
pinned shared card is a separate previewed, confirmed **whole-collection
replacement**, showing removed local and unmapped source assignments. Mappings
are explicit and injective; local fields must explicitly source the shared identity.
Names never infer mappings. The first mapping has native controls; additional
compact rows are `shared_field_id local_field_id revision`, one per line. Multiple
rows participate in the same reviewed proposal, preserving source assignment order.
Copying changes only the field collection, not rules text, labels or source pin.

## Migration 0009 and fixture compatibility

`0009_cards.sql` is a compatibility boundary, not a historical rewrite. It records
each existing native identity's maximum revision in immutable
`card_legacy_boundaries`. Pre-boundary revisions have unmodeled custom data
(`legacy_unmodeled=true`), not fabricated authored empty components. Existing
snapshot bytes, object heads, IDs and audit events remain unchanged. Migration
bookkeeping invents no actors/events. New native creations and later owning
revisions require complete sealed components, even when their data is unmodeled;
missing post-boundary components fail closed. No creature v4 snapshot is introduced.

All **nine migrations** are now required. Use the existing deliberate backup/stop/
`cargo run -p web-app --bin admin -- migrate`/restart workflow inside `nix develop`.
Startup and seed commands do not migrate. No production database was migrated by
this documentation task. Migration seeds no field definitions or standalone cards.
Fresh explicit fixture creation now seals native components, but authored fixture
totals remain **65 objects / 69 initial audit events**; component/child/boundary
tables add technical rows, not independently counted authored objects. Reruns
preserve old heads/history and authored changes.

## Inspection and export

Public read-only `/worlds/{world}/inspection/cards` lists captured components and
exact reusable definitions. `/worlds/{world}/inspection/cards/where-used` joins
native and custom usage: native cards/components are one identity, field assignments
and exact references retain occurrence sites, and mixed native/custom root paths
are traversed together. Native membership/ownership is not usage; identity-only
observations cannot invent exact historical paths. Use the native GET links for
card/field target, shared/local scope, optional namespace, exact version or `all`,
and direct/path view. “All” is all captured versions, not all history.

This uses the **same world-first read capture** as typed inspection, including
archived/unused local roots and explicit source/reference closure. The transaction
ends before analysis/rendering. The common fingerprint now uses the
`karta-world-inspection-sha256-v2` marker and includes card/field data, roots and
observed heads. Each request recaptures; changed input announces refresh and resets
page 1. Only complete captures have fingerprints. Partial capture and bounded
usage truncation remain explicit: no global-unused/safe-delete or readiness claim.
Existing privacy, admission and cooperative capture limits apply; pages show 25,
paths/results are bounded to 256, depth 32 and traversal work 65,536.

New browser exports use [CardPackage schema 2](design-package-v2.md), combining
typed schema-1 projection with complete card/field closure. The frozen
[v1 contract](design-package-v1.md) and reader remain unchanged. Offline
`tools validate-package [path|-]` dispatches schemas 1/2. Neither inspection nor
export executes the authored prose or establishes gameplay readiness.

## Implementation and verification inventory

- `crates/game-data/src/cards.rs`, `card_fields.rs`: public records and invariants;
  `card_inspection.rs`: pure mixed bounded usage; `card_package.rs` and
  `card_package/validation.rs`: offline schema-2 contract.
- `crates/web-app/src/application/cards.rs`: authorization, expected-revision-first
  edits, source-copy/adoption proposals and public history DTOs.
- `crates/web-app/src/persistence/cards.rs`: checked exact reads, `stage`,
  `seal_native`, dependency verification, attributed history. Native writers in
  `authoring.rs`, `types.rs` and `moves.rs` invoke the same-revision bridge.
- `src/persistence/inspection.rs` and `src/application/inspection.rs`: combined
  capture/fingerprint; `src/application/export.rs`: schema-2 preview/confirmation.
- `src/web/cards.rs`, `src/web/card_inspection.rs` and corresponding templates:
  operation forms, public exact history and read-only card reports.

Source inventory: five workflows in `tests/browser/test_cards.py`, inherited by
`DesktopCardTests` and `NarrowCardTests`, add **10** cases to the prior 84, for
**94 browser tests**. They cover loaded Unicode prose/history, five field kinds
and raw errors, explicit multi-row copying/adoption, native metadata carry-forward,
and archive/revoked drafts. The world source-copy workflow also traverses public
card inspection, exact field usage and representative root paths by keyboard.
Five web tests and six pure usage tests cover further inspection contracts.
Formatting, strict workspace Clippy, **438 Rust tests** and the complete
**94-case browser suite** passed, with targeted paragraph-transport verification
afterward at both widths. The two ordinary-run ignored Rust tests are
explicit browser-owned fixture provisioners. Human visual/zoom/screen-reader
assessment is not established by headless keyboard and reflow checks.

Player instances, decks/hands/discards, deck-building execution, attachment/resource
execution, dice, formulas, Lua and combat remain future work. No engine integration
or simulator is implemented; both remain the absolute final product steps.
