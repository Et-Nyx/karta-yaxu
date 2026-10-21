# Frozen design package v3 — reusable record closure

[ADR0018](../decisions/0018-card-records-and-content-relationships.md) introduces
`game_data::record_package::RecordPackage`. The v1 and v2 wire contracts/readers remain
unchanged; [v1](design-package-v1.md) and [v2](design-package-v2.md) define reused data.
This is an offline design-content protocol, not executable rules or engine acceptance.

## Envelope and canonical representation

All object fields below are in required serialization order. Nullable fields must
be present as `null` or a value. Reject unknown/duplicate fields, unsupported versions,
noncanonical token encodings and unsorted keyed vectors. Whitespace outside JSON
strings is allowed; canonical output is compact UTF-8, no BOM or trailing newline,
with the exact string/integer/decimal encodings specified in v1.

```text
RecordPackage = {"manifest": Manifest, "content": Content}
Manifest = {
  "format": "karta-design-package", "schema_version": 3,
  "producer": "karta-yaxu", "content_sha256": lowercase-hex-64,
  "root_count": integer, "record_count": integer
}
Content = {
  "schema_version": 3, "cards": CardPackageContentV2,
  "layout_roots": [LayoutPin], "collection_roots": [CollectionPin],
  "layouts": [LayoutRecord], "collections": [CollectionDefinition],
  "relations": [CardRelationRecord]
}
LayoutPin = {"world_id": ID?, "layout_id": ID, "version": Revision}
CollectionPin = {"world_id": ID?, "collection_id": ID, "version": Revision}
LayoutRecord = {
  "id": ID, "world_id": ID?, "source": LayoutPin?, "fields": Fields,
  "revision": Revision, "archived_at": i64?, "columns": [Column]
}
Column = {"id": ID, "label": string, "kind": ColumnKind}
CollectionDefinition = {
  "id": ID, "world_id": ID?, "source": CollectionPin?, "fields": Fields,
  "revision": Revision, "archived_at": i64?, "layout": LayoutRecord
}
CardRelationRecord = {
  "card": CardPinV2, "relations": {"collections": [Assignment]?},
  "legacy_unmodeled": boolean
}
Assignment = {"definition": CollectionDefinition, "value": CollectionValue}
Entry = {"id": ID, "cells": [Cell]}
Cell = {"column_id": ID, "value": RecordValue}
```

`ID` is 32 lowercase hex characters; `Revision` is a positive i64. `Fields`, exact
`FieldDefinition`, `CardPinV2` and canonical decimal strings reuse v2. `cards` has
content schema 2 and its existing schema-1 typed projection, not a manifest or a
separately checksummed nested file. References reached only through records may
extend these projections, so they need not independently form closed v1/v2 packages.

### Tagged kinds and states

```text
ColumnKind = {"kind":"field", "definition": FieldDefinitionV2}
          | {"kind":"reference", "definition":{"allowed":[ReferenceKind]}}
ReferenceKind = "card" | "stat" | "field"

DefinitionPin = {"kind":"card", "pin":CardPinV2}
              | {"kind":"stat", "pin":{"world_id":ID?,"stat_id":ID,"version":Revision}}
              | {"kind":"field", "pin":FieldPinV2}

CollectionValue = {"state":"missing"}
                | {"state":"unknown"}
                | {"state":"not_applicable"}
                | {"state":"entries", "entries":[Entry]}

RecordValue = {"state":"missing"}
            | {"state":"unknown"}
            | {"state":"not_applicable"}
            | {"state":"number", "value":DecimalString}
            | {"state":"text", "value":string}
            | {"state":"boolean", "value":boolean}
            | {"state":"choice", "value":StableChoiceID}
            | {"state":"reference", "value":DefinitionPin}
```

Payload-free states omit their payload entirely, even if a generic parser accepts
an explicit null. Reserialization/token comparison rejects alternate representations.
Allowlist order is authored and must be preserved; entries must be nonempty/distinct.
Column/entry/cell order is authored, not sorted. Column labels have 1–120 normalized
characters. Flat scalar columns embed exact same-scope fields; text allows LF and
the exact field's 0–2000 character bound. Scalar CardReference fields permit Card
pins only. Reference columns enforce their allowlist. Shared references are shared;
local references are shared or same-world. No private target kind is defined.

