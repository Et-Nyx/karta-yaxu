# Local development placeholders

These invented names and descriptions were created for this prototype. They are
not approved game content or PokeAPI imports. No remote data or artwork is loaded.
All records are displayed as placeholders. Preserve their assigned opaque IDs
when changing names or slugs. The JSON format is internal, not an engine export.

Validation: `cargo run -p tools -- validate-fixtures`.

After deliberate migration, `cargo run -p web-app --bin admin -- seed-fixtures`
initializes the SQLite shared library. Existing creature IDs become species IDs;
default forms receive separate IDs. Initial versions are attributed to the named
fixture loader, without creating a user account. Reruns preserve designer edits and
validate creation baselines; they are not a fixture-overwrite/import mechanism.
Normal browsing reads SQLite rather than this JSON file.

The planned PokeAPI reference coverage is tracked in
`docs/architecture/reference-coverage.md`. Source fixtures and adapters will be
added with their domain slices; no PokeAPI fetching occurs in normal tests.
