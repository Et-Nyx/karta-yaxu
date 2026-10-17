# Pokémon reference-model coverage

Pokémon is a temporary stress fixture for an adaptable authoring model. This is a
coverage checklist, not a claim that PokeAPI is a complete executable ruleset or
that the current application implements all of these concepts.

## Implemented foundation

- Shared canonical species identities; separate default/additional form ids.
- Immutable typed species/form definition versions with exact parent pins.
- World creation by editor/administrator, ownership and scoped collaboration.
- Multiple independently editable world variants of the same species/form,
  including the same source version; separate world-exclusive entries.
- Explicit adoption of newer form versions, with old/local/new comparison.
- Transactional versions, snapshots and audit; optimistic concurrency; public
  content history and private access history; keyboard authoring at two widths.
- Shared/world-local Type definitions and exact ordered form/variant typing;
  compatible v1/v2 history and explicit unmodeled versus defined-empty state.
- Cached Type/Charizard reference subset, source provenance and owned synthetic labs.
- Independently versioned shared/local pairwise charts with exact fractions,
  explicit source mappings/adoption, Unknown/neutral/no-effect distinctions and
  canonical Type identity matching across metadata revisions.
- Independent shared/local Stat definitions, explicit variable-length world
  rosters, exact signed decimal base values and Missing/zero distinctions;
  pinned constraints, explicit reconciliation/mapping and v3 creature snapshots.
- Independent shared/local numeric move-attribute definitions with descriptive
  units and exact constraints, distinct from creature Stats; Moves own exact
  attribute revisions and Known/Unknown/NotApplicable values. Unmodeled, empty,
  absent and known-zero inputs remain distinct; no fixed attribute vocabulary/count.
- Explicit injective attribute/Move source mappings, keep-local or previewed
  whole-collection replacement, and independent source pins without retargeting.
- One independent learnset per shared form/local variant, ordered stable scoped
  acquisition IDs and multiple records for the same Move. Optional method/context/
  condition prose is inert; numeric thresholds are not implicitly levels.
- Multiple named variant presets with ordered acquisition selections and exact
  same-owner learnset/same-world ruleset pins. Repeated selections are data;
  forbid-repeat diagnostics compare canonical Move identity.
- Independently versioned world ruleset configuration with exact local attribute
  expectations, optional roster/chart context, optional maximum, unspecified/allow/
   forbid repeat policy and inert notes. Deterministic versioned authoring reports
  identify exact inputs, contradictions, incomplete inputs and checks not evaluated.
  Well-formed diagnostic drafts are saveable; structurally invalid references,
  malformed numbers and intrinsic constraint violations are errors. These reports
  do not evaluate executable eligibility or combat readiness.
- Public read-only world inspection captures independent current roots (including
  archived/unused local objects), explicit absences and exact dependency closure in
  one ordinary read transaction. Shared sources require explicit dependencies;
  source forms do not implicitly add independent learnsets. Current catalog/owner
  observations are not historical world-revision pins. Older versions, repeated
  acquisitions and preset selection occurrences remain distinct.
- Typed policy-2 Move/ruleset findings, saved policy-1/2 preset reports and optional
  ad-hoc policy-2 comparisons; incomplete/context-difference/contradiction and
  not-evaluated categories remain distinct from capture gaps. Reached archived
  dependencies remain valid/informational. Chart lookup preserves scoped canonical
  Type identity across metadata versions and neutral/zero/Unknown/outside states.
- Complete-only deterministic public-semantic inspection fingerprints and fresh
  request capture with changed-input notice/page-1 reset; partial coverage has no
  fingerprint or “Unused” claim. Public DTOs exclude private world/access/account
  data, actors, grants, sessions, raw audits and raw snapshot JSON from reports/hashes.
- World-scoped where-used for all captured versions or one exact revision, direct
  content sites and bounded representative shortest root paths. Membership/ownership
  is not usage; learnset acquisition context is not actual preset selection. Neither
  global/all-history usage nor safe deletion, combat readiness or export readiness
  is established by this transient dependency graph.
