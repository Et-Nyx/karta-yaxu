# Product roadmap

This preserves the maintainer's milestone discussion. It is a set of directions,
not a promise that all mechanics or infrastructure choices are settled.

## Foundation delivered

Shared species/forms, independently owned world variants, scoped collaboration,
explicit source adoption, immutable attributed history, exact typing, independent
effectiveness charts, independent Stats, ordered world stat rosters and signed
decimal base values. Independent Moves, learning/presets and read-only world
inspection and frozen design-package export now extend this authoring foundation.
Pokémon is a temporary reference stress fixture.

The Card Definition Foundation is also delivered: same-identity native cards,
standalone cards, reusable typed fields,
public card inspection and card-aware package schema 2. Karta Yaxu means **Monster
Cards** in Globasa, a creature collector with deck-building; definitions remain
distinct from eventual player instances and executable decks.

## Delivered: Moves, Learnsets and Movesets

Designer-defined move attributes belong to move definitions. Each world defines
its ruleset environment; that ruleset interprets attributes and their relationships
with Stats and eventual derived values. Objects support Unknown values. Shared and
local definitions remain independent and consumers pin exact revisions. Learnsets
describe availability/acquisition; named movesets are configured design presets,
not runtime individual creatures. Establish configuration and authoring validation
before executable formulas. See decision 0012.

The authoring slice is implemented: independent move attributes and definitions,
world ruleset configuration, learning records and named presets with exact pins,
separate histories and authoring diagnostics. Executable effects, acquisition
conditions and formula evaluation remain follow-up mechanics work.

## Delivered: World Inspection and Dependency Visibility

The approved authoring-module foundation is implemented. Public read-only
`/worlds/{world}/inspection` and `/worlds/{world}/inspection/where-used` are reached
through the world's **Authoring inspection** link. One ordinary read transaction
anchors the world first, captures all independent current roots (including archived
and unused local objects), then their exact dependency closure. Current ownership/
catalog membership is an observation, not a historical world-revision pin. Shared
sources enter only through explicit dependencies; a source form never implicitly
adds its independent learnset. Older pins and acquisition/selection multiplicity
are preserved.

Typed authoring findings retain saved preset policy 1/2, use current policy 2 for
Move/ruleset diagnostics and optionally compare presets under policy 2 without
saving. Context differences, incomplete inputs, contradictions, not-evaluated
checks and coverage gaps do not establish combat/export readiness. Public-only
complete fingerprints identify fresh captures; navigation recaptures and announces
changes with a page-1 reset. Partial results have gaps and no fingerprint or
“Unused” claim. Scoped where-used covers all captured versions or an exact revision,
with direct sites and bounded representative shortest root paths, not all history
or global safe-delete analysis. Membership is distinct from usage, and learnset
acquisitions are distinct from actual preset selections.

