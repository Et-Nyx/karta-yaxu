# Card Workshop: named authoring and original examples

[ADR0017](../decisions/0017-card-workshop-and-authoring-refinement.md) refines the
[card foundation](card-foundation.md) with bounded named selection, exact typed
editing and a separate original example world. Engine integration and simulation
remain the absolute final product steps.

## Install the optional examples

Inside `nix develop`, on a deliberately initialized schema-9 database:

```bash
cargo run -p web-app --bin admin -- list-users
cargo run -p web-app --bin admin -- seed-card-workshop --owner-user-id <immutable-user-id>
```

Select an existing enabled editor/administrator. The command creates no accounts,
does not implicitly select an owner, and is not part of `seed-fixtures`, startup or
migrations. It is trusted local operator tooling: the selected owner is not claimed
to have initiated the import. Creation audits use `bundled-fixture-loader` with
reason `Original Card Workshop example v1 (ADR0017)`.

Open `/worlds/card-workshop`, then **World Cards** or **World reusable card fields**.
The batch creates **15 authored objects**: one world, seven cards and seven reusable
field definitions, with **16 initial audit events** including world access/content.
Existing labs still have their separate 65-object/69-event setup baseline.

| Local card | Native origin | Illustrative content |
| --- | --- | --- |
| Mossrunner | Variant | Creature definition with an exact relationship to Ember |
| Ember | Type | Inert attachment/resource descriptions and world-defined labels |
| Steady Ascent | Move | Action example with target 15 and exact related-card data |
| Trail Kit | Standalone | Roll Bonus +3, exact shared Trail Kit example source |
| Focused Effort | Standalone | Roll Bonus +5 and descriptive additional effect |
| Field Wrap | Standalone | Item example including explicit state distinctions |

The additional shared card is **Trail Kit example**. Reusable fields are shared
**Roll Bonus**, local **Roll Bonus** (explicit shared source), **Target Number**,
**Usage Notes**, **Optional Marker**, **Use Context** and **Associated Card**.
They illustrate Number, Text, Boolean, Choice and CardReference. Choice IDs are
stable; their labels describe Trail, Attachment example and Resource example.

The example `10 + 3 + 5 = 18 against target 15` is authored prose and exact values.
Nothing rolls, adds bonuses, spends resources, applies effects or resolves success.
Names, labels and fields introduce no global card classes or mechanics, and these
original examples are not final-game lore.

## Find and read content

Card catalogs have literal name-substring, exact label, technical origin and archive
filters. Field catalogs have name, kind and archive filters. Shared and world scopes
are explicit. SQL filters apply before stable 25-result pagination; `%`, `_` and
backslashes are literal search text. SQLite supplies ASCII case folding, not a
Unicode search/collation framework. Filters survive page navigation.

Details lead with description, rules text, labels and readable properties. Choices
use labels from the embedded exact field revision; references and sources use names
from their exact pins, with exact-history links. Current renames do not overlay old
names. Displayed references are bounded and deduplicated per request. Technical
identity, revision, origin and archive evidence remain available in a disclosure.

## Assign values by name

1. Open a card and **Edit assignment**.
2. Search **Picker name contains**, using **Find by name → Assignment fields**.
3. Choose a named result. Its scope, exact revision and archive status are shown.
4. Select **Missing**, **Unknown**, **NotApplicable** or **Known value**. The checked
   exact field determines the Known control and constraints, not a submitted kind.
5. Enter a number/text value, select a boolean, or use **Exact field choice options**
   or **Reference cards** in the named picker. Select exact revisions explicitly.
6. Save. Selection/search alone never writes content or follows a latest alias.

Reference candidates are shared or in this card's world. Assignment fields are
same-scope. Named creation sources are shared definitions of the corresponding
origin. Adoption lists revisions of the already pinned shared identity. Archived
retained pins stay visible independently of candidate pages; new selections still
pass authoritative locked eligibility checks.