- Frozen `karta-design-package` v1 export for enabled editor/administrator owners
  and collaborators of an active world, without global administrator bypass.
  Authorized native entry/POST preview/CSRF confirmation downloads recapture and
  recheck permission, requiring fresh confirmation after changed inputs.
- Active-catalog roots, older/archived exact dependency closure and explicit
  capture-time owner/default-form identity bindings; ordered/repeated occurrences
  survive. Deterministic manifest/content JSON and a dedicated SHA-256 domain are
  separate from inspection fingerprints. Private data and generation timestamps
  are excluded; no content changes, migrations or fixture objects are added.
- Shared offline Rust package reader and `tools validate-package [path|-]` (stdin
  by default) verify structure, canonical wire encodings, exact embedded equality,
  declared roots and full closure. Current diagnostics use policy 2 and saved
  presets retain 1/2; well-formed Unknown/contradictory/incomplete drafts are allowed.
  This is an intended engine integration contract, not actual engine integration,
  executable gameplay acceptance or a CLI package-creation workflow (ADR0015).

The original bundled creatures remain invented placeholders with retained ids.
The cached PokeAPI projection now supplies Fire/Flying/Dragon and Charizard base/
Mega X typing. A general network importer or complete mechanics model is not present.

## Reference scenarios to add incrementally

| Domain | Acceptance scenario | Status |
| --- | --- | --- |
| Source contexts | Distinguish individual game version, version group and generation from Karta definition/edit versions | Planned |
| Types | Ordered exact pins; shared/world independence; directed pairwise effectiveness | Implemented; synthetic chart fixtures; multi-Type composition remains planned |
| Stats | Describe supplied stats and modifiers without making Pokémon's stat vocabulary a universal engine constraint | Definitions, rosters and signed decimal base values implemented; calculation/modifier mechanics planned |
| Moves and learnsets | Extensible move-owned numeric values; preserve multiple ordered acquisition records, descriptive method/context/condition and explicit threshold states | Shared/local Moves, attributes and independent learnsets implemented; source-edition-qualified learning and executable acquisition eligibility remain planned |
| Movesets | Distinguish authored acquisition records, named configured presets and runtime repertoire/selection | Multiple independently versioned variant presets and exact-context policy-1/2 reports with explicit revalidation implemented; individual learned repertoires and runtime selection remain planned |
| World ruleset configuration | Pin exact expectations and roster/chart context; author maximum/repeat policies without universal Pokémon limits | Configuration and authoring diagnostics implemented; formulas, effects, conditions and battle execution remain planned |
| World inspection and dependencies | Capture current independent roots and exact closure consistently; explain authoring findings and scoped direct/root usage without losing old pins or repeated sites | Implemented read-only dependency graph with complete-only public fingerprint, explicit partial gaps and bounded where-used; not an engine package or readiness verdict (ADR0014) |
| Abilities | Preserve available slots and hidden status, separate from a selected active ability; represent historical assignments | Planned |
| Natures | Reference increased/decreased stats and modifiers, separately from final stat calculation/rounding | Planned |
| Items | Distinguish item definitions, machine mappings, transformation requirements and wild held-item rarity | Planned |
| Descriptions | Language/game-qualified source text, normalized for display but inert; preserve source provenance | Planned; plain-text authored paragraphs implemented |
| Evolutions | Branches, conjunctions/alternatives, form restrictions and typed conditions bind explicit world entries | Planned |
| Transformations | Regional/persistent forms differ from Mega/Gigantamax activation and reversion; do not fold battle transformations into evolution | Planned; Mega X name/type reference only, no activation or reversion |
| Cosmetic forms | Multiple appearances may share a mechanical reference definition | Planned |
| Locations | Region/location/area relationships; actual playable map geometry needs a separate model | Planned |
| Encounters | Edition, method, conditions, levels and chances; resolve mutually exclusive conditions rather than summing unrelated rows | Planned |
| World packages | Explicit package roots and exact dependency closure, ruleset ids; deterministic, validated, self-contained export | First Frozen Design Package v1 delivered: active catalog, exact closure/bindings, owner/collaborator preview/confirm download, shared offline reader and validation CLI; actual engine integration/gameplay acceptance and retained publication remain deferred (ADR0015) |
| Simulation | Independently verify executable stat/damage/turn/effect/transformation rules and deterministic replays | Later battle-core milestone |