An absent cell has no entry in `cells`; it differs from explicit Missing/Unknown/NA.
Known empty text, false and zero retain their values. Inventories distinguish null
from empty arrays; collections distinguish absent assignments, explicit states and
known empty entries. Stable row IDs are unique per card/collection, cell IDs unique
per row and declared by its exact layout. Repeated targets remain separate occurrences.

## Keyed sorting, hashing and counts

- Preserve v2 keyed ordering inside `cards`.
- `layout_roots` and `layouts` sort by `(world_id, layout_id, version)` using null
  scope before strings, ASCII identity order and numeric revision order.
- `collection_roots`/`collections` sort analogously by collection pin.
- `relations` sort by the v2 qualified CardPin ordering. Exactly one companion
  exists for every exact card in `cards.cards`.
- Reject duplicate keys/roots; multiple old revisions of one identity can be
  dependencies, but root identities are unique/current active local definitions.

```text
C = canonical_compact_utf8_json(content)
content_sha256 = lowercase_hex(SHA256(
  ASCII("karta-design-package-content-sha256-v3\n") || C
))
```

The domain ends with one LF byte. Hash the entire content object, not its field name,
manifest, original whitespace, inspection fingerprint or nested projection checksum.
There is no generation timestamp, actor or authenticity/signature claim.

`root_count` sums typed roots, card roots, field roots, layout roots and collection
roots. `record_count` sums typed records, card records, field definitions, layouts,
collection definitions and relation companions. Components are counted evidence,
not independently identified gameplay objects.

## Structural acceptance and complete closure

The reader validates all v1/v2 intrinsic/native equality checks plus every exact
layout/collection/component and cell. Embedded fields equal indexed v2 definitions;
embedded layouts equal indexed layout records; assignment collection definitions
equal indexed collection records. Sources are exact shared pins from local definitions.
Every relation companion has its exact card; `legacy_unmodeled=true` requires an
unmodeled relation inventory. Standalone cards may legitimately have pre-0010 record
boundaries; this does not alter the frozen scalar-card legacy contract from v2.

Combined traversal derives edges from payloads, never a supplied edge list:

| Node | Required dependencies |
| --- | --- |
| Typed/card/field projections | Existing v2 references, bindings, exact metadata and sources |
| Card | Same-pin relation companion |
| Relation companion | Every exact assignment definition and every referenced cell target |
| Collection definition | Exact embedded layout and optional exact shared source collection |
| Layout | Every exact embedded scalar field and optional exact shared source layout |
| Cell Card/Stat/Field pin | Its exact scoped card/typed Stat/reusable field record |

Roots are existing v2 active catalogs plus all active local layouts/collection
definitions. Required old/archived data remains dependency-only. Cycles are allowed;
bounded visited traversal must reach all and only the included records. External
seeds extend internal projection validation but do not relax root eligibility or
justify unrelated cycles/extras. Recomputed checksums do not authorize invalid
allowlists, numbers, ownership, references, duplicate/missing/extra evidence or
inconsistent embedded data.

The reader cannot prove that a producer enumerated every live root or that a
capture-time binding was really a database head. Producer capture supplies those
guarantees in one authorized world-first ordinary read snapshot. Permission is
rechecked before release; preview binds the v3 content digest and final bytes pass
this actual reader before private/no-store attachment. Partial or unsupported
captures block export, not authored Unknown data.

Combined budgets remain 2048 roots, 8192 records and 65536 authored/reference
occurrences, including repeated/embedded evidence, and **32 MiB input/output**.
Capture retains its 1 MiB/snapshot, 16 MiB accounted snapshot bytes and cooperative
10-second deadline, cancellation cleanup and two-job admission. These are operational
limits, not new gameplay cardinality rules or hard real-time guarantees.

Offline `tools validate-package [path|-]` reads bounded bytes and dispatches 1/2/3.
Diagnostics retain current Move/ruleset policy 2 and saved preset policy 1/2,
separately from structural acceptance. Records do not execute conditions, costs,
effects or Nature adjustments. The database migration, package schemas and validator
policies are independently versioned. Old package files are not rewritten.
