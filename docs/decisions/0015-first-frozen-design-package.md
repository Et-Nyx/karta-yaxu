# 0015 — First frozen design-content package

## Context and alternatives

After ADR0014 the maintainer approved a first frozen package: a shared offline
Rust reader, active world catalog roots, export access for eligible owners and
collaborators, and preservation of well-formed diagnostic drafts. There is no
engine importer in this workspace. A database dump, serialized inspection report,
or claim of executable combat readiness would not establish an engine contract.

The dedicated Plan agent is unavailable. A read-only architecture assessment used
the general agent before implementation; it identified identity-only owner/default
references, embedded snapshot equality, and membership versus content closure as
important boundaries. Selected-only roots and separate export grants were
considered; the maintainer selected active catalog and owner/collaborator policy.

## Decision

- Introduce a separately versioned `karta-design-package` v1 JSON envelope. Its
  dedicated manifest/content DTOs reuse the explicit public record representations
  from the domain, pinned as part of this wire contract. Changes to those wire
  representations require package compatibility review; the inspection contract
  and its fingerprint are not the package schema or identity.
- Roots are one active world anchor, current unarchived local definitions,
  variants and chart, configured roster/ruleset, and current learnsets/unarchived
  presets of active variants. Archived variants' children are not independent
  active-catalog roots. Required archived/older revisions remain dependencies.
  Membership observations never recursively import excluded catalog entries.
- Resolve required exact pins transitively, preserving every ordered entry and
  repeated selection. Freeze identity-only owners/default Forms with explicit
  identity-to-exact-key bindings, resolved in the same read snapshot. These are
  capture-time resolutions, not newly authored historical pins. Cycles are legal;
  no implicit source Form learnset or name-based adoption is introduced.
- Capture selected roots and closure using checked readers in one ordinary SQLite
  read transaction. The world is the first read. Session eligibility and explicit
  world export permission are checked on that connection; content is materialized
  before validation, diagnostics, serialization or rendering. Recheck permission
  before releasing a download. No global administrator bypass, browser actor IDs,
  content changes, account changes, cached packages or export audit schema.
- An active world may be exported by an enabled editor/administrator who owns it
  or is its collaborator. This explicit export policy currently has the same
  eligibility as editing; Lua execution/publishing is not authorized by it.
- Shared `game-data::package` parsing is database/network-free. Strictly validate
  supported format/policies, exact keys, normalized metadata, intrinsic numeric
  constraints, cross-record embedded equality, scopes, ownership, source mappings,
  acquisition selections and full declared closure. Do not trust a supplied edge
  list. Reject missing/extra/duplicate records or bindings and malformed input.
  The reader verifies declared root eligibility, not completeness against a DB it
  cannot see. Consistent complete root enumeration is a producer responsibility.
- Preserve Missing/Unknown/NotApplicable/zero and unmodeled/defined-empty, saved
  preset policies 1/2, exact decimal strings and rational chart factors. Well-formed
  contradictions/incomplete/context-difference/not-evaluated findings are separate
  authoring diagnostics, not structural rejection or readiness certification.
  Unsupported schema/policy, invalid data, required read failures and budget gaps
  prevent export. Existing explicit Learnset source adoption remains independent
  of a Variant's source Form; do not add a new source-equality restriction.
- Sort keyed roots/records/bindings deterministically; preserve meaningful nested
  array order. SHA-256 covers canonical content bytes with a package-specific
  domain marker, never an inspection hash. Manifest records format/schema,
  producer, content digest and counts. No generation timestamp or actor is needed
  in v1; identical content and producer yield identical file bytes. Reuse existing
  workspace `sha2`/`hex` in the shared reader for checksum verification, introducing
  no new external libraries or crypto implementation.
- Validate final serialized bytes through the actual shared reader before making
  an attachment available. Build a complete single JSON file in memory; no partial
  server-side file or streaming serialization is published. Checksums detect
  content mismatch, not authenticity. No retained/downloadable server artifact.
- Native `/worlds/{world}/export` GET/HEAD is an authorized entry page, not package
  creation. ID-bound CSRF-protected POST preview and confirmation recapture and
  bind confirmation to the package digest. Changed package inputs show a fresh
  preview requiring confirmation. Each confirmed request checks permission again.
  Response attachments are private/no-store, with a fixed canonical-ID filename.
- Offline `tools validate-package <path>` reads bounded bytes and uses the same
  reader, reporting structural acceptance and separate diagnostic counts. It does
  not use the live database or imply engine gameplay acceptance.
- Reuse capture operational bounds (2048 roots, 8192 nodes, 65536 occurrences,
  1 MiB/snapshot, 16 MiB accounted snapshot bytes, cooperative 10 seconds), with
  package input/output capped at 32 MiB. At most two export jobs per service,
  admission held through validation/serialization/render. CPU work uses blocking
  workers. Identity closure shares checked caches and budget accounting. These
  operational limits do not add game cardinality constraints.

## Consequences and acceptance

No migrations, fixture objects, hosting, scripts, new external dependencies or
live database changes. Package roots and exact bindings survive later edits.
Private ownership/access/account/session/audit data and raw snapshot JSON never
enter package DTOs, hashes or diagnostics. Existing fixture totals remain 65
objects/69 initial audit events. Existing historical source adoption is preserved.

Cover deterministic offline round trips and corrupt/recomputed-checksum inputs,
root selection, archived dependencies, repeated acquisitions/selections, draft
states and saved policy compatibility; transactional coherence, authorization,
revocation and stale preview; no writes; public/privacy behavior and keyboard
preview/download at desktop/320px. Run fmt, Clippy, workspace and browser checks.

The reader is the first shared parsing contract intended for engine integration.
No engine currently consumes it; actual engine integration/gameplay acceptance,
package migration tooling, signatures, retained publication, selective roots and
executable rules/scripts remain later work. Do not label this a playable package.