## Proposed bounded source set

Use a small selected set of species and supporting references, not recursive
fetching of every linked object:

- Charizard: base, Mega X/Y and Gigantamax, sharing one species identity.
- Ordinary/Alolan Rattata and Raticate: form-qualified conditional evolution.
- Unown A/B: cosmetic forms sharing a mechanical definition.
- Clefairy: historical types/abilities/stats and selected Metronome learning rows
  with level-up/machine/tutor contexts.
- Pidgey plus a few selected area encounters: version/time/method/level variation.
- A few supporting types, moves (including Tackle), abilities, items, a nature
  and machine mapping; only the source game contexts needed by those cases.

Create two synthetic worlds and two same-source variants in one of them. The
synthetic worlds are test configurations, not claims of exact official-game
availability. Distinguish source observations, curated additions, and unsupported
or unknown executable semantics.

Implemented subset: `fixtures/reference-types.json`, `fixtures/pokeapi/` and
`fixtures/worlds.json`, initialized by complete `admin seed-fixtures` setup or
targeted reference/world commands. ClassicLab and VariantLab have a retained,
explicitly chosen or bootstrapped development owner, independent same-source entries, local
Type versions, three-Type typing and world-exclusive Mist/Mistling. No fixture
generation depends on a live network during use/tests; current canonical ids never
reuse PokeAPI ids. See decision 0008 for the exact slice and history compatibility,
and 0009 for atomic setup and development administrator bootstrap.
`fixtures/charts.json` adds a synthetic shared chart and independent lab charts
through complete setup, with explicit mappings and original Mist relationships.
These factors are not source observations or claimed official Pokémon rules.
Decision 0010 defines exact rational storage, canonical metadata-identity matching,
independent chart history and explicit adoption. Frozen packages preserve exact
chart pins; simulation must separately define stacking/rounding before calculating
damage.

`fixtures/stats.json` adds three shared Stats, seven independent local Stats,
three-/four-entry lab rosters and new sample form/variants. Values and constraints
are synthetic, not sourced Pokémon mechanics. Decision 0011 preserves exact
definition/roster context and old history; frozen packages now preserve that context.
Formulas, modifiers and actual engine integration remain separate work.

`fixtures/moves.json` adds invented **Impact/Precision** move attributes, **Probe/
Veil**, independent local copies, two lab rulesets, three learnsets and four named
presets. This batch reuses StatsSample's existing form/variant identities and adds
no creature or world revisions. The original 44-object creation baseline and all
old bytes remain unchanged; complete setup totals 65 objects/69 initial audit
events. Unknown, NotApplicable, zero, repeated acquisitions, contradictory drafts
and incomplete inputs are examples, not claims of official Pokémon mechanics.
ClassicLab's maximum-two/forbid policy and VariantLab's unspecified maximum/allow
policy are authored configurations, not universal rules.

Decision [0012](../decisions/0012-moves-learning-and-world-rulesets.md) keeps these
histories separate: form versions do not capture learnsets, variant versions do
not capture presets, and world revisions do not capture rulesets. There is no new
creature v4 projection. Dependencies stay on exact saved revisions until explicit
adoption/reconciliation; new selections require unarchived definitions, while
retained exact archived pins permit unrelated edits. No general sourced-learning
edition/generation model, level-up semantics, condition execution or combat
readiness claim is supplied by these authored records.

