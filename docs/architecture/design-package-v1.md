# Frozen design package v1

**Delivered:** `karta-design-package`, schema version **1**, root policy
`active-world-catalog-v1`, producer `karta-yaxu`.
[ADR0015](../decisions/0015-first-frozen-design-package.md) records the decision;
the [README workflow](../../README.md#frozen-design-packages) describes browser use.

This is a self-contained frozen **design draft**, with a database/network-free
shared Rust reader intended for engine integration. No engine currently consumes
it. Structural acceptance does not mean executable gameplay acceptance or combat
readiness. Formulas, effects, eligibility, scripts, derived stats and battle
execution are not implemented by this protocol.

## Browser production and offline validation

An enabled editor/administrator may export an active world only as its owner or
collaborator. This explicit export policy currently matches world editing
eligibility; global administrator status supplies no bypass and export permission
does not authorize Lua execution/publishing.

- Authorized GET/HEAD `/worlds/{world-slug}/export` opens an entry page only.
- **Preview package** submits a native CSRF-protected POST to
  `/worlds/{immutable-world-id}/export`, with `intent=preview` and `csrf`.
- Review the digest, counts, byte size and separate authoring findings. The page
  previews at most 25 roots and 25 findings, with remaining counts. Full content
  and full diagnostics are not limited to those previews.
- **Confirm and download** submits `intent=download`, `csrf`, the 64-character
  lowercase hexadecimal `reviewed_digest`, and checkbox `confirm=yes`. Forms have
  a 4 KiB body limit and reject unknown/duplicate fields. The application use case
  enforces digest confirmation as well as the web handler.
- Confirmation captures again. A different content digest, or a missing checkbox,
  produces fresh preview HTML requiring another confirmation, not an attachment.
  Permission/session eligibility is checked in capture and rechecked after
  processing before release. Denied, invalid, busy or incomplete captures release
  no partial package.
- Final bytes are built completely in memory and passed through the actual shared
  reader before release. The attachment is `application/json`, with
  `Content-Disposition: attachment; filename="karta-{world-id}-{digest-prefix}.json"`
  (`digest-prefix` is the first 12 content-digest characters),
  `Cache-Control: private, no-store` and `X-Content-Type-Options: nosniff`.
  Pages/errors are also no-store. There is no retained server artifact or public
  package GET URL.

Inside `nix develop`, offline validation is available as:

```bash
cargo run -p tools -- validate-package path/to/package.json
cargo run -p tools -- validate-package - < path/to/package.json
cargo run -p tools -- validate-package < path/to/package.json
```

`validate-package [path|-]` defaults to stdin and reads at most 32 MiB plus one byte
to detect excess before parsing. It uses `game_data::package::Package::from_bytes`,
needs no live database/network and returns nonzero for structural rejection.
Success prints “Accepted design draft”, root/record counts, the content digest,
separate contradiction/incomplete/context-difference/not-evaluated counts and a
structural-only qualification. Findings need not be zero for acceptance. This is
validation only: there is no CLI package creation or database import command.

## Capture, roots and closure

The producer resolves the world as the first read in one ordinary SQLite read
transaction, then checks session eligibility and world permission on that same
connection. It enumerates selected roots and resolves all required dependencies
in that snapshot. The transaction finishes before package-wide validation, diagnostics,
hashing, serialization or rendering. A world content revision alone does not pin
every independently versioned aggregate.

### `active-world-catalog-v1`

Independent roots are:

1. The current unarchived world anchor.
2. Every current unarchived local Type, Stat, move-attribute definition, Move and
   variant, including unused active definitions.
3. The world's current unarchived chart, if present, and configured current Stat
   roster and ruleset, if present. Defined-empty configuration remains configured.
4. Current learnsets and current unarchived presets belonging to active variants.
   Children of archived variants are not independent active-catalog roots.

There is no selected-only export UI. Shared catalog records cannot be roots.
Unconfigured singleton/learning content is not fabricated. Unlike inspection,
export does not independently root every archived local object or serialize
inspection inventory, absences, head observations, findings or edges.

Starting from these roots, the package includes the complete required transitive
closure. Required archived or older exact revisions are valid dependencies; they
are not silently advanced, filtered out or treated as automatically invalid.
Every ordered typing/stat/attribute/roster/acquisition/preset occurrence survives,
including repeated Moves and selections. A record is stored once per exact key;
that does not deduplicate occurrences inside authored arrays. Cycles are legal.

Some content relationships name identities rather than historical revisions:
Species → default Form, Learnset → owning Form/Variant, Preset → owning Variant.
For each required identity, `bindings` supplies one explicit exact target, resolved
from its current head in the same capture snapshot. Multiple referenced versions
of an object share that identity's binding. A binding is a **capture-time
resolution**, not a newly authored historical pin. An older exact pin may coexist
with the bound current revision. Ownership here concerns content objects, never
account/world-access ownership.

Membership observations do not recursively import excluded catalog entries.
A Variant's exact source Form does not implicitly add that Form's independent
Learnset. Explicit Learnset source adoption is independent of the Variant's source
Form; the package adds no new equality restriction between those sources. Names,
slugs and matching provenance never infer source adoption or mappings.

### Required references derived from payloads

The reader derives references itself; the file contains no trusted edge list.
Scope is shared (`world_id=null`) or the package's local world as appropriate.

| Payload | Exact dependencies | Identity bindings required |
| --- | --- | --- |
| World | None (no catalog-membership closure) | None |
| Species | None | Default Form |
| Form | Species at `species_version`; typing Types; base Stat definitions | None |
| Variant | Optional source Form; local typing Types; optional Stat roster; base Stat definitions | None |
| Shared Type | None | None |
| World Type | Optional shared source Type | None |
| Stat | Optional shared source Stat | None |
| Stat roster | Every embedded Stat definition | None |
| Move attribute | Optional shared source attribute | None |
| Move | Optional shared source Move; embedded attribute definitions; mapping local-attribute pins | None |
| Chart | Optional shared source chart; every roster Type | None |
| Ruleset | Optional Stat roster/chart; embedded expectation attribute definitions | None |
| Learnset | Optional shared source Learnset; each acquisition's embedded Move; mapping local-Move pins | Owning Form or Variant |
| Preset | Exact Learnset and Ruleset | Owning Variant |

Source mapping endpoints naming shared identities are resolved within the exact
source aggregate, not through global identity bindings. Chart cells name roster
identities; preset entries name acquisition IDs within the exact pinned learnset.
They do not introduce unrelated catalog/head lookups.

## JSON wire layout

The following type notation specifies **ordered JSON object fields**, not a
permissive object-key map or a JSON Schema file. `T?` means the field is present
with either `null` or a value of type `T`; `[T]` is an array. All fields are
required on the wire, including nullable ones, except the explicitly payload-free
numeric enum forms below. No unknown/duplicate fields are accepted. JSON field
names, enum spellings and field order are case-sensitive.

### Envelope

```text
Package = {
  "manifest": Manifest,
  "content": Content
}
Manifest = {
  "format": "karta-design-package",
  "schema_version": 1,
  "producer": "karta-yaxu",
  "content_sha256": string,
  "root_count": integer,
  "record_count": integer
}
Content = {
  "schema_version": 1,
  "root_policy": "active-world-catalog-v1",
  "world": Key,
  "roots": [Key],
  "records": [Record],
  "bindings": [Binding]
}
Record = {"key": Key, "payload": Payload}
Binding = {"identity": Identity, "target": Key}
Key = {"kind": Kind, "world_id": ID?, "id": ID, "revision": Revision}
Identity = {"kind": Kind, "world_id": ID?, "id": ID}
```

`ID` is exactly 32 lowercase hexadecimal characters, independent of names/slugs
and external fixture IDs. `Revision` is a positive signed 64-bit integer. The
world anchor has `kind="world"` and `world_id=null`; local keys carry the world's
ID, shared keys use `null`. Records outside this one world/shared scope are invalid.
Manifest counts equal the lengths of `content.roots` and `content.records`.

`Kind` spellings and their **sort rank** are, in order:

```text
world, species, form, variant, type, stat, stat-roster,
move-attribute, move, chart, ruleset, learnset, preset
```

This is enum declaration order, **not alphabetical string order**. Keys compare
lexicographically by `(kind rank, world_id, id, revision)`; identities compare by
`(kind rank, world_id, id)`. Nullable scope sorts `null` before any string; strings
use lexicographic order (IDs are ASCII); revisions compare numerically. Roots are
strictly sorted by key, records by their `key`, bindings by their `identity`.
Duplicate roots/keys/bindings are rejected. Root revisions of one identity cannot
repeat, although closure records may contain several revisions of that identity.

### Payload tags and record fields

`Payload` is an externally tagged object with **one field**, for example
`{"world": World}` or `{"world-type": WorldType}`. These are the complete v1 tags
and ordered fields; the field types are expanded below.

```text
"world": {
  "id": ID, "fields": Fields, "revision": Revision, "archived_at": i64?
}
"species": {
  "id": ID, "fields": Fields, "revision": Revision,
  "default_form_id": ID, "archived_at": i64?
}
"form": {
  "id": ID, "species_id": ID, "species_version": Revision,
  "fields": Fields, "revision": Revision, "is_default": bool,
  "archived_at": i64?, "typing": Typing, "stats": BaseStats
}
"variant": {
  "id": ID, "world_id": ID, "source": FormPin?, "fields": Fields,
  "revision": Revision, "archived_at": i64?, "typing": Typing, "stats": BaseStats
}
"type": {
  "id": ID, "fields": Fields, "revision": Revision, "archived_at": i64?
}
"world-type": {
  "id": ID, "world_id": ID, "source": TypePin?, "fields": Fields,
  "revision": Revision, "archived_at": i64?
}
"stat": {
  "id": ID, "world_id": ID?, "source": StatPin?, "fields": Fields,
  "revision": Revision, "archived_at": i64?, "constraints": Constraints
}
"stat-roster": {
  "id": ID, "world_id": ID, "revision": Revision, "entries": [Stat]
}
"move-attribute": {
  "id": ID, "world_id": ID?, "source": AttributePin?, "fields": Fields,
  "unit": string, "revision": Revision, "archived_at": i64?,
  "constraints": Constraints
}
"move": {
  "id": ID, "world_id": ID?, "source": MovePin?, "fields": Fields,
  "revision": Revision, "archived_at": i64?,
  "attributes": [AttributeValue]?, "mappings": [AttributeMapping]
}
"chart": {
  "id": ID, "world_id": ID?, "fields": Fields, "revision": Revision,
  "source": ChartPin?, "roster": [TypeAssignment], "cells": [ChartCell],
  "mapping": [ChartMapping], "archived_at": i64?
}
"ruleset": {
  "id": ID, "world_id": ID, "revision": Revision, "fields": Fields,
  "expectations": [Expectation]?, "stat_roster": RosterPin?, "chart": ChartPin?,
  "max_moves": i64?, "repeat_policy": RepeatPolicy, "notes": string
}
"learnset": {
  "id": ID, "world_id": ID?, "form_id": ID?, "variant_id": ID?,
  "source": LearnsetPin?, "revision": Revision,
  "records": [Acquisition]?, "mappings": [MoveMapping]
}
"preset": {
  "id": ID, "world_id": ID, "variant_id": ID, "fields": Fields,
  "revision": Revision, "archived_at": i64?, "learnset": LearnsetPin,
  "ruleset": RulesetPin, "entries": [ID], "validator_version": i64
}
```

`Stat`, `MoveAttribute` and `Move` below mean the **untagged record bodies** above,
not another externally tagged `Payload`. Key kind `type` covers both `type` and
`world-type` payload tags, distinguished by scope. Each other payload tag has its
corresponding key kind. Key identity/scope/revision must exactly match its payload.

There is no generated-at timestamp, account actor or private ownership metadata.
Existing public `archived_at` nullable integer archive fields are retained, including
on archived dependencies; “no generation timestamp” does not remove these fields.
The envelope is separate from database migration versions and historical creature
v1/v2/v3 projections; it contains public typed records, never raw snapshot JSON.

### Nested records and pins

```text
Fields = {"slug": string, "display_name": string, "description": string}
FormPin = {"form_id": ID, "version": Revision}
TypePin = {"type_id": ID, "version": Revision}
StatPin = {"stat_id": ID, "version": Revision}
RosterPin = {"roster_id": ID, "version": Revision}
AttributePin = {"attribute_id": ID, "version": Revision}
MovePin = {"move_id": ID, "version": Revision}
ChartPin = {"chart_id": ID, "version": Revision}
RulesetPin = {"ruleset_id": ID, "version": Revision}
LearnsetPin = {"learnset_id": ID, "version": Revision}

TypeAssignment = {"pin": TypePin, "fields": Fields}
Typing = {"assignments": [TypeAssignment]?}
Constraints = {"whole_only": bool, "minimum": Decimal?, "maximum": Decimal?}
StatValue = {"definition": Stat, "value": Decimal?}
BaseStats = {"roster": RosterPin?, "assignments": [StatValue]?}
AttributeValue = {"definition": MoveAttribute, "value": NumericValue}
AttributeMapping = {"shared_attribute_id": ID, "local_attribute": AttributePin}
Factor = {"numerator": u32, "denominator": u32}
ChartCell = {"attacker_type_id": ID, "defender_type_id": ID, "factor": Factor}
ChartMapping = {"source_type_id": ID, "local_type_id": ID}
Expectation = {
  "definition": MoveAttribute, "required": bool,
  "allow_not_applicable": bool, "constraints": Constraints
}
Acquisition = {
  "id": ID, "move_record": Move, "method": string?, "context": string?,
  "condition": string?, "threshold": NumericValue
}
MoveMapping = {"shared_move_id": ID, "local_move": MovePin}
```

- `NumericValue` has exactly three wire forms:
  `{"state":"known","value":"-1.25"}` (any canonical Decimal),
  `{"state":"unknown"}`, or `{"state":"not_applicable"}`. Unknown/NA must
  **omit** `value`; even `"value":null` is invalid. Known requires its value.
- `RepeatPolicy` is the JSON string `"unspecified"`, `"allow"` or `"forbid"`.
  `max_moves` is `null` or a positive integer; no maximum means no cap. Saved
  `validator_version` is exactly 1 or 2, independently of package schema version.
- `Decimal` is a **JSON string**, not a JSON number or coefficient/scale object.
  It represents `coefficient × 10^-scale`, with a signed 64-bit normalized
  coefficient and scale 0–6. Canonical spelling has no leading `+`, no redundant
  integer leading zeroes, no trailing fractional zeroes, and no decimal point
  for integers; negative zero is `"0"`. Examples: `"0"`, `"-1.25"`, `"0.000001"`.
  Exponents, non-finite values, overflow, excess scale and noncanonical strings
  such as `"1.0"` or `"+1"` are rejected, never rounded.
- Factors are reduced exact fractions: numerator 0–1,000,000, denominator
  1–1,000,000, with zero written `0/1` and neutral `1/1`. Cells contain only known
  factors. Missing in-roster pairs mean Unknown; out-of-roster is a separate lookup
  state, not a cell/tag. Lookup uses scoped canonical Type identity across metadata
  revisions; no multi-Type composition or damage rule is implied.
- Nullable collections distinguish unmodeled (`null`) from defined-empty (`[]`).
  Missing Stat values (`null`), absent Move attributes, explicit NumericValue
  Unknown/NA and known zero remain distinct. Nullable acquisition prose is inert,
  and thresholds have no implicit level meaning.
- All nested arrays retain stored/authored order and multiplicity. Do not sort
  typing, rosters, values, expectations, acquisitions or selections to obtain a
  digest. Existing intrinsic chart constraints still require cells strictly sorted
  by `(attacker_type_id, defender_type_id)` and chart mappings strictly sorted by
  `source_type_id`; these arrays are not arbitrarily reordered during packaging.

Metadata must already be normalized and valid; parsing rejects rather than edits
it. Slugs have 1–80 lowercase ASCII letters/digits with single separating hyphens;
display names have 1–100 Unicode characters, descriptions 1–2000, with no outside
whitespace. Descriptions use LF paragraphs; other control characters are invalid.
Acquisition prose, when present, has 1–2000 trimmed characters with only LF controls.
Units are trimmed inert text of at most 100 UTF-8 bytes without controls; ruleset
notes allow at most 2000 trimmed UTF-8 bytes with LF paragraphs. Empty unit/notes
strings are permitted. These are validation rules, not Unicode normalization or
name-based identity resolution.

## Canonical bytes and SHA-256

Canonical output is compact UTF-8 JSON, no BOM, inter-token whitespace or trailing
newline, emitted with the object fields in the order above. This is the v1
`serde_json::to_writer` representation, **not** alphabetical key sorting or a
general-purpose JSON canonicalization standard.

Strings use literal UTF-8 except for the escapes below, without Unicode normalization.
Escape `"` as `\"`, backslash as `\\`, and backspace/tab/LF/form-feed/CR as
`\b`, `\t`, `\n`, `\f`, `\r`. Other U+0000–U+001F characters use lowercase
`\u00xx` escapes. Do not escape `/`, printable non-ASCII characters or HTML
characters. Integers use ordinary base-10 digits, a minus only for negative values,
no leading zeroes (except zero itself), fractions or exponents. Booleans and null
are `true`, `false`, `null`. Decimal numbers remain canonical strings as above.

Given validated, canonically sorted content, compute:

```text
C = canonical_compact_utf8_json(content)
D = SHA256(ASCII("karta-design-package-content-sha256-v1\n") || C)
manifest.content_sha256 = lowercase_hex(D)  # exactly 64 characters
```

The marker ends with **one LF byte `0x0a`**, not the two characters backslash/n;
there is no NUL terminator. Hash the complete content object from its `{` through
its `}` (including schema, policy, world, roots, records and bindings). Do not hash
the `"content":` field name, manifest, entire envelope, file whitespace, inspection
fingerprint or a database representation. The fixed producer/format and matching
manifest counts are validated separately. No timestamp or actor is added to the
hash domain. Identical content and producer yield identical canonical file bytes.

The reader accepts JSON whitespace (space, tab, CR, LF) outside strings, including
pretty-printing, but otherwise requires **exact canonical tokens** for the entire
envelope. It parses, validates, reserializes and compares bytes after removing only
that outside-string whitespace. Consequently reordered object fields, omitted
nullable fields, alternate string escapes (for example `\u0061` for `a`), duplicate
fields, floats/exponents in integer fields and alternate Decimal encodings are
invalid even when a generic JSON parser would consider the values equivalent.
For a pretty-printed input, the digest is still over `C`, never its original bytes.

This is a checksum, not a signature or authenticity proof. Recomputing it after
tampering does not bypass structural/closure validation, but a malicious producer
can still construct a different structurally valid draft.

## Reader acceptance and diagnostics

The reader checks supported format, producer, both schema versions, root policy
and saved preset policies. It rejects malformed keys, inconsistent counts/digests,
noncanonical tokens, duplicate/extra/missing records or bindings, wrong scopes,
invalid normalized text and intrinsic numeric/collection violations. In addition:

- The world anchor is an unarchived root; all declared roots are eligible local
  active records (except the world anchor's null scope), with one root revision
  per identity. Chart/roster/ruleset roots are singletons. Root learnsets/presets
  require their bound active Variant to be a root.
- Every required exact reference and identity target must exist. Bindings must
  target the same kind/scope/ID, be required by a content relationship and have no
  unrelated extras. Traversal from roots must reach **all and only** records in the
  file; cycles do not require topological ordering.
- Embedded Type metadata, Stat/attribute definitions and acquisition Moves must
  equal their separate exact records. Local Stat assignments must match their
  complete exact roster in order. Shared Forms cannot pin world rosters.
- Default Forms must belong to the corresponding Species and have consistent
  default status. Form/Learnset/Preset ownership is consistent across included
  revisions. There is only one independent local singleton identity per kind and
  one Learnset identity per owner, though several revisions may be dependencies.
- Source mappings are explicit/injective, refer to identities in the exact source
  aggregate, and local mapping endpoints explicitly source those shared identities.
  Presets pin same-owner Learnsets and same-world Rulesets; every selected
  acquisition ID exists in the exact Learnset. Repeated selections are preserved.

The offline reader can verify declared roots' eligibility and closure, not that a
producer enumerated every current root of a database it cannot see, nor that a
capture-time binding really was a database head. Consistent complete enumeration
and snapshot resolution are producer responsibilities. It cannot certify origin.

Authoring diagnostics are recomputed from accepted content, not trusted serialized
reports. Current Move/ruleset checks use policy **2**; saved preset reports retain
policy **1 or 2** and exact inputs. Contradiction, Incomplete, ContextDifference and
notEvaluated are separate from structural rejection. Well-formed diagnostic drafts
are permitted, including Unknown required inputs and contradictory authored
expectations. Invalid references/numbers and intrinsic violations are errors;
unsupported schema/policy, required read failure or capture budget gaps block
production. There is no partial-package/fingerprint fallback. A package has no
inspection latest-head observations and never reuses an inspection fingerprint.

## Operational and compatibility boundary

| Bound | Default/contract |
| --- | --- |
| Selected roots | 2,048 |
| Exact records | 8,192 |
| Identity bindings | At most 8,192, also counted in occurrence work |
| Reference/authored occurrence budgets | 65,536; repeated and embedded occurrences count, not just distinct IDs |
| Preflighted snapshot | 1 MiB each |
| Accounted snapshot bytes | 16 MiB total |
| Connection acquisition, world anchor and capture | Cooperative 10-second deadline |
| Complete package input/output | 32 MiB, including envelope on output and whitespace on input |
| Export admission | Two jobs per service, held through validation/serialization/preview render |
| Rendered root/finding previews | At most 25 each, with remaining counts |

Capture reuses checked inspection readers, preflight, caches, SQLite progress
interruption (about every 1,000 VM operations), handler removal/rollback and
close-on-drop cancellation-safe service leases. Identity resolution shares checked
caches and budget accounting; even current/companion verification needed to read
an older pin can consume work. Nested existing-reader verification is not a perfect
cost meter. Five-second SQLite busy waits, filesystem and non-VM work can outlast
the cooperative deadline; this is not a hard real-time or whole-request CPU limit.
Package validation, diagnostics, hashing and serialization use blocking workers,
with admission retained even if the awaiting request is cancelled during that
work. Export admission is separate from inspection's two-job admission. These are
operational bounds, not new gameplay cardinality rules or user-tunable query caps.

Export adds no content/account mutations, audit events, migrations, fixture objects,
hosting or external dependencies. Existing complete fixtures remain **65 objects/
69 initial audit events**; historical creature bytes, object heads and source pins
retain their meaning. Private world ownership/access data, users/profiles, sessions,
credentials, actors/grants, raw audits and raw snapshot JSON are excluded from DTOs,
hashes and diagnostics. Saved files remain frozen after later live edits.

The v1 public enums and payload representations reused from `inspection.rs`,
`types.rs`, `stats.rs`, `moves.rs`, `charts.rs`, `rulesets.rs` and `learning.rs` are
**locked parts of this package wire contract**. Their names, field order, enum tags,
nullable fields, numeric encodings and reference meanings cannot be changed merely
because their Rust types are shared. Future changes require explicit package
compatibility review/version decisions. Package schema, root policy, validator
policy and database/historical snapshot versions remain separate concepts.

Implementation entry points: `crates/game-data/src/package.rs`,
`package/references.rs`, `package/validation.rs` (shared contract/reader),
`crates/web-app/src/application/export.rs` (policy/admission/confirmation),
`src/persistence/inspection.rs` (checked capture), `src/web/export.rs` and
`templates/export.html` (native workflow), and `crates/tools/src/main.rs`
(offline validation).

Actual engine integration/gameplay acceptance, a minimal executable contract,
schema migration tooling, selective root export, signatures and retained/atomic
publication remain future work. Engine integration/minimal execution is a proposed
direction for discussion, not a newly approved milestone; see the [roadmap](roadmap.md).
