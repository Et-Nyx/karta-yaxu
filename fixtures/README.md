# Local development placeholders

The original `creatures.json` names/descriptions were invented for this prototype.
`reference-types.json` and `pokeapi/` now contain a small cached Pokémon reference
projection; `worlds.json` defines synthetic authored lab worlds. No runtime remote
data or artwork is loaded. All records are placeholders, not approved game content.
Preserve their assigned opaque IDs
when changing names or slugs. The JSON format is internal, not an engine export.

Validation: `cargo run -p tools -- validate-fixtures`.

After deliberate migration, `cargo run -p web-app --bin admin -- seed-fixtures`
initializes the SQLite shared library. Existing creature IDs become species IDs;
default forms receive separate IDs. Initial versions are attributed to the named
fixture loader, without creating a user account. Reruns preserve designer edits and
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
must already be an enabled editor/administrator account. No synthetic accounts are
created. Initial snapshots include all children and ordered pins; reruns check
creation baselines, skip matching objects and never restore over user edits.
Different ownership, drift, incomplete batches or collisions are errors.

The lab fixtures exercise same-source independent variants, three ordered Types,
exact source versions, and original world-exclusive Mist/Mistling. These are
synthetic authoring examples, not claims of official availability or executable
Pokémon behavior. Type effectiveness/transformation execution remains unimplemented.

Reference coverage is tracked in `docs/architecture/reference-coverage.md`. Further
source fixtures/adapters follow their domain slices; normal tests never fetch PokeAPI.