Inspection reuses ClassicLab, VariantLab and the existing StatsSample examples;
it adds no schema, dependencies or fixture objects. Complete setup remains **65
objects/69 initial audit events**, with old heads/history preserved. Public native
GET filters and 25-entry pages do not save or adopt content. Capture defaults are
2,048 roots, 8,192 exact nodes, 65,536 edge occurrences, 1 MiB per snapshot, 16 MiB
accounted snapshot bytes and a cooperative 10-second deadline; these are operational
capture limits, not game rules; export shares these defaults and adds
package-specific bounds below. Where-used bounds results to 256, depth to 32 and
work to 65,536, explicitly reporting truncation. See
[ADR0014](../decisions/0014-world-inspection-and-dependency-visibility.md) and the
[inspection workflow](../../README.md#world-inspection-and-where-used).

Export also reuses these labs unchanged: **65 objects/69 initial audit events**.
Roots are the active world, current unarchived local Types/Stats/move attributes/
Moves/variants/chart, configured roster/ruleset, and current learnsets/unarchived
presets of active variants. Required archived/old dependencies remain included;
archived variants' children are not independent roots. Shared learnsets require
explicit references, not a variant's source form. Identity bindings resolve owners/
default forms in the same capture transaction, without inventing historical pins.
The offline reader verifies declared root eligibility/closure; completeness against
the live catalog is the producer's responsibility. Package input/output is capped
at 32 MiB, with two export jobs per service held through validation/serialization/
render, blocking CPU work and the same cooperative capture/cancellation caveats as
inspection. Preview roots/findings show at most 25 each; packages are complete or
rejected. See [ADR0015](../decisions/0015-first-frozen-design-package.md) and the
[v1 protocol](design-package-v1.md).

The [pinned product roadmap](roadmap.md) preserves all deferred directions,
including derived formulas/modifiers, source contexts/imports, recovery, engine
integration/further package workflows and deterministic simulation. Engine
integration and a minimal executable contract are proposed directions, not a newly
approved next milestone.

## Adapter and version rules

PokeAPI is an import adapter, never a browsing/combat runtime dependency. Capture
bounded local source fixtures with endpoint kind, external id/URL, UTC retrieval
time and transformation version. Allocate internal canonical ids independently;
endpoint ids from `pokemon-species`, `pokemon` and `pokemon-form` are different
namespaces. Imports must be idempotent, previewed before replacing edited library
content, and create new attributed definition versions. Existing world pins stay
fixed. Normal tests use local fixtures and forbid external requests.

Historical coverage has mixed granularity. Empty lists and absent historical
values do not establish unavailability or unchanged rules. Form/evolution
introduction groups do not establish all later availability. Preserve supplied
facts and document adapter derivations; mark unknowns instead of guessing.

Shared relations must pin their dependencies; world relations must target
explicit same-world entries. An encounter or evolution cannot choose an arbitrary
entry by species id when several variants exist. Changes to owned relations
advance their aggregate revision and snapshot in the same audited transaction.

Frozen design packages pin their declared design-content closure and ruleset
revisions; they contain no executable scripts. Future published playable packages
and battles need complete executable rules/script contracts as well. Editing or
exporting parameters does not grant permission to run or publish Lua.
Imported effect prose and condition expressions are not automatically executable.

## Sources reviewed during planning

Current official [PokeAPI v2 documentation](https://pokeapi.co/docs/v2), including
species, Pokémon, forms, evolution chains, games, types, moves, abilities, natures,
items, locations and encounters. Representative read-only samples included:

- https://pokeapi.co/api/v2/pokemon-species/charizard/
- https://pokeapi.co/api/v2/pokemon/clefairy/
- https://pokeapi.co/api/v2/pokemon-form/charizard-mega-x/
- https://pokeapi.co/api/v2/pokemon-form/charizard-gmax/
- https://pokeapi.co/api/v2/evolution-chain/7/
- https://pokeapi.co/api/v2/pokemon-form/unown-b/
- https://pokeapi.co/api/v2/location-area/296/
- https://pokeapi.co/api/v2/move/tackle/
- https://pokeapi.co/api/v2/type/steel/

The cached Type/Charizard projections now record the relevant baselines and retrieval
metadata locally. Other URLs above remain planning references, not fixture baselines;
capture and validate those slices when implemented.
