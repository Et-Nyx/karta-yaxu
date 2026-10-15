# Local development placeholders

The original `creatures.json` names/descriptions were invented for this prototype.
`reference-types.json` and `pokeapi/` now contain a small cached Pokémon reference
projection; `worlds.json` defines synthetic authored lab worlds. No runtime remote
data or artwork is loaded. All records are placeholders, not approved game content.
Preserve their assigned opaque IDs
when changing names or slugs. The JSON format is internal, not an engine export.

Validation: `cargo run -p tools -- validate-fixtures`.

After deliberate migration, `KARTA_DEV_AUTH=1 cargo run -p web-app --bin admin -- seed-fixtures`
initializes all bundled creatures, Types and lab worlds atomically. With no users
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
Pokémon behavior. Type effectiveness/transformation execution remains unimplemented.

Reference coverage is tracked in `docs/architecture/reference-coverage.md`. Further
source fixtures/adapters follow their domain slices; normal tests never fetch PokeAPI.
