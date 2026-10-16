# Local development placeholders

The original `creatures.json` names/descriptions were invented for this prototype.
`reference-types.json` and `pokeapi/` now contain a small cached Pokémon reference
projection; `worlds.json` defines synthetic authored lab worlds and `charts.json`
defines independently versioned synthetic effectiveness examples. `stats.json`
defines independent Stats, lab rosters and sample creature values. `moves.json`
adds synthetic move attributes, Moves, rulesets, independent learnsets and named
presets on the existing StatsSample owners. No runtime remote data or artwork is
loaded. All records are placeholders, not approved game content.
Preserve their assigned opaque IDs
when changing names or slugs. The JSON format is internal, not an engine export.

Original creature catalog validation: `cargo run -p tools -- validate-fixtures`.
Complete setup also strictly loads the dedicated manifests; fixture tests cover
their canonical values, exact dependencies and immutable creation baselines.

After deliberate migration, `KARTA_DEV_AUTH=1 cargo run -p web-app --bin admin -- seed-fixtures`
initializes all bundled creatures, Types, lab worlds, charts, Stats and Moves/
learning examples atomically. With no users
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
targeted commands keep their earlier scopes. Complete reruns skip 65 objects and
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
rosters. The retained Stats-era baseline has 44 objects and 48 audit events,
including 6 forms, 7 variants, 10 Stat definitions and 2 rosters. Definition, roster and
sample-creation baselines must match, with later designer edits preserved. Partial
batches, conflicting custom rosters and late seals reject the entire setup,
including bootstrap/invitation/chart writes. Migrations 0005/0006 add schema only;
new creature snapshots are v3, while old v1/v2 bytes remain readable. See decision
0011 for exact decimal constraints, pinning and reconciliation.

## Synthetic Moves, learning and presets

`moves.json` is a strict version-1 manifest with immutable canonical IDs, explicit
nullable fields, tagged numeric states and canonical decimal strings. Compact
attribute/Move pins are resolved to exact domain definitions when loading, rather
than using current database heads or names. These examples are invented authoring
data, not PokeAPI observations, official learning availability or implemented effects.

- **Impact** and **Precision** are two shared numeric move attributes, each with an
  independent local copy in each lab: **6 definitions**. Units and bounds are
  descriptive/configured data. Move Precision is a separate identity from the
  creature Stat of the same name; neither name implements a formula.
- **Probe** and **Veil** have shared definitions and two local copies per lab:
  **2 shared + 4 local Moves**. Probe has known zero Impact and Unknown Precision;
  Veil has NotApplicable Impact and known `0.75` Precision. Local definitions and
  Moves pin shared revision 1 and explicitly map source attribute identities.
- **2 world rulesets** pin their exact local attribute expectations, existing
  Stat-roster v1 and chart v1. ClassicLab's example selects maximum **2** and
  **forbid repeats**; VariantLab selects **no specified maximum** and **allow
  repeats**. These are user-configured examples, not application defaults or
  permanent mechanical claims. Required Impact makes Veil a contradictory draft;
  required Unknown Precision makes Probe incomplete. Notes remain inert.
- **3 independent learnsets** belong to the existing shared **StatsSample** form
  and the existing **Stats Sample Classic / Variant** variants. Each seals three
  acquisition records: two refer to the same Probe revision without collapsing.
  Method/context/condition prose can be Unknown and is never executed. Thresholds
  demonstrate Unknown, known zero and NotApplicable, with no implicit level meaning.
  Local learnsets explicitly pin the shared learnset v1 and map both shared Moves
  to exact local Moves. Copied acquisition IDs remain stable in each separate scope.
- **4 named presets** pin the same-owner learnset v1, same-world ruleset v1 and
  validator policy **1**. ClassicLab's **Overfilled Repeat Draft** shows cap/repeat/
  required-NotApplicable contradictions; **Unknown Input Draft** is incomplete.
  VariantLab's **Restriction Draft** contradicts the required Impact expectation;
  **Repeat Exploration** retains repeated selections as an incomplete draft under
  its allow-repeat configuration. Reports distinguish contradictions, incomplete
  inputs and checks not evaluated; they are **not combat readiness**.

Creation baselines deliberately remain on policy 1, including its original optional
policy diagnostics. New authored presets use corrected policy 2; an editor can
explicitly preview/confirm revalidation of an existing preset as a new attributed
revision. Complete setup never upgrades those heads or changes historical reports.

Browse `/moves`, `/move-attributes`, each world's local catalogs and `/ruleset`,
then the StatsSample form/variant **Learnset** and variant **Moveset presets** links.
Learning, preset and ruleset histories are independent: these 21 new objects do
not create forms/variants or advance existing form, variant or world revisions.
No creature v4 snapshot is introduced; all prior history bytes are preserved.

Fresh complete setup contains **65 objects and 69 audit events**. A complete 0006
baseline retains its 44 objects/48 audit events and gains only the 21 new objects/
audits on explicit setup. Reruns skip 65 matching objects and preserve later edits,
including exact old source/dependency pins. A partial new batch, changed creation
baseline/actor, conflicting custom ruleset or owner learnset rejects the whole
invocation. Failure at the very last preset entry/seal rolls back bootstrap,
invitations and all earlier fixture groups. Migrations 0007/0008 add schema only;
targeted creature/reference/world commands initialize zero Moves/learning objects.
See [ADR0012](../docs/decisions/0012-moves-learning-and-world-rulesets.md) and
[ADR0013](../docs/decisions/0013-authoring-validation-policy-and-archive-selection.md).

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
The [pinned roadmap](../docs/architecture/roadmap.md) retains deferred formulas,
source-edition models, imports, engine packages and simulation directions.
