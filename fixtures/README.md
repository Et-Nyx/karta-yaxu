# Local development placeholders

The original `creatures.json` names/descriptions were invented for this prototype.
`reference-types.json` and `pokeapi/` now contain a small cached Pokémon reference
projection; `worlds.json` defines synthetic authored lab worlds and `charts.json`
defines independently versioned synthetic effectiveness examples. `stats.json`
defines independent Stats, lab rosters and new sample creature values. No runtime remote
data or artwork is loaded. All records are placeholders, not approved game content.
Preserve their assigned opaque IDs
when changing names or slugs. The JSON format is internal, not an engine export.

Validation: `cargo run -p tools -- validate-fixtures`.

After deliberate migration, `KARTA_DEV_AUTH=1 cargo run -p web-app --bin admin -- seed-fixtures`
initializes all bundled creatures, Types, lab worlds, charts and Stats atomically. With no users
at all, it also creates the development administrator `admin` (customizable with
`--admin-subject`) and records a consumed invitation; sign in normally afterward.
A matching pending administrator invite is consumed; a lower-role invite rejects
the setup. All account/invitation/content/audit writes share one transaction.
Existing fixture worlds keep their eligible owner; otherwise a sole enabled admin
is selected. Use `--owner-user-id` for an explicit eligible owner. The two options
are mutually exclusive. Setup never promotes/enables users or transfers worlds.
See decision `0009-complete-fixture-bootstrap.md` for the owner-resolution contract.

For the old creature-only scope, use `admin seed-creature-fixtures`. Existing
creature IDs become species IDs; default forms receive separate IDs. Content
versions are attributed to the named fixture loader. Reruns preserve designer edits and
validate creation baselines; they are not a fixture-overwrite/import mechanism.
Normal browsing reads SQLite rather than this JSON file.

## Synthetic effectiveness charts

`charts.json` supplies shared **Lab Effectiveness**, **ClassicLab Chart**, and
**VariantLab Chart**. The numbers are synthetic authoring examples, not cached
official Pokémon effectiveness or a complete battle ruleset. ClassicLab explicitly
maps the three source Types; VariantLab has deliberate local differences and
unmapped Mist relationships, including Unknown, neutral, zero and fractional cells.
References use exact Type versions; pair lookup uses the same canonical identity
across metadata versions. Multi-Type composition/damage remains unimplemented.

Only complete `admin seed-fixtures` initializes the three charts, after the Type/
world prerequisites and within the same transaction as account bootstrap. Existing
targeted commands keep their earlier scopes. Complete reruns skip 44 objects and
preserve later chart edits; a chart collision, incomplete chart batch or late seal
failure rejects the entire invocation. Migration 0004 adds the chart schema without
seeding charts or changing any old snapshots. See decision 0010.

## Synthetic Stats and rosters

`stats.json` supplies shared **Momentum**, **Reserve** and **Precision**. Each lab
has independently constrained local copies. ClassicLab's explicit ordered roster
has three entries; VariantLab's has four, including world-exclusive **Resonance**.
These are invented authoring examples, not Pokémon source observations or formulas.

New shared form **StatsSample** (`/creatures/emberling/forms/statssample/stats`)
pins Emberling v1 and demonstrates `-1.25`, explicit zero and Missing. New variants
**Stats Sample Classic** and **Stats Sample Variant** use their own exact roster
pins and local values. Original fixture forms/variants and creation snapshots are
not retrofitted. Each new variant advances its world's membership revision once.

Only complete setup loads this batch. Targeted old commands create no Stats or
rosters. Fresh complete setup now has 6 forms, 7 variants, 10 Stat definitions,
2 rosters and 48 audit events; reruns skip 44 objects. Definition, roster and
sample-creation baselines must match, with later designer edits preserved. Partial
batches, conflicting custom rosters and late seals reject the entire setup,
including bootstrap/invitation/chart writes. Migrations 0005/0006 add schema only;
new creature snapshots are v3, while old v1/v2 bytes remain readable. See decision
0011 for exact decimal constraints, pinning and reconciliation.

## Type/reference and world fixtures

`admin seed-reference-fixtures` initializes the cached Fire/Flying/Dragon Types
and Charizard base/Mega X form typing. Files under `pokeapi/` are small selected
projections with URL, endpoint kind, external id, UTC retrieval time and transform
version. They are not raw full resources, sprites, complete rules or game editions.
Source `pokemon-form.is_default` is interpreted in its variety context, not copied
as the library's species-default flag; internal identities are allocated separately.

`admin seed-world-fixtures --owner-user-id <immutable-user-id>` initializes the
reference prerequisites plus ClassicLab/VariantLab in one atomic batch. The owner
must already be an enabled editor/administrator account. Targeted seed commands
create no accounts. Initial snapshots include all children and ordered pins; reruns check
creation baselines, skip matching objects and never restore over user edits.
Different ownership, drift, incomplete batches or collisions are errors.

The lab fixtures exercise same-source independent variants, three ordered Types,
exact source versions, and original world-exclusive Mist/Mistling. These are
synthetic authoring examples, not claims of official availability or executable
Pokémon behavior. Pairwise effectiveness authoring is implemented separately;
multi-Type damage and transformation execution remain unimplemented.

Reference coverage is tracked in `docs/architecture/reference-coverage.md`. Further
source fixtures/adapters follow their domain slices; normal tests never fetch PokeAPI.
