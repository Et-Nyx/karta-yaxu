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

The original bundled creatures remain invented placeholders with retained ids.
The cached PokeAPI projection now supplies Fire/Flying/Dragon and Charizard base/
Mega X typing. A general network importer or complete mechanics model is not present.

## Reference scenarios to add incrementally

| Domain | Acceptance scenario | Status |
| --- | --- | --- |
| Source contexts | Distinguish individual game version, version group and generation from Karta definition/edit versions | Planned |
| Types | Ordered exact form/variant type pins; shared/world version independence | Implemented; effectiveness charts remain planned |
| Stats | Describe supplied stats and modifiers without making Pokémon's stat vocabulary a universal engine constraint | Planned; stat semantics open |
| Moves and learnsets | Keep method, level/order and source context; the same creature/move may have several learning records | Planned |
| Movesets | Distinguish learned/learnable moves from configured active moves and runtime selection | Planned |
| Abilities | Preserve available slots and hidden status, separate from a selected active ability; represent historical assignments | Planned |
| Natures | Reference increased/decreased stats and modifiers, separately from final stat calculation/rounding | Planned |
| Items | Distinguish item definitions, machine mappings, transformation requirements and wild held-item rarity | Planned |
| Descriptions | Language/game-qualified source text, normalized for display but inert; preserve source provenance | Planned; plain-text authored paragraphs implemented |
| Evolutions | Branches, conjunctions/alternatives, form restrictions and typed conditions bind explicit world entries | Planned |
| Transformations | Regional/persistent forms differ from Mega/Gigantamax activation and reversion; do not fold battle transformations into evolution | Planned; Mega X name/type reference only, no activation or reversion |
| Cosmetic forms | Multiple appearances may share a mechanical reference definition | Planned |
| Locations | Region/location/area relationships; actual playable map geometry needs a separate model | Planned |
| Encounters | Edition, method, conditions, levels and chances; resolve mutually exclusive conditions rather than summing unrelated rows | Planned |
| World packages | Exact world-entry/dependency closure and ruleset ids; deterministic, validated, self-contained export | Planned |
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

Published world packages and battles will pin complete content/rules/script
versions. Editing parameters does not grant permission to run or publish Lua.
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
