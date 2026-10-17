# Frozen card design package v2

**Delivered.** [ADR0016](../decisions/0016-card-definition-foundation.md)
extends [ADR0015](../decisions/0015-first-frozen-design-package.md). New browser
exports are `karta-design-package` schema **2**, producer `karta-yaxu`, read by
`game_data::card_package::CardPackage`. The standalone schema-1 reader and wire
contract remain unchanged; [design-package-v1.md](design-package-v1.md) is the
frozen historical specification and defines the reused typed payloads below.

This freezes design definitions, including well-formed diagnostic drafts, not
player instances or executable decks. No engine consumes it. Engine integration
and simulation are the absolute final steps of the product roadmap.

## Workflow and capture

The [README export workflow](../../README.md#frozen-design-packages) retains the
authorized GET/HEAD entry, immutable-world-ID-bound CSRF POST preview and reviewed
digest confirmation. Enabled eligible owners/collaborators only; no global admin
bypass. Capture checks permission in the world-first ordinary read transaction;
permission is rechecked before release. End capture before CPU validation,
diagnostics, hashing, serialization or rendering. Changed content needs renewed
confirmation. Final serialized bytes pass `CardPackage::from_bytes` before the
private/no-store attachment is released. No retained artifact, content mutation or
export audit event is created. There is no partial package.

`cargo run -p tools -- validate-package [path|-]` defaults to stdin, reads bounded
bytes without DB/network, dispatches manifest schema 1 to `Package::from_bytes`
and 2 to `CardPackage::from_bytes`, and rejects unsupported versions. Dispatch is
only a hint: the same complete bytes must pass the selected reader. Diagnostics
are separate from structural acceptance; CLI creation/import is not implemented.

## Ordered JSON wire contract

Notation follows the v1 specification: object fields below are **ordered**, `T?`
is an explicitly present nullable field, `[T]` an array, `ID` exactly 32 lowercase
hexadecimal characters, `Revision` a positive signed i64. Unknown/duplicate fields,
omitted nulls and alternate encodings are rejected. Reuse v1 `Fields`, `Constraints`,
canonical Decimal strings, typed `Key`, `Record`, `Binding`, payload tags/fields,
intrinsic validation and reference derivation unchanged.

```text
CardPackage = {"manifest": Manifest, "content": Content}
Manifest = {
  "format": "karta-design-package", "schema_version": 2,
  "producer": "karta-yaxu", "content_sha256": string,
  "root_count": integer, "record_count": integer
}
Content = {
  "schema_version": 2, "typed": TypedContent,
  "card_roots": [CardPin], "field_roots": [FieldPin],
  "cards": [CardRecord], "fields": [FieldDefinition]
}
TypedContent = {
  "schema_version": 1, "root_policy": "active-world-catalog-v1",
  "world": Key, "roots": [Key], "records": [Record], "bindings": [Binding]
}
CardIdentity = {"namespace": Namespace, "world_id": ID?, "id": ID}
CardPin = {"identity": CardIdentity, "version": Revision}
FieldPin = {"world_id": ID?, "field_id": ID, "version": Revision}
CardRecord = {
  "identity": CardIdentity, "revision": Revision, "fields": Fields,
  "archived_at": i64?, "source": CardPin?, "component": Component,
  "legacy_unmodeled": bool
}
Component = {
  "fields": [CardField]?, "rules_text": string?, "labels": [string]?
}
CardField = {"definition": FieldDefinition, "value": FieldValue}
FieldDefinition = {
  "id": ID, "world_id": ID?, "source": FieldPin?, "fields": Fields,
  "revision": Revision, "archived_at": i64?, "kind": FieldKind
}
ChoiceOption = {"id": ID, "label": string}
```

`Namespace` string spellings **and sort ranks**, in order, are `form`, `variant`,
`type`, `move`, `standalone` (not alphabetical). Card identities compare by
`(namespace rank, world_id, id)`, pins by `(identity, version)`; field pins compare
by `(world_id, field_id, version)`. Null scope sorts before string; IDs compare
lexicographically and versions numerically. Scope is shared/null or the one
`typed.world.id`. Forms must be shared, Variants local.

`typed.roots`, `typed.records` by key and `typed.bindings` by identity use v1 strict
sorting. `card_roots`, `field_roots`, `cards` by pin and `fields` by pin are strictly
increasing and unique. Do not sort nested authored fields, labels or choice options;
their order is content. Multiple exact revisions of an identity may be dependencies,
but each root identity appears once. Native typed records and their card components
are counted separately in the manifest:

```text
root_count   = len(typed.roots) + len(card_roots) + len(field_roots)
record_count = len(typed.records) + len(cards) + len(fields)
```

### Field kinds, values and invariants

FieldKind has exactly these forms (unit variants omit `definition`, not null):

```json
{"kind":"number","definition":{"whole_only":false,"minimum":null,"maximum":null}}
{"kind":"text","definition":{"max_length":2000}}
{"kind":"boolean"}
{"kind":"choice","definition":{"options":[]}}
{"kind":"card_reference"}
```

FieldValue has exactly these tags; the first three omit `value`:

```text
{"state":"missing"}
{"state":"unknown"}
{"state":"not_applicable"}
{"state":"number","value":Decimal}
{"state":"text","value":string}
{"state":"boolean","value":bool}
{"state":"choice","value":ID}
{"state":"card_reference","value":CardPin}
```

All three non-Known states are permitted for any kind. Known values must match
the exact embedded kind: number constraints accept the canonical Decimal; Text
length is at most `max_length` (u32, 0–2000), with no control except LF; Choice
names an option ID in that revision. Empty text, false and zero remain Known.
`component.fields=null` is unmodeled, `[]` defined-empty, and no assignment differs
from a selected field with Missing. Field identities in a component are distinct
and **same scope as the card**, even for world cards with shared sources.

Choice IDs are distinct; option labels and card labels are nonempty trimmed strings
of at most 120 Unicode characters without controls. Card labels are distinct and
ordered; `null` differs from `[]`. Rules text is nullable plain text of at most
2000 Unicode characters, permitting LF only among controls; `null` differs from
known empty text. Field/card metadata uses v1 normalization rules. Names, labels,
descriptions and rules text are inert, without gameplay semantics.

Only local fields may have a shared FieldPin source. Only local cards may source
shared cards in the corresponding namespace, except Variant sources Form.
Shared cards cannot reference local cards; local CardReference values may name
shared or same-world cards, never another world. All source/reference pins resolve
exactly. Old/archived pins are legal dependencies and are never auto-advanced.

`legacy_unmodeled=true` requires a native (not standalone) card with exactly
`{"fields":null,"rules_text":null,"labels":null}`. False with the same component
is valid for a newly sealed but unmodeled revision. Offline validation verifies
this shape, not the producer's database migration boundary or authenticity.

## Roots and complete combined closure

The typed projection retains `active-world-catalog-v1` roots and all v1 intrinsic,
scope, owner, source-mapping, embedded equality and binding checks. Add every active
local standalone card and reusable field as roots, even unused. Native card roots
must **exactly mirror** typed Variant/Type/Move roots at the same qualified identity
and revision; Forms are dependency-only. Card/field roots must be unarchived local
records, with distinct identities. Archived/older exact dependencies are allowed.
The reader validates declared roots; complete live catalog enumeration and
same-snapshot current bindings remain producer responsibilities.

`typed` is a schema-1 **projection, not necessarily a self-contained v1 package**.
A custom CardReference can reach an archived/older native card outside the typed
roots' own closure. Include its exact typed payload and all required typed closure
without promoting it to an active catalog root. Extracting `typed` and wrapping it
as v1 may therefore fail v1's no-extra-record rule. This does not relax the frozen
v1 reader: schema-2 validation supplies external native seeds only internally,
then checks reachability across the complete combined graph to reject extras.

An independent reader can implement acceptance as follows:

1. Validate schema/manifest/counts, canonical ordering, budgets and every DTO's
   intrinsic constraints. Build maps keyed by exact typed key, CardPin and FieldPin.
2. For **every** typed Form/Variant/Type/Move, require the same-revision CardRecord.
   Metadata and archive state must equal the typed payload. Source must equal the
   typed source (Variant→Form, local Type→shared Type, local Move→shared Move;
   otherwise none). Conversely every native card requires its exact typed payload.
3. Validate typed content using all v1 checks and references, allowing native typed
   keys supplied by card records as external closure seeds. Do not alter declared
   typed roots, binding requirements or intrinsic/cross-record constraints.
4. Build a combined graph with nodes `Typed(Key)`, `Card(CardPin)`, `Field(FieldPin)`.
   Typed edges are v1 exact references and required identity-binding targets; each
   native typed record also points to its same-revision Card node. Native Card
   nodes point back to their Typed node. Every Card points to its optional exact
   source, each embedded field's Field node and every CardReference value's Card
   node. Every Field points to its optional exact source Field node. Embedded
   definitions must equal their separate exact Field records, not just their IDs.
5. Traverse with visited sets from the union of typed/card/field roots. Every edge
   target must exist, and **all and only** provided graph nodes must be reachable.
   Disconnected cycles cannot justify extras; reachable cycles are legal. Preserve
   authored ordering/repetition even though each exact node is stored once.
6. Verify the digest and exact canonical tokens of the complete envelope.

V1 identity-only owner/default-Form bindings remain explicit same-snapshot
resolutions, not authored historical pins. Membership does not imply closure,
and a source Form never implicitly adds its independent Learnset. Card pins do
not invent pins to current rosters/charts/rulesets/learnsets/presets.

## Canonical bytes and digest

Use the exact [v1 JSON token rules](design-package-v1.md#canonical-bytes-and-sha-256):
compact UTF-8, field order above, literal non-ASCII, specified lowercase control
escapes, ordinary decimal integers, explicit nulls and canonical Decimal strings.
This is not alphabetical object-key sorting. Whitespace outside strings is
accepted on input; reordered fields, alternative escapes/numeric spellings or
omitted nullable fields are not. Parse, validate, reserialize and compare after
removing only outside-string JSON whitespace, as in v1.

```text
C = canonical_compact_utf8_json(content)
D = SHA256(ASCII("karta-design-package-content-sha256-v2\n") || C)
manifest.content_sha256 = lowercase_hex(D)
```

The marker ends in one LF byte, no NUL. Hash the whole schema-2 content object,
including `typed`, roots, cards and fields, not the manifest, `"content":` label,
original whitespace or entire envelope. This domain differs from v1 and inspection
fingerprint v2. A checksum is not authentication. No generation timestamp, private
account/access data, actors, audit events or raw snapshot JSON enter the package.

## Bounds, diagnostics and compatibility

The complete input (including whitespace) and serialized output are capped at
**32 MiB**. Combined typed/card/field roots ≤2048; combined records ≤8192; typed
bindings ≤8192. Combined occurrence budget ≤65,536 is computed as:

- Start with combined root count plus typed binding count.
- Add each typed payload's v1 occurrence cost.
- A FieldDefinition costs `1 + source-present + choice-option-count` (zero options
  for non-Choice kinds).
- Add that cost for each outer field definition.
- For each card add `2 + source-present + label-count`, then for each assignment
  add `2 + embedded-FieldDefinition-cost`.

Embedded/repeated occurrences count again; these are operational budgets, not
game cardinality rules. Capture retains 1 MiB preflight per snapshot, 16 MiB
accounted snapshot bytes, cooperative 10-second deadline and safe SQLite progress/
rollback/cancellation cleanup. Two export jobs/service hold admission through
blocking validation/serialization/render; previews show 25 roots/findings, never
truncate the package. Busy waits/filesystem/non-VM work can outlast the deadline.

Diagnostics recompute existing typed authoring findings: current Move/ruleset
policy 2, saved preset policy 1/2. Custom fields/prose do not execute or acquire
name-based validation semantics. Structurally valid Unknown/incomplete/contradictory
drafts are allowed. Broken references, intrinsic violations, unsupported formats/
policies, mismatched seals or capture gaps block production/acceptance.

Database migration 0009, native historical schemas, validator policies and package
schemas are separate versions. Old v1 files/readers remain supported unchanged;
no v1 file is rewritten. New exports require complete card closure, never a silent
legacy-only projection. Fixtures remain 65 objects/69 initial audit events; no
implicit custom fields or standalone cards are seeded.

Implementation: `crates/game-data/src/card_package.rs`,
`card_package/validation.rs`, `cards.rs`, `card_fields.rs`, reused `package/`
helpers; `crates/web-app/src/persistence/inspection.rs`, `application/export.rs`,
`web/export.rs`; `crates/tools/src/main.rs`. Shared-reader, end-to-end export,
workspace and keyboard checks passed. Instance/deck execution, engine
integration, simulation, schema migration tooling, signatures and retained
publication remain future work.
