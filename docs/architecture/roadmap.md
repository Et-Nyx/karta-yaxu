# Product roadmap

This preserves the maintainer's milestone discussion. It is a set of directions,
not a promise that all mechanics or infrastructure choices are settled.

## Foundation delivered

Shared species/forms, independently owned world variants, scoped collaboration,
explicit source adoption, immutable attributed history, exact typing, independent
effectiveness charts, independent Stats, ordered world stat rosters and signed
decimal base values. Pokémon is a temporary reference stress fixture.

## Current focus: Moves, Learnsets and Movesets

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

## Pinned follow-up directions

1. **Derived Stats, formulas and modifiers:** explicit inputs, dependency/cycle
   validation, evaluation order, exact rounding/clamping policy and inspectable
   breakdowns. Unknown required inputs produce explained unresolved results.
2. **World validation and authoring recovery:** where-used views, cross-world
   comparisons, incomplete/outdated-context reports, explicit archive, attributed
   revert-as-new-revision and ownership recovery.
3. **Immutable world packages and engine export:** exact dependency closure,
   deterministic content, schema/ruleset IDs, manifest/checksums, real shared/engine
   importer validation and atomic publication. Export permissions remain open.
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

Recommended path: moves/availability/presets → validation and first export → a
minimal executable ruleset and CLI battle → browser PvE. This is adjustable by the
maintainer; completing every reference domain is not a prerequisite to a first
bounded export or simulation.