Exact Choice options paginate at 25 with stable IDs and retained off-page selection.
The first option is never implicitly selected. Known zero, false and empty text
remain distinct from Missing/Unknown/NA, absent assignments and unmodeled/empty
collections. **Advanced exact coordinates and inactive values** remains available
as an optional fallback; ordinary named authoring does not require copying IDs.

## Preserve a draft while selecting

Search, candidate/revision paging, choosing, mapping edits and loading use the same
ID-bound, CSRF-protected editor POST and render the editor without a write/redirect.
Raw numbers, prose, reasons, explicit states, exact pins and starting revisions
remain in the draft. Errors and revoked permissions retain raw input. Unknown and
duplicate fields are rejected; unexpected raw keys are displayed read-only rather
than forwarded as successful controls.

**Load selected assignment** reads the exact starting card revision and exact field
pin. Dirty text, including deliberate clearing to empty, must be saved or explicitly
discarded before loading another value. **Discard text and load selected assignment**
does that explicitly; **Discard raw value** clears only the pending value draft,
not any saved assignment. Loading markers include the exact field and card revision.

One prose value is submitted once per operation. LF/CRLF transport is normalized
without trimming authored text. Server limits count Unicode characters; textareas
do not use HTML `maxlength`, whose UTF-16 units could truncate valid non-BMP text.
The URL-encoded 32 KiB form bound remains. Compact selection/mapping additions have
a conservative next-form byte check; an oversized addition retains the prior draft
and pending selection with a recoverable error, never truncating it.

## Build explicit source mappings

Open **Edit copy** on a sourced local card. Choose a named field from its exact
pinned source, then choose an eligible same-world field whose definition explicitly
sources that shared field identity. Each choice adds/replaces one compact mapping;
named Remove buttons remove it. Search and paging preserve the complete off-page
mapping draft. No matching-by-name or automatic source adoption occurs.

Preview the whole collection replacement and review removed local/unmapped source
assignments. Confirmation binds all mappings, exact inputs and the proposed state;
the application recomputes the proposal under the write lock. The 25-entry display
and 32 KiB stateless transport bounds are not game-wide field-count rules. Compact
manual rows remain available as an advanced fallback. Copying preserves rules text,
labels and the already selected source pin.

## Atomic seed, reruns and compatibility

The seed validates owner and deterministic creation baselines in one
`BEGIN IMMEDIATE`, stages native components at revision 1 with their owning audit,
seals world membership/access and commits once. Complete reruns verify immutable
creation content, options, sources, components and audit provenance and skip all 15
objects without writes. Edited heads, pins, archives, collaboration, accounts and
sessions survive. Partial batches, wrong ownership, scoped slug/ID collisions,
inconsistent creation evidence and late failures reject/roll back the invocation;
the command does not repair or overwrite content.

No schema change, package version change, dependency or hosted resource is needed.
Normal card inspection/where-used and schema-2 export include this world and its
exact shared closure. Old seed scopes and frozen schema-1 reading stay intact.
Implementation: `application/cards/search.rs`, `persistence/card_search.rs`,
`web/cards.rs` and `web/cards/pickers.rs`; the typed immutable workshop creation
manifest is `application/card_workshop.rs`, exposed only through `admin`.

## Verification

Formatting, strict workspace Clippy, **462 Rust tests** and the complete
**104-case keyboard browser suite** passed. Two ordinary-run ignored Rust tests
are browser-owned fixture provisioners. The five new Workshop workflows run at
both desktop and 320px: named typed values/invalid drafts; dirty Unicode text and
explicit discard/load; exact choice/reference names; named source mappings and
adoption; and explicit seed, edited rerun, inspection and checksummed frozen export.

Rust coverage includes filters-before-page, >25 options/mappings, literal query
characters, exact revisions and archived retention, allowlists, single-prose Unicode
budgets, read-only transitions, system attribution and whole-batch rollback/reruns.
Verification used temporary test-owned databases, accounts, downloads and servers,
with debug symbols disabled for memory-constrained builds. The existing local
database was not seeded or migrated by this implementation. Headless keyboard and
320px reflow do not establish human visual, screen-reader or actual zoom assessment.
