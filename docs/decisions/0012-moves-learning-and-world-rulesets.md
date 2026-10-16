# 0012 — Move attributes, learning, presets and world ruleset configuration

Decision [0013](0013-authoring-validation-policy-and-archive-selection.md) adds
policy 2 and explicit policy revalidation while retaining the policy-1 historical
semantics described here, and refines new versus retained archived selections.

## Context and alternatives

The maintainer approved extensible move attributes owned by move definitions,
shared/local moves, independent learnsets, multiple named variant moveset presets,
Unknown values and versioned world ruleset configuration/validation before formula
execution. No fixed attribute vocabulary, acquisition method or four-move rule.
The broader milestone discussion is retained in architecture/roadmap.md.

Fixed accuracy/power columns, current-head dependency lookup, arbitrary entity JSON
and executable imported condition prose are rejected. A read-only architectural
assessment identified independent learning aggregates as the smallest reversible
way to preserve existing creature v1/v2/v3 history without adding another layer of
full creature projections.

## Decision

- Dedicated numeric move-attribute definitions have shared/world scope, immutable
  identity, generated scoped slug, metadata, optional descriptive unit, exact
  Decimal constraints, archive state and optional exact shared source. Reuse the
  approved signed decimal primitive, not creature Stat identities.
- Shared/local moves own exact same-scope attribute definitions and explicit Known,
  Unknown or NotApplicable values. Unmodeled collection, defined-empty and absent
  attribute differ. Known zero is ordinary data. Names have no built-in mechanics.
- Each world has at most one independently versioned ruleset configuration. It
  selects exact local attribute expectations, optional exact local Stat-roster and
  chart context, optional positive moveset maximum, repeat policy (unspecified,
  allow, forbid), and inert interaction/environment notes. Expectations can be
  unmodeled or explicitly defined. Additional constraints and required-known/
  optional/NotApplicable policies are declarative checks, not formulas. Missing
  expectations do not prohibit other attributes.
- World rulesets interpret object data; they do not backfill object values. Base
  Stats retain their exact definitions/roster context. Derived-value execution,
  formulas, conditional rule languages and effects remain separate milestones.
- One independent learnset belongs to a shared-form identity or same-world variant
  identity. Its own revisions seal ordered learning records. A record has a scoped
  stable ID, exact same-scope Move pin, optional method/context/condition text and
  explicit numeric threshold state. Text is inert and thresholds are not assumed
  to mean levels. Multiple records may reference the same Move/revision.
- A local learnset may independently pin a shared learnset. This provenance is not
  the variant's form-source pin. Shared form versions do not capture learnsets,
  variant versions do not capture presets and world revisions do not capture
  rulesets. UI/history must identify these separate exact contexts.
- A variant has multiple named, independently versioned moveset presets, each
  pinning an exact same-owner learnset and same-world ruleset revision. Ordered
  entries reference learning-record IDs within that exact learnset. Repeated
  selections are representable; forbid-repeat validation compares canonical Move
  identity, not acquisition-record identity or Move metadata version. Empty drafts
  are allowed. No default preset, runtime individual or learned repertoire yet.
- Definition, move and learnset adoption is explicit old/local/new/proposed review.
  Attribute copying uses explicit injective shared→local attribute mappings; local
  definitions must have matching shared source identity. Learnset copying uses
  explicit injective shared→local Move mappings and preserves every acquisition
  record separately. Neither names nor duplicate provenance choose a mapping.
  Keep-local and explicitly previewed replacement/copy policies preserve Unknown,
  NotApplicable and unmodeled states; dependencies are never silently advanced.
  Record IDs are scoped to a learnset; copied source IDs may be retained in the
  independent local context. Existing presets stay on their saved learnset pin.
- Well-formed incomplete or ruleset-incompatible drafts can be saved. Malformed
  numbers, intrinsic definition violations, broken references and wrong scope are
  errors. Authoring reports distinguish contradictions, incomplete inputs and
  checks not evaluated. Required Unknown/absent inputs are incomplete; required
  NotApplicable or violated known restrictions contradict the configuration.
  Exact definition-version mismatches are context diagnostics, never implicit
  reassignment. Conditions, formulas and combat eligibility remain not evaluated.
- Reports identify validator policy version 1 and exact inputs, with deterministic
  ordering. They are explicitly **not combat readiness**. Historical validation
  uses exact dependencies and an identified policy, not latest heads; future
  unsupported validator policies must be reported rather than reinterpreted.

## Storage, policy and UI

Use additive migrations 0007 (attributes, moves, rulesets) and 0008 (learning and
presets). Concrete typed version/child tables, exact scoped FKs, singleton/owner
constraints, immutable children and revision-last seals follow Stats/charts.
JSON is a strict immutable snapshot, never the live arbitrary attribute store.
Readers verify snapshots/typed rows/dependencies/audit/predecessors/head agreement;
corruption never becomes Unknown. Existing schema/history bytes are preserved.

Shared editing is administrator-only. Local writes require eligible ownership or
collaboration, checked with the session on BEGIN IMMEDIATE. Expected revisions
precede submitted dependency validation; no-ops add no audit. Every mutation seals
data, complete owning-aggregate history and attribution together. Source changes
and any copied values commit atomically.

Native forms, readable scoped GET/HEAD and canonical-ID POSTs, 32 KiB limits,
bounded page/operation submissions, raw-error retention and explicit conflict
review follow existing patterns. Confirmation binds reviewed inputs/proposals.
Large lists use pagination rather than game-wide cardinality caps. Separate
learning/configuration histories are visible from creature pages.
Editors display at most 25 entries per page/operation. Long prose loads one field
at a time (`text_field`, `text`, `next_text_field`, `load_text`); load before editing,
then save or explicitly discard dirty text before switching. Other texts remain
saved rather than duplicated in hidden fields, preserving Unicode domain limits
within the 32 KiB body bound without truncation. New selections require unarchived
definitions/Moves; retained exact archived pins permit unrelated edits/history.

Synthetic complete-setup fixtures demonstrate unknown/zero/NotApplicable, repeated
learning records, independent worlds and several presets. Existing targeted seed
scopes and prior immutable creation baselines stay unchanged. Reruns preserve edits
and late failure rolls back the entire setup transaction. No new dependency,
infrastructure, external fetch or Lua execution is needed.

## Verification and consequences

Cover pure value/state/report validation; exact scope/owner/version references;
repeated learning records and preset restrictions; explicit source mapping;
revocation/stale/no-op/rollback; sealed/corrupt history; populated 0006 upgrades;
complete setup/reruns; readable addresses and keyboard desktop/320px workflows.
Run fmt, Clippy with warnings denied, workspace tests and the browser suite. Any
test/preview server must stop afterward.

Independent histories make changes explicit and avoid rewriting creature history.
Complete-world export must pin all relevant aggregates separately. A ruleset here
is authored configuration, not an executable battle engine. The dedicated Plan/
Build types are unavailable; read-only assessment and maintainer approval preceded
this record, with bounded general-agent implementation.
