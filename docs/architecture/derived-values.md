# Derived values foundation

ADR0019 is implemented as independent shared/world-local **formula definitions**
and a public sample calculator. Formatting, strict Clippy and 539 Rust cases passed
(538 in the full workspace run plus one subsequent targeted upgrade regression).
All 120 browser cases passed across runs; the full attempt timed out after one hour,
and the remaining Stats/Types modules passed separately. No completed single full
browser run is claimed.
See [ADR0019](../decisions/0019-derived-values-foundation.md) and
[package schema 4](design-package-v4.md).

## Author a formula

Open **Formulas** (`/formulas`) for shared definitions, or **Formulas** on a world
(`/worlds/{world}/formulas`). Shared writes require an enabled administrator;
local writes require an eligible enabled owner/collaborator, without a global
administrator bypass. Browsing, exact history and sample calculation are public.
Formulas are not cards or replacements for authored Stats/Move attributes/fields.

1. Choose **Create formula**, enter a name and description, then **Save operation**.
   Creation starts with no inputs/steps and constant-zero output.
2. Follow **Edit inputs**. **Add entry** generates a stable ID; give each input a
   readable name, optional descriptive unit, optional inclusive minimum/maximum and
   a whole-number constraint. Save before moving to another editing operation.
3. Follow **Edit steps** and add ordered named steps. Choose each operation and
   operand kind: decimal constant, named input, or strictly earlier named step.
   After changing kinds, use **Load named operand choices (keep draft)** to show
   named reference selectors. IDs establish identity; labels do not bind content.
4. Save the steps, then **Edit output** and explicitly choose a constant, input or
   step. Saving steps does not automatically select the last one as output.
5. Follow **Sample calculator (this exact revision)**. Select each input's state,
   enter a decimal for Known values and choose **Calculate sample**. Read the output
   and named step trace; trace pages recalculate the same submitted sample.

Native labeled controls, normal links and buttons work with keyboard navigation.
There is no expression-text parser. Names/units are trimmed inert plain text, at
most 100 characters (names required, units optional). The format allows **100 inputs
and 200 steps**, operational limits rather than gameplay rules. IDs are distinct
within their input/step collections; self/forward step references are invalid.

### Drafts, pages and history

Lists, detail entries, history, sample inputs and trace pages show at most **25**
entries. The step editor shows **five steps** per page. Operation-scoped authoring
loads one description rather than duplicating prose across operations. POST bodies
are limited to **32 KiB**, including sample submissions. Draft-preserving page/add
operations conservatively check encoded size and ask for a save before more draft
data would exceed the transport budget. Not every maximum-size definition fits in
one request; save incrementally. Off-page saved entries remain intact.

Writes are immutable-ID-bound and CSRF-protected. Validation, authorization and
optimistic-conflict errors retain raw drafts. Review the linked newer exact revision
before explicitly saving a reviewed operation against it; no silent overwrite.
Archive/unarchive uses **Review archive change** then **Confirm archive change**.
History retains attributed revisions and exact definitions, including archived ones.
An old revision's calculator continues to calculate that revision after later edits
or archival; it does not follow the current head.

## Exact numeric semantics (version 1)

Constants and Known sample values use the existing exact Decimal: signed i64
coefficient and scale 0–6, with canonical wire encodings. Input constraints validate
Known values; they never clamp them. Calculations use reduced rational fractions:

- Numerator: signed i64, **−9,223,372,036,854,775,808 through
  9,223,372,036,854,775,807**.
- Denominator: positive i64, **1 through 9,223,372,036,854,775,807**.
- Coprime terms, unique zero `0/1`; checked i128 intermediate arithmetic,
  cross-cancellation/reduction before final i64 range checks.

This is bounded exact arithmetic, not arbitrary precision. Intermediates or reduced
results outside the supported range produce explicit Overflow. Neither arithmetic
nor presentation silently rounds: the UI displays an integer or exact `numerator/denominator`.
Rounded results are still reduced fractions, not forced decimal strings.

| Operation | Ordered operands | Meaning |
| --- | --- | --- |
| `add`, `subtract`, `multiply`, `divide` | a, b | a+b, a−b, a×b, a÷b |
| `minimum`, `maximum` | a, b | Exact comparison |
| `clamp` | value, lower, upper | Inclusive bounds; reversed bounds are an error |
| `round` | value | Explicit scale 0–6 and rounding mode |

Only the operation's required operands are saved. Extra editor operand controls
are retained drafts. Arity, supported round settings and all references are checked
before evaluation. Round means rounding to a multiple of `10^-scale`:

