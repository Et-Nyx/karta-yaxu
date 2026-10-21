# ADR0019: Derived values foundation

Status: Accepted for implementation, following explicit maintainer approval.

## Context and alternatives

Designers need inspectable numeric calculations before runtime mechanics. Existing
Stats and numeric fields remain authored exact decimals. A text expression language,
implicit content bindings, floating point, and Lua would introduce unnecessary
semantics. The maintainer selected named sample inputs first and exact fractional
intermediates, with explicit rounding.

## Decision

- Add independent shared/local formula definitions, not new Card namespaces or
  replacements for base Stats. Identity is scope/id; exact pins include revision.
  Shared edits require administrators; local edits require enabled world owners or
  collaborators. No administrator bypass for local content.
- Each formula has normalized metadata, semantics version 1, ordered stable-ID
  named inputs (descriptive units and NumericConstraints), ordered stable-ID named
  steps, and an explicit output operand. Operands are decimal constants, input IDs,
  or strictly earlier step IDs. No external content/formula references or inferred
  bindings. Up to 100 inputs and 200 steps are operational format limits, not game
  limits. Labels/units are inert plain text, at most 100 characters.
- Operations are add/subtract/multiply/divide/minimum/maximum (two operands), clamp
  (value, lower, upper), and round (one operand, scale 0..6, mode floor/ceiling/
  toward_zero/nearest_ties_even). Validate arity and reference order before running.
  Clamp with lower greater than upper is an evaluation error, not a silent swap.
- Use reduced fractions with i64 numerator and positive i64 denominator. Checked
  i128 intermediates, cross cancellation and reduction precede range checks. This
  is bounded exact arithmetic, not arbitrary precision. Overflow is an explicit
  calculation error. No arithmetic or presentation silently rounds. Decimal input
  uses existing canonical Decimal encodings. Rounded results remain fractions;
  display integers or numerator/denominator exactly, optionally an exact terminating
  decimal only when representable. Signed rounding, especially negative halves,
  follows the named mathematical policy.
- Sample inputs are Known(Decimal), Missing, Unknown, or NotApplicable. Omitted
  declared inputs are Missing. Duplicate/undeclared submissions are errors. Validate
  Known values against input constraints; constraints never clamp. Every step is
  evaluated once, in order. Nonnumeric operands block their consumers and retain
  contributing input/step reasons; arithmetic errors are distinct from Unknown.
  No algebraic simplification hides unresolved inputs (including zero times Unknown).
  Valid definitions may produce errors for particular samples and remain saveable.
- Calculator submissions bind an exact formula revision, are read-only public
  bounded POSTs, save no inputs/results/history and require no login or CSRF token:
  no authority or server-side state changes. Responses are private/no-store and
  inputs stay out of URLs. CPU work runs off async handlers with bounded admission.
- Native semantic forms, ID-bound CSRF writes, readable scoped GET/HEAD addresses,
  at most 25 displayed entries per page, one loaded description, raw error/conflict
  retention and explicit conflict review follow existing authoring contracts.
- Migration 0011 adds typed heads/version/input/step/operand tables and immutable
  audit-backed snapshots. Mutation, revision and audit commit atomically, with
  locked authorization and expected revision checked first. Preserve old data,
  snapshots and fixture baselines; startup never migrates or seeds.
- Inspection captures current local formulas including archived definitions in the
  same world-first snapshot under combined budgets. Formula definitions have no
  external dependencies in semantics 1. Fingerprint advances to v4 over public
  formula evidence. Partial captures still have no fingerprint or Unused claim.
- New complete packages use schema 4 wrapping unchanged schema-3 record content
  plus active local formula roots and exact formula definitions. Independent formula
  closure rejects extras/missing/duplicate/foreign/unsupported evidence. Canonical
  hash domain is `karta-design-package-content-sha256-v4\n`. Preserve public v1/v2/v3
  readers unchanged; combined 32 MiB package and capture budgets remain. Final bytes
  must pass the actual shared reader before authorized release. Diagnostics are
  authoring-only; sample evaluation is neither engine nor combat acceptance.

## Consequences and verification

No new dependencies, script execution, live bindings, runtime instances or battle
mechanics. Engine integration and simulation remain last. Examples are original,
documented sample programs; existing seeds are unchanged. Local databases are not
migrated during implementation. Tests cover exact arithmetic/rounding/bounds,
state propagation, structural validation, atomic permissions/history/conflicts,
typed/snapshot corruption, inspection, old/new packages and keyboard workflows at
desktop and 320px. Run fmt, strict Clippy, workspace tests and the browser suite.