Operational capture, concurrency, evidence/rendering and traversal bounds keep
this foundation bounded without adding game cardinality rules. Capture deadlines
are cooperative, not hard real-time. Existing labs suffice: no schema, dependency
or fixture objects were added; the 65-object/69-audit setup baseline is preserved.
See [ADR0014](../decisions/0014-world-inspection-and-dependency-visibility.md) and
[README](../../README.md#world-inspection-and-where-used) for contracts and limits.

## Delivered: First Frozen Design Package

`karta-design-package` v1 freezes the active world catalog and required exact
dependencies, including older/archived revisions and explicit capture-time bindings
for identity-only owners/default forms. Roots include current unarchived local
definitions/variants/chart, configured roster/ruleset, and learnsets/unarchived
presets of active variants. Membership never imports excluded catalog entries.

Enabled editor/administrator owners and collaborators use authorized native
`/worlds/{world}/export` GET/HEAD entry → CSRF-protected ID-bound POST preview →
reviewed-digest confirmation/download. There is no global administrator bypass.
Confirmation recaptures and permission is rechecked before release; changed inputs
require fresh confirmation. One world-first read transaction supplies complete
public content; no content changes, cached artifacts or export audit schema are added.

The manifest/content envelope has deterministic canonical JSON and a dedicated
SHA-256 content domain, independent of inspection fingerprints. The actual shared
offline Rust reader validates final bytes, full declared closure, exact embedded
equality and supported schema/policies before attachment release. Private data and
generation timestamps are excluded. Well-formed Unknown/incomplete/contradictory
drafts remain exportable: current diagnostics use policy 2, saved presets keep 1/2,
and structural acceptance is not combat readiness. `tools validate-package [path|-]`
validates offline (stdin by default); there is no CLI package creation.

Capture retains inspection's bounded cooperative defaults, plus a 32 MiB package
input/output limit and two export slots through validation/serialization/render.
No migrations or fixture objects are introduced; setup remains 65 objects/69 initial
audit events. See [ADR0015](../decisions/0015-first-frozen-design-package.md),
[the v1 protocol](design-package-v1.md) and
[the workflow](../../README.md#frozen-design-packages).

## Delivered: Card Definition Foundation

The maintainer clarified Karta Yaxu (Monster Cards) as a creature collector with
deck-building. Creatures, Moves, Types, Abilities and Items can be cards; each
world determines their uses. A Type card could be an attachment or resource.
Definitions are distinct from eventual player-owned instances and dynamic state.

Existing Forms/Variants/Types/Moves now are cards at their qualified native identity
and owning revision, with no wrapper IDs; Species stays a grouping. Standalone
cards and reusable exact shared/local Number/Text/Boolean/Choice/CardReference
fields support further content authoring. Native metadata stays authoritative,
every writer carries components forward, and late seals share the owning audit
event atomically. Migration 0009 preserves old bytes/heads/audits and marks legacy
custom data unmodeled; new native creations require complete seals. No fields are
implicitly seeded and fixture totals remain 65 objects/69 initial audit events.

Public Cards/Fields operation forms and exact history, explicit source-copy mappings,
card/field inspection and mixed bounded where-used are implemented. Inspection uses
the common complete-only fingerprint v2. New exports use schema-2 CardPackage with
complete card closure and a schema-1 typed projection; the frozen v1 reader/wire
is unchanged. Formatting, strict Clippy, 438 Rust tests and the complete 94-case
keyboard suite passed. See [ADR0016](../decisions/0016-card-definition-foundation.md),
[the foundation contract](card-foundation.md) and [package v2](design-package-v2.md).
No player-instance/deck/attachment/resource execution, engine integration or
simulation is delivered.

**Engine integration and the battle simulator are the absolute final steps.**
Complete and fine-tune the design database/authoring domains first. Earlier
engine-next recommendations are superseded by this maintainer decision.

## Delivered: Card Workshop authoring refinement

The approved next authoring slice adds named bounded exact selectors, type-driven
Known controls, draft-preserving search/page/load operations, searchable catalogs
and readable card references. A separate opt-in **Card Workshop** supplies original
creature/Type/action/tool/ability/item examples, reusable fields and shared/local
source context. Its illustrative `10 + 3 + 5 = 18` target-15 example is authored
prose, not executed mechanics. No fixed global gameplay classes are introduced.
See [ADR0017](../decisions/0017-card-workshop-and-authoring-refinement.md) and the
[workshop guide](card-workshop.md). Strict Clippy, 462 Rust tests and the complete
104-case keyboard suite passed. No migration, new package format, dependency or
hosting resource was introduced. Workshop initialization is explicit and adds its
own 15-object/16-event baseline, preserving subsequent edits and old seed scopes.

## Pinned follow-up directions

1. **Derived Stats, formulas and modifiers:** explicit inputs, dependency/cycle
   validation, evaluation order, exact rounding/clamping policy and inspectable
   breakdowns. Unknown required inputs produce explained unresolved results.
2. **Further world validation and authoring recovery:** build on delivered inspection,
   scoped where-used and incomplete/context-difference reports; cross-world
   comparisons, explicit archive, attributed revert-as-new-revision and ownership
   recovery remain deferred.
3. **Engine integration and further package workflows:** build on delivered frozen
   design packages/shared-reader validation and owner/collaborator export policy;
   actual engine integration, gameplay acceptance, package migration, signatures,
   retained/atomic publication and selective roots remain deferred.
4. **Evolution, transformations and cosmetic relationships:** explicit world-entry
   targets, branches and typed conditions; persistent progression, reversible
   battle changes and shared-mechanics appearances remain distinct.
5. **Source contexts and imports:** distinguish source game editions/groups/
   generations from Karta revisions; curated bounded caches, provenance, previewed
   updates and explicit adoption. No runtime external browsing/combat dependency.
6. **Items, abilities, natures and resources:** independent definitions and explicit
   assignments, informed by concrete effect/modifier mechanics rather than a
   universal Pokémon schema.
7. **Locations and encounters:** authored region/location/area relationships,
   context-qualified conditions/methods/chances and explicit variant targets;
   playable geometry is a separate model.
8. **Real team access and pilot:** choose authentication, visibility and identity
   migration deliberately. Polycentric remains an independent experiment. Verify
   durable hosting and current costs before deployment.
9. **Deterministic Battle Lab:** small versioned ruleset, CLI simulation and replay
   first, browser PvE next, then server-authoritative turn-based PvP. Pin content,
   rules and RNG; bound execution. Introduce trusted Lua only for concrete needs.

Current direction: card foundation → further card-content and world authoring
domains (including Items/Abilities and explicit relationships) → fine-tuning and
authoring recovery. Engine integration, then deterministic CLI battle/replay,
browser PvE and eventual PvP come last. Detailed intermediate ordering remains a
maintainer decision, not a promise to reproduce every reference-game domain.