| Mode | Rule | −2.5 at scale 0 | −3.5 at scale 0 |
| --- | --- | --- | --- |
| `floor` | Toward negative infinity | −3 | −4 |
| `ceiling` | Toward positive infinity | −2 | −3 |
| `toward_zero` | Discard fractional part | −2 | −3 |
| `nearest_ties_even` | Nearest, exact halves to even | −2 | −4 |

### States and failures

Sample inputs are **Known(Decimal), Missing, Unknown or NotApplicable**. Omitted
declared inputs are Missing; zero is a Known value. Duplicate/undeclared submissions,
malformed decimals and Known constraint violations are submission errors, not
calculation results. Even unused Known inputs must satisfy their constraints.

Each step is evaluated once in authored order, including steps unused by output.
Results are Known exact fractions, Blocked with contributing input/step reasons,
or an explicit calculation error: **Overflow, DivisionByZero,
InvalidClampBounds**. Dependent steps/output preserve the original error and
unresolved step provenance as Blocked; unrelated steps still run. `0 × Unknown`
stays blocked; no algebraic simplification hides unavailable inputs. Valid formulas
remain saveable even when a particular sample divides by zero or overflows.

## Original examples to enter through the UI

These are manual documentation exercises, **not new seeds** or official mechanics.
Use generated IDs and named selectors; retain stable IDs when renaming entries.

### Trail effort

Create **Trail effort sample** in a world. Add inputs **Base effort**, **Tool bonus**
and **Terrain bonus**, units `points`, whole-only true, leaving bounds unspecified.
Save inputs. Add and save these steps:

| Step name | Operation | Operand 1 | Operand 2 |
| --- | --- | --- | --- |
| Equipped effort | add | Input: Base effort | Input: Tool bonus |
| Total effort | add | Step: Equipped effort | Input: Terrain bonus |

Set output to **Total effort**. In the exact calculator choose Known values `10`,
`3`, `5`: the trace shows `13`, then `18`. Change Terrain bonus to Unknown: Total
effort/output are Blocked while Equipped effort stays `13`. This computes a sample
number only; it performs no target check, spending, card lookup or action resolution.

### Shared load and negative rounding

Create **Shared load sample** with an unconstrained input **Load adjustment**. Add
**Per carrier**: divide Input Load adjustment by Constant `2`. Add four round steps
referencing Per carrier, scale `0`, one for each mode in the rounding table. Save,
then choose one as output. Known `-5` yields `-5/2` then `-3`, `-2`, `-2`, `-2` in
the four round steps. Known `-7` demonstrates the ties-even result `-4`. Select
Unknown to see blocked traces without converting it to zero. Change the divisor
to constant `0` in a new revision to inspect a saveable DivisionByZero example;
the prior calculator link still evaluates the old exact revision.

## Read-only calculation and boundaries

Calculator GET/HEAD and bounded POST at
`/formulas/{formula}/history/{revision}/calculate` (also world-prefixed) bind scope,
immutable ID and revision. Public POST needs no login/CSRF because it changes no
authority or server-side state. Inputs stay in the POST body, out of URLs; samples,
results and calculator history are never persisted. Responses use
`Cache-Control: private, no-store`. Evaluation uses bounded admission and blocking
CPU workers outside async handlers. Public availability is distinct from cache policy.

There are **no live content or formula bindings**, formula-to-formula calls, source
adoption/copying, scripts, Lua, modifiers, derived Stat assignments or battle rules.
Units and names cannot infer connections. Inspection captures current local formulas,
including archived definitions, as inventory in the same world-first read snapshot.
Complete fingerprint **v4** includes public formula definitions/roots/head observations;
partial captures have no fingerprint. Semantics 1 has no external formula edges:
inventory membership is not usage, and no consumer links are invented.

## Persistence and operations

Migration **0011_formulas.sql** additively creates typed heads, immutable versions,
ordered inputs/steps/operands and audit-backed snapshots. Authorized mutation,
revision and audit commit atomically with locked permission/expected-revision checks.
Readers check typed data against immutable snapshot/audit evidence. Old snapshots,
heads and fixture baselines are preserved; startup never migrates or seeds.

Back up an existing database with SQLite backup facilities or a quiesced procedure,
stop the server, deliberately run `cargo run -p web-app --bin admin -- migrate`
inside `nix develop`, then restart. Current binaries require all **eleven migrations**.
The local database was **not migrated** for this milestone.
No dependencies or formula fixtures were added: old labs remain **65 objects/69
initial audit events**, Card Workshop **15/16**, Card Relations Workshop **28/29**.

Implementation: `game-data/src/formulas.rs` and `formulas/rational.rs`, web-app
`application/formulas.rs`, `persistence/formulas.rs`, `web/formulas.rs` and
`web/formulas/editing.rs`. Verification uses temporary databases. Human visual,
screen-reader and actual browser-zoom checks remain separate from the keyboard
desktop/320px cases.
