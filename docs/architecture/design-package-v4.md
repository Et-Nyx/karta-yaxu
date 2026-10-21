# Frozen design package v4: independent formulas

ADR0019 implements schema 4 in `game_data::formula_package::FormulaPackage`.
Verification and its browser-run timeout are recorded in the
[derived-value guide](derived-values.md). Frozen [v1](design-package-v1.md), [v2](design-package-v2.md) and
[v3](design-package-v3.md) public readers/wire acceptance remain unchanged.

## Envelope and canonical hash

The envelope has `manifest`, then `content`. The manifest reuses `RecordManifest`:
`format`, `schema_version`, `producer`, `content_sha256`, `root_count`, `record_count`.
Format/producer retain the existing package constants; schema version is **4**.
Content fields, in canonical order, are:

1. `schema_version`: 4.
2. `records`: unchanged schema-3 `RecordPackageContent`, with its schema-2 `cards`
   content and schema-1 `typed` projection nested within it.
3. `formula_roots`: exact `FormulaPin` objects (`world_id`, `formula_id`, `version`).
4. `formulas`: exact `FormulaDefinition` objects.

These are nested content objects, not additional manifest/package envelopes.
The v3 combined record/card/typed closure and embedded equality checks still apply.
Formula evidence is independent and cannot expand or substitute for that closure.

`content_sha256` is lowercase hex SHA-256 over the bytes
`karta-design-package-content-sha256-v4\n` followed by compact canonical content
JSON. It is neither an inspection fingerprint nor a signature. Manifest counts
must match the combined content. There are no timestamps, calculator samples,
results, private ownership/access/account data, actors, sessions or audit records.

Roots and definitions are strictly sorted by `(world_id, formula_id/id, revision)`;
duplicates are rejected. Authored input/step order and operand occurrences remain
meaningful, not sorted away. Definitions serialize `id`, `world_id`, `fields`,
`revision`, `archived_at`, `semantics_version`, `inputs`, `steps`, `output` in that
order, using the strict domain encodings. Metadata, constraints and decimals retain
their canonical encodings and explicit nulls. Operands are tagged
`{"kind":"constant|input|step","value":...}`; constants use canonical Decimal
strings. Operations use `{"kind":"add"}` and the other named operations; round
adds `scale` and `mode`. Extra operation fields are invalid, including on unit operations.

The reader rejects unknown/duplicate fields, wrong field order, alternative value
encodings and unsorted keyed collections. JSON whitespace alone is allowed.
Deserialization is followed by full validation and comparison against canonical
serialization; it does not repair supplied bytes. `build` sorts producer vectors;
`from_bytes` rejects unsorted input. No silent projection-only download is allowed.

## Roots and exact closure

The producer adds every **current active unarchived local formula** as a root to
the existing active-world catalog selection. Each formula root must have exactly
one matching active definition at its exact scope/id/revision. Formula world scope
must equal `records.cards.typed.world.id`; shared or foreign-world formulas cannot
be roots or unrelated evidence. Only one current revision per identity is allowed.

Semantics **1** permits only constants, declared input IDs and earlier step IDs;
there are no external content/formula dependencies. Consequently the formula closure
is exactly its roots: missing, extra, duplicate, archived, foreign, invalid or
unsupported formula evidence rejects the package. The reader verifies declared
closure, not completeness against a live database; catalog enumeration is the
producer's responsibility. Archived formulas remain inspectable/history-readable
but are not export roots, and no invented usage pulls them into the package.

Definition validation includes the 100-input/200-step format bounds, stable IDs,
normalized names/units and constraints, exact operation arity, round scale 0–6,
reference order and explicit valid output. See [derived values](derived-values.md)
for signed i64 rational/checked i128 evaluation, rounding and state semantics.
The package contains definitions, **not evaluated samples**. A structurally valid
formula that would fail for some samples remains exportable. Diagnostics reuse
schema-3 authoring findings (current policy 2, saved preset policy 1/2); they do not
evaluate formulas or establish gameplay, engine or combat acceptance.

## Bounds and release

Input and complete output are capped at **32 MiB**. Combined v4 limits are **2,048
roots**, **8,192 evidence records**, **65,536 authored occurrences**; formula roots
and definitions contribute alongside typed/card/field/layout/collection/record
evidence. Formula occurrence accounting adds each root, definition, output operand,
input, step and each step operand to the existing v3 accounting, including its
embedded evidence. These are operational format bounds, not game cardinality rules.

Capture uses one world-first ordinary SQLite read transaction, shared checked readers,
preflight/cancellation cleanup and combined inspection budgets: 1 MiB per snapshot,
16 MiB total accounted snapshot bytes, 2,048 roots, 8,192 nodes, 65,536 edges and a
cooperative 10-second acquisition/anchor/capture deadline. Busy waits/filesystem work
can outlast it; it is not hard real-time. Transaction completion precedes CPU work.
There are at most two export jobs per service, held through blocking validation,
hashing, serialization and preview rendering, including worker cancellation.

The native authorized export workflow remains GET/HEAD entry → ID-bound
CSRF-protected POST preview → digest-bound confirmed POST download. Enabled eligible
world owners/collaborators only; no administrator bypass. Confirmation recaptures;
changed content requires fresh review and permission is rechecked before release.
Final serialized bytes must pass the actual shared v4 reader. Gaps, unsupported data
or exhausted budgets block export rather than emit a partial file. Attachments are
private/no-store, with no retained artifact or content/history write.

`tools validate-package [path|-]` dispatches **1/2/3/4** to the corresponding strict
reader, bounded and DB/network-free; stdin is the default. CLI package creation,
engine consumption, scripts, live bindings and formula source adoption are absent.
Migration 0011 is additive, deliberate and backup-first; it was not applied to the
local database for this milestone. No old seed scopes/baselines change.

Source: `game-data/src/formula_package.rs`, `formula_package/budgets.rs`, web-app
`application/export.rs`, `persistence/inspection/formula_capture.rs` and
`web/export.rs`. Export tests cover exact formula roundtrips, independent v4 hashing,
archive-triggered confirmation refresh and unchanged nested record content.
