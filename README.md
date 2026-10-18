# Karta Yaxu

**Karta Yaxu (Monster Cards in Globasa)** is a creature collector with deck-building,
supported by a design wiki built with Rust and server-rendered HTML. Worlds interpret
card roles, including whether Types are attachments or resources. The local app
authors design definitions, distinct from eventual player-owned instances, in a
versioned shared species/form library
and independently owned game worlds, with scoped collaboration, multiple local
variants, versioned Types, ordered typing, effectiveness charts, independent Stats,
designer-defined move attributes and Moves, world ruleset configuration, learnsets
and named moveset presets. Public read-only world inspection exposes authoring
diagnostics and scoped dependencies. Authorized world export produces frozen,
self-contained design-content packages with a shared offline Rust reader. Exact
pins, explicit source adoption and attributed history keep these contexts
independent. Explicit fixture commands provide sample authoring worlds, not an
executable game.

The **Card Definition Foundation is delivered**.
Forms, variants, Types and Moves are cards at their native qualified identities and
owning revisions, alongside standalone cards and reusable typed fields. See
[Card foundation](docs/architecture/card-foundation.md). **Engine integration and
the battle simulator are the absolute final steps**, after content/world authoring
and fine-tuning; instance and deck-building execution are not implemented.

## Run locally

Requires Nix with flakes enabled on Linux. From the project root:

```bash
nix develop
cargo run -p web-app --bin admin -- migrate
KARTA_DEV_AUTH=1 cargo run -p web-app --bin admin -- seed-fixtures
KARTA_DEV_AUTH=1 cargo run -p web-app
```

On a fresh database, setup creates the development administrator **`admin`** and
all bundled content in one transaction. Open **http://127.0.0.1:3000** and sign in
at `/login` as `admin`; no prior browser sign-in or manual invitation is needed.
To choose a different bootstrap username, use this instead of the seed command:

```bash
KARTA_DEV_AUTH=1 cargo run -p web-app --bin admin -- seed-fixtures --admin-subject alice
```

Open **Types** (`/types`) for Fire/Flying/Dragon and **Worlds** (`/worlds`) for
ClassicLab/VariantLab. **Charts** (`/charts`) shows the shared synthetic lab chart;
each world has a **View or create world chart** link to its independent chart.
**Stats** (`/stats`) provides independent numeric definitions; each world has a
**World stat roster** and **World stat definitions** link. Complete setup includes
three-stat ClassicLab and four-stat VariantLab examples, plus StatsSample under
Emberling and a sample variant in each lab. **Moves** (`/moves`) and **Move
attributes** (`/move-attributes`) provide shared definitions; each world has local
catalogs and its own **Ruleset**. The StatsSample owners also have independent
learnsets and two named presets per variant, illustrating incomplete and
contradictory drafts. Follow **Authoring inspection** on either world's detail page
or open [/worlds/classiclab/inspection](http://127.0.0.1:3000/worlds/classiclab/inspection)
or [/worlds/variantlab/inspection](http://127.0.0.1:3000/worlds/variantlab/inspection).
See [Moves, learning and presets](#moves-learning-and-presets) and
[World inspection and where used](#world-inspection-and-where-used). Eligible owners
and collaborators can follow **Export package** on a world to preview and download
a frozen design draft; see [Frozen design packages](#frozen-design-packages).
Setup prints the sample-world owner and, when created, the
bootstrap sign-in name. Existing databases keep their fixture owner or use the
sole enabled administrator; ambiguous ownership needs `--owner-user-id`.
The server reads the database and does not automatically load fixture files.
See [World fixtures](#world-fixtures) for ownership, targeted commands and reruns.

For original card-authoring examples, explicitly install the optional **Card
Workshop** with an existing eligible owner:

```bash
cargo run -p web-app --bin admin -- list-users
cargo run -p web-app --bin admin -- seed-card-workshop --owner-user-id <immutable-user-id>
```

Open `/worlds/card-workshop/cards` for Mossrunner, Ember, Steady Ascent, Trail Kit,
Focused Effort and Field Wrap. Named field/reference/source selectors, typed Known
controls and searchable catalogs support editing without copying opaque IDs.
Workshop setup is a separate atomic 15-object/16-event batch and never runs with
old seeds, startup or migrations. Reruns preserve edits. See the
[Card Workshop guide](docs/architecture/card-workshop.md).

Stop with Ctrl+C. SIGINT and SIGTERM trigger graceful shutdown. The first build
needs network access for pinned packages and crates; normal use and tests do not
fetch external content.

## Authentication (development placeholder)

Authentication is a **development placeholder and is not secure**. It uses no
passwords: anyone who can reach the server can sign in as any invited user. It is
disabled unless `KARTA_DEV_AUTH=1` is set, and it must not be exposed beyond local
development. Other accounts are invite-only: an operator invites a subject before
first sign-in, and the account is created on that first sign-in. The explicit
`seed-fixtures` development bootstrap is the exception: with no users at all and
development authentication enabled, it creates an administrator and records a
consumed invitation. It creates no session; sign in normally. A matching pending
administrator invitation is consumed; a lower-role invitation causes an error.

The code keeps authentication behind an application-layer identity-provider port.
A future Polycentric identity provider implements this port and adds its own
credential-specific browser sign-in flow. Preserving existing local user ids
requires an explicit identity-mapping procedure; the placeholder adapter test
does not implement that migration. See decisions
`0003-authentication-foundation.md` and `0004-authentication-refinement.md` in
`docs/decisions/`.

Accounts and sessions are persistent. Disabling an account permanently revokes
all its sessions in the same transaction; re-enabling restores none of them.
Sign-in must be repeated. Invitations only authorize first sign-in: re-inviting
an existing account is rejected and does not change its role.

Authentication forms have a **4 KiB** body limit. Login POSTs share **30 attempts
per 60 seconds** across the whole server process, including rejected attempts.
Exhausting the allowance returns `429` with `Retry-After`; wait up to one minute
before retrying. Account pages and logout remain available. The allowance resets
on restart and is deliberately process-wide for the local preview.

Database failures return temporary-unavailability errors rather than pretending
the user signed out. If logout cannot revoke the session, it preserves cookies
and offers a retry button; it does not claim logout succeeded.

### Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `KARTA_BIND` | `127.0.0.1:3000` | Listen address (IPv4 loopback by default). |
| `KARTA_DATABASE_URL` | `sqlite://data/karta.db` | SQLite path; only the explicit `migrate` command creates it. |
| `KARTA_PUBLIC_ORIGIN` | `http://127.0.0.1:3000` | Expected `Origin` for form POSTs. |
| `KARTA_COOKIE_SECURE` | off | Set `1` to add `Secure` to cookies (needed under HTTPS). |
| `KARTA_SESSION_TTL_SECONDS` | `604800` | Session lifetime: 1–2,592,000 seconds (30 days maximum). |
| `KARTA_DEV_AUTH` | off | Set `1` to enable the placeholder provider. |

Keep the database file out of tracked source (`/data/` and `*.db*` are ignored).
Cookies are `HttpOnly` and `SameSite=Strict`; `Secure` is off only for local HTTP.

## Authoring workflow

| Action | Access |
| --- | --- |
| Browse library, worlds, content history and read-only world inspection/where used | Public local preview |
| Create a world | Editor or administrator; creator becomes its owner |
| Create/edit shared species, forms, Types, charts, Stats, move attributes, Moves, form learnsets, cards and reusable card fields | Administrator |
| Edit a world, its variants, local Types, chart, Stats/stat roster, move attributes/Moves, ruleset, variant learnsets/presets, cards or reusable card fields | Owner or collaborator with editor/administrator eligibility |
| Preview/download a frozen design package for an active world | Enabled owner or collaborator with editor/administrator eligibility |
| Grant/revoke collaboration and inspect access history | World owner |

Administrator status does not bypass another world's ownership. Accounts remain
invite-only; collaborators are existing enabled accounts, not new identity invites.
Users can find their immutable user id on `/account` and share it with the owner.
Disabling an owner preserves ownership and history. Ownership transfer/recovery
is a future explicit operation; it is not silently performed by account disabling.

1. Sign in as your administrator (`admin` after default bootstrap), open
   **Creatures**, and edit/create shared species or forms.
   A new species automatically creates its default form. Every successful change
   adds an immutable definition version and audit event in the same transaction.
2. To try a separate editor, run `cargo run -p web-app --bin admin -- invite-user
   --subject bob --role editor`, then sign in as `bob` and open **Worlds → Create
   world**. Add variants from a concrete
   shared form/version, or choose world-exclusive content. Multiple variants may
   use the same form/version; local slugs distinguish them within the world.
3. Edit a variant's effective name/description without changing the library or
   another variant. Each form version also pins its exact species version.
4. After an administrator edits a source form, follow **Review adoption of version
   N** from a variant. Compare old source, local values and new source; explicitly
   confirm the effective fields and new pin. Untouched variants/worlds stay pinned.
5. Follow history links for actor, UTC time, reason, readable field changes and
   complete before/after snapshots. Public world snapshots omit access details.
6. The owner can manage collaborators using their immutable account ids. Access
   history is private and has a separate revision counter from content.

### Cards and reusable fields

Open **Cards** (`/cards`) or **Reusable fields** (`/card-fields`), or the matching
world catalogs. Concrete Forms, Variants, Types and Moves are cards at the **same
qualified identity and owning revision**, without wrapper IDs. Species remains a
grouping. Standalone cards allow Item/Ability design without fixed gameplay classes.
Native metadata remains authoritative; every native writer carries the component
forward and seals it with the same revision/audit event atomically.

Select ordered distinct same-scope field definitions at exact revisions: **Number**
(exact decimals/constraints), **Text** (0–2000 characters, LF plain text), **Boolean**,
**Choice** (stable option IDs) or **CardReference** (exact shared/same-world card).
Missing, Unknown and NotApplicable differ from an absent field, unmodeled or
defined-empty collection, known empty text, false and zero. Labels and optional
rules text are inert; names never implement mechanics. Definitions/values are not
automatically advanced or merged with existing Stats/Move attributes.

Public origin-qualified native links include `/cards/form/{id}` and world-local
`/worlds/{world}/cards/variant/{id}`; standalone cards and fields also have readable
scoped slug aliases. Operation forms load one assignment/option/label/prose edit,
retain raw validation/authorization/conflict drafts, use ID-bound CSRF POSTs within
32 KiB, and show lists/history at 25 per page. Exact attributed history and native
metadata/full-history links retain both specialized and custom context. Standalone
card/field archive changes require preview/confirmation. Field adoption defaults
keep-local; copying from a pinned shared card is a separate confirmed whole-field
replacement with explicit injective mappings and removed/unmapped previews.
Additional mapping rows use `shared_field_id local_field_id revision`, one per line.
See [the foundation contract](docs/architecture/card-foundation.md) for details.

### Types and ordered typing

Use **Types** to author shared definitions. World owners/collaborators add exact
shared Type versions to their world or create a world-exclusive Type. Both library
and local Type edits create immutable versions. Updates do not automatically
retarget world sources or form/variant assignments; adoption and reassignment are
explicit, separate actions.

Shared forms select ordered shared Type versions. World variants select ordered
**same-world** local Type versions. Follow **Edit typing**, choose the displayed
slot count *before editing* (the GET reloads saved values), then choose the typing
state and native slot dropdowns:

- **Unmodeled**: this definition does not describe its typing yet.
- **Defined** with no selections: intentionally typeless.
- **Defined** with selections: exact pins, in displayed order; blank slots are
  skipped and duplicate Type identities are rejected.

The 32-slot software work bound is not Pokémon's two-type rule. Metadata edits
preserve exact typing state and pins. Shared-form adoption shows old/new source
typing but retains local world typing; it never maps by name or picks arbitrary
matching local Types. Type descriptions are inert metadata; pairwise effectiveness
is authored independently in charts. Damage and transformation execution are not implemented.

### Effectiveness charts

Shared charts are versioned baselines. A world has at most one independently
versioned local chart, optionally pinned to an exact shared-chart revision. Open
**Charts** or follow **View or create world chart** from a world. Public pages show
an attacking-row/defending-column matrix at desktop and a vertical pair list at
narrow widths. History shows readable relationship/pin changes and inert full JSON.

1. Create a chart, enter its metadata, and select an ordered roster of exact Type
   versions. Up to 32 distinct Types is a software work bound, not a game rule;
   an empty roster is a valid draft. Use **Reload slots and source** before editing:
   it deliberately discards unsaved fields. Source selection does not infer pins or
   mappings. Preview creation, review the proposed matrix, then confirm it.
2. Follow **Edit attacking row: …**. Enter a numerator and denominator against each
   defender. Both blank means **Unknown**; one blank is an error. Exact reduced
   fractions support values such as 1/2, 1, 2 and 3/2. Numerators range from 0 to
   1,000,000; denominators from 1 to 1,000,000. A row save atomically replaces that
   row; equivalent fractions are no-ops. Reverse pairs and diagonals are independent.
3. **Unknown**, explicit **1× Neutral**, **0× No effect**, and an outside-roster
   **Not covered** result are distinct. The separate confirmed **Fill remaining
   Unknown cells with neutral** command fills missing pairs only. Adding a roster
   Type leaves its pairs Unknown; removing one drops its cells/mappings only in
   the new revision. Past snapshots remain unchanged.
4. To use a shared baseline locally, select an exact chart revision and explicitly
   map source identities to local roster identities. Each source and local Type
   maps at most once; multiple local Types sharing one source are never chosen
   arbitrarily. A mapped local Type must have that shared Type identity as its
   source. Unmapped/world-exclusive Types are allowed.
5. To adopt a newer revision, choose mappings and **keep** or **apply** source cells,
   preview old/local/new/proposed matrices, then confirm. Applying replaces every
   mapped pair, including clearing a local factor when its source pair is Unknown.
   Unmapped pairs stay local. Changing the shared chart never updates worlds silently.

Lookup uses canonical Type identity across metadata versions: a variant retaining
Fire v1 can use Fire's chart row pinned at v2 after a description change. The exact
pins and historical names remain preserved. Chart edits/adoption do not retarget
form/variant typing or Type definitions. Distinct mechanical categories need
distinct Type identities. Pairwise lookup does not yet define multi-Type stacking,
damage, rounding or a battle ruleset.

Chart edits have independent optimistic revisions, session/role/world-scope checks,
attributed audit, retained error drafts and explicit conflict resubmission. POSTs
are canonical-ID-bound and capped at 32 KiB. See decision
`docs/decisions/0010-type-effectiveness-charts.md` for storage and compatibility.

### Independent stats and world rosters

Stat names and counts are authored data, not a fixed Pokémon vocabulary. Shared
and world-local definitions have independent identities, exact revisions, inert
descriptions and effective numeric constraints. A local Stat may pin an exact
shared source or be world-exclusive. Creating a definition does not add it to a
roster, and definition edits never retarget existing roster or creature pins.

1. Open **Stats → Create shared stat**, or a world's **World stat definitions**.
   Numeric minimum/maximum are optional and inclusive. **Whole numbers only** is
   optional; signed decimals are otherwise permitted. Negative values are valid
   unless an authored minimum excludes them.
2. Open **World stat roster → Edit stat roster**. Explicitly initialize the ordered
   roster, then add/remove entries, select newer versions, or move positions.
   Preview and confirm each operation. Unconfigured differs from intentionally
   defined-empty. There is no fixed stat count or inherited 32-Type limit.
3. Shared forms expose **exact base stats**. Select shared definitions and author
   reusable values. Variants expose **View or edit local base stats** and use an
   exact world-roster revision. Unmodeled, defined-empty, Missing and explicit zero
   are distinct. Incomplete values are valid authoring drafts.
4. Values use exact signed decimals with a signed 64-bit coefficient and at most
   six fractional digits. `-1.250000` becomes `-1.25`; `-0.00` becomes `0`.
   Equivalent spellings are no-ops. Excess precision, overflow, exponent notation
   and non-finite numbers are errors, never rounded/clamped. Blank means Missing.
5. A roster change leaves saved variant context intact. **Review roster
   reconciliation** shows carried, new and removed values. Identity retains a value
   across reorder; new stats start Missing. Correct values that violate selected
   constraints or explicitly leave them Missing before confirming. History keeps
   removed values and exact historical constraints.
6. Definition source adoption changes only that definition. Variant **Review source
   base values** compares old/new source, local and proposed values. Select explicit
   injective shared-to-local mappings; names, order and duplicate provenance never
   choose a mapping. Unmapped values stay local. A Missing source clears a value
   only with its explicit clear choice. Applying updates the form source pin and
   selected values atomically. Ordinary form adoption keeps local stats.

Editors show at most 25 entries per page. **Save the current page before navigating**
after changing values, pins, mappings or reason. Clean pages navigate without
writing; dirty navigation retains raw input and offers explicit discard. Saves
merge only displayed values with verified saved state, preserving off-page edits.
Preview confirmations are bound to the exact reviewed inputs/proposal; changing
them requires a fresh preview. Both creature and current roster revisions are
checked, and conflicts require explicit review. Public history includes readable
identity-based changes and pageable historical definitions/values/constraints.

These are authored base values, not derived-stat formulas, level scaling, modifiers
or damage execution. See decision `0011-independent-stats-and-world-rosters.md`.

### Moves, learning and presets

Move attributes are independent numeric definitions, separate from creature Stats.
Designers choose their names, descriptive units, exact signed decimal constraints
and count; Power, Accuracy and a particular attribute count are not built-in rules.
Shared and world-local definitions have their own histories and optional exact
shared-source pins. Moves own values against exact same-scope attribute revisions:
**Known**, **Unknown** and **NotApplicable** are explicit states. An unmodeled
collection, a defined-empty collection, an absent attribute and known zero all
have different meanings. Numeric precision/range rules are the same exact decimal
rules used for Stats; malformed numbers and intrinsic constraint violations are errors.

1. Open **Move attributes** or a world's local catalog, create a definition, then
   open **Moves** to create a shared or local Move and author its attribute values.
   New selections require unarchived definitions/Moves. Retained exact archived
   pins remain usable for unrelated edits and historical reads; no dependency is
   silently advanced or replaced.
2. For source adoption, review **old source / local / new source / proposed** and
   choose keep-local or explicit source application. Move copying uses injective
   shared-to-local attribute mappings; learnset copying uses injective Move mappings.
   Each source and local identity maps at most once, and mapped local objects must
   explicitly name the matching shared source. Names and duplicate provenance never
   choose a mapping. Applying a source collection is a **whole-collection
   replacement**, with removed local and unmapped source entries shown in preview;
   Unknown, NotApplicable and unmodeled states are preserved rather than guessed.
3. Open a world's **Ruleset** to configure exact local attribute expectations,
   required/optional and NotApplicable policies, additional numeric restrictions,
   and optional exact Stat-roster/chart context. The optional positive moveset
   maximum and repeat policy (**unspecified / allow / forbid**) are authored data;
   there is no four-move default. Interaction/environment notes are inert text.
   Configuration does not backfill object values or execute formulas.
4. Open a shared form's or local variant's **Learnset**. Each owner has at most one
   independently versioned learnset. Ordered acquisition records have stable IDs
   scoped to that learnset, exact same-scope Move pins, optional method/context/
   condition prose and a Known/Unknown/NotApplicable numeric threshold. Multiple
   records may reference the same Move/revision. Prose is inert; a threshold is not
   implicitly a level. A local learnset's optional shared-learnset source pin is
   independent of the variant's form-source pin. Source copying retains every
   acquisition separately, including repeated Moves.
5. Open a variant's **Moveset presets** and create multiple named drafts. Each
   preset pins an exact learnset revision belonging to that same variant and an
   exact same-world ruleset revision. Ordered entries select acquisition-record
   IDs within the pinned learnset, not Move IDs. Repeated entries are representable;
   forbid-repeat diagnostics compare canonical Move identity even across different
   acquisition IDs or Move metadata revisions. There is no default preset or
   runtime-individual/learned-repertoire model.
6. Preview a preset or open **Validation**. Deterministic authoring reports identify
   the saved **validator policy**, exact input revisions, **contradictions**, **incomplete**
   inputs and **not_evaluated** checks. Required Unknown/absent values are incomplete;
   required NotApplicable and violated known restrictions contradict configuration.
   Well-formed incomplete or contradictory drafts can be saved. Broken references,
   wrong scope/owner, malformed numbers and intrinsic definition violations cannot.
   Reports are **not combat readiness**: acquisition eligibility, conditions,
   effects, formulas and battle results are not evaluated. Historical reports use
   stored exact dependencies and the identified policy, never current heads.
7. Reconcile a preset explicitly by selecting new learnset/ruleset pins and reviewing
   the selected acquisition IDs. Definition, source, creature or ruleset edits do
   not retarget existing presets. Review and confirm the intended new context.

New presets and ad-hoc Move/configuration reports use **policy 2**. Impossible
Required Known ranges are configuration contradictions even with Unknown values or
an empty selection: intrinsic and additional bounds must intersect, including
whole-number feasibility. No configured maximum means no cap; unspecified repeat
policy is not evaluated. Structurally valid contradictory drafts remain saveable.

Existing presets retain **policy 1** on ordinary edits, with their original report
semantics and historical bytes. In the preset editor, explicitly choose
**Validation policy → Revalidate using current policy 2**, preview and confirm to
create an attributed policy-2 revision. Changing the choice invalidates a previous
confirmation. Old revisions and fixture creation baselines stay on policy 1.
Unknown future policies remain explicitly unsupported. See
[ADR0013](docs/decisions/0013-authoring-validation-policy-and-archive-selection.md).

These native editors paginate collections and bound operations to at most 25
displayed entries. Save before dirty navigation or explicitly discard the draft;
off-page saved entries are retained. Long prose uses **one text field per
operation**: `text_field` identifies the loaded field, `text` is its textarea,
and `next_text_field` plus `load_text` selects and loads another field *before*
editing. Save or explicitly discard dirty text before switching. Other text values
stay saved rather than travelling in hidden fields. This supports the domain's
Unicode limits within the 32 KiB form limit without hidden-field duplication or
silent truncation.
Acquisition method/context/condition and ruleset Description/Notes use this loaded
field workflow. New rulesets save their required Description first; Notes can then
be loaded and edited separately. Notes allow 2000 UTF-8 bytes, descriptions allow
2000 characters, and descriptive move-attribute units allow 100 UTF-8 bytes.

Previews require explicit confirmation bound to a digest of the reviewed
inputs/proposal; changed inputs require a fresh preview. Expected revisions are
checked before submitted dependencies. Validation, storage, conflict and **403**
authorization failures retain raw drafts. GET/HEAD pages use readable scoped
addresses; mutation POSTs remain canonical-ID-bound across rename/reuse.

Move attributes, Moves, rulesets, learnsets and presets have separate attributed
histories. Form versions do not capture learnsets, variant versions do not capture
presets, and world revisions do not capture rulesets. There is **no creature v4
snapshot**: existing v1/v2/v3 bytes and membership semantics remain intact. See
[ADR0012](docs/decisions/0012-moves-learning-and-world-rulesets.md) and the
[pinned roadmap](docs/architecture/roadmap.md) for the delivered boundary and deferred work.

### World inspection and where used

Open a world's **Authoring inspection** link for a public, read-only GET/HEAD report
at `/worlds/{world}/inspection`. Native GET filters select finding category, object
kind and an optional current-policy preset comparison; query fields are typed and
allowlisted. Findings, inventories and
usage lists show at most **25 entries per page**. Each finding previews at most
25 evidence nodes and explicitly counts the remainder; full typed exact evidence
is still captured for analysis. Inspection has no POST, CSRF form, new authorization
policy, saved report or content/audit writes.

One ordinary SQLite read transaction resolves the world as its first read, then
captures all independent current local roots, including archived and unused
objects, explicit absences and their exact dependency closure. Shared content
enters only through explicit dependencies. A variant's source form does **not**
implicitly supply that form's independent learnset. Older pinned revisions,
multiple acquisitions of the same Move and every preset selection occurrence
remain distinct. Current catalog membership/ownership is an observation, not a
claim that the displayed world revision historically pins every independent head.
The read transaction ends before fingerprinting, analysis, usage traversal or
rendering.

Reports identify **Contradiction**, **Incomplete**, **ContextDifference** and
**notEvaluated** findings; capture gaps are separate. Current Move/ruleset
diagnostics use policy 2. Saved presets retain their exact inputs and saved policy
1 or 2; the optional policy-2 comparison is ad hoc and saves nothing. Older pins
and reached archived dependencies are informational, not automatically invalid.
Missing/zero, Unknown/NotApplicable and unmodeled/defined-empty stay distinct.
Chart lookup uses scoped canonical Type identity across metadata revisions and
preserves known neutral, zero, Unknown and outside-roster results. These reports
establish neither combat nor export readiness; effects, eligibility and formulas
are not executed.

Follow **Where used · exact revision** or the all-versions link to
`/worlds/{world}/inspection/where-used`. “All” means **all captured versions**, not
all history. Direct consumers and representative bounded shortest root paths are
scoped to this current-world capture. Membership/ownership does not count as
content usage; full learnset acquisitions differ from actual preset selections.
Path results are bounded to 256, depth to 32 and traversal work to 65,536, with
explicit truncation. “Unused in this world capture” never means globally unused or
safe to delete. Partial captures only say no uses were observed in the inspected
portion. Keys use immutable identities and exact revisions; history links and
labels derive from captured public data, without post-capture database reads.

**Freshness:** every request captures again. Only a complete capture has a
deterministic SHA-256 inspection fingerprint over public semantic roots, absences,
exact nodes/edge occurrences and observed heads. Time, private/access data,
filters and pagination are excluded. Navigation carries the previous fingerprint;
changed inputs produce a refreshed-result banner and reset pagination to page 1.
**Refresh inspection** starts a fresh capture without the previous fingerprint.
This is not frozen historical navigation, a database checksum or an engine package.
Private world ownership/access records, actors, grants, sessions, raw audits and
raw snapshot JSON are never reported or fingerprinted; only explicit public DTOs
cross the capture boundary.

**Operational bounds:** defaults are 2,048 roots, 8,192 exact nodes, 65,536 edge
occurrences, 1 MiB per preflighted snapshot, 16 MiB total accounted snapshot bytes
and a 10-second deadline across connection acquisition, anchor and capture. An
unusable world anchor is unavailable; child read failures, unsupported data and
budget exhaustion produce explicit partial coverage, not Unknown game data.
Partial captures have **no fingerprint** and make no “Unused” finding. Preflight
and per-request caching bound outer work; existing readers' nested verification
queries are not a perfect cost meter. SQLite's safe progress callback checks about
every 1,000 VM operations for cooperative interruption; handler removal, rollback
and close-on-drop service leases protect connection reuse/cancellation. The existing
five-second busy wait, filesystem work and non-VM work can outlast the deadline:
this is not a hard real-time bound. At most **two inspection jobs** run per service,
with admission held through rendering. Fingerprinting, analysis and path work use
`spawn_blocking`; shared immutable exact evidence avoids quadratic deep clones.
These limits are operational, not new game cardinality caps or query-adjustable
allowances. See [ADR0014](docs/decisions/0014-world-inspection-and-dependency-visibility.md).

The original ADR0014 inspection reuses ClassicLab/VariantLab and StatsSample and
added no schema, dependencies or fixture objects. Setup still has **65 objects and 69 initial audit
events**. Existing history bytes, heads and exact pins retain their meaning.

Card-aware public reports extend this capture at
`/worlds/{world}/inspection/cards` and `/worlds/{world}/inspection/cards/where-used`.
They include native components, standalone cards, reusable field definitions and
mixed native/custom direct usage and bounded root paths. Both typed and card
pages share the complete-only **fingerprint v2**, including card/field roots,
content and observed heads. Partial capture/truncated usage stays explicit; it
does not imply authored Unknown, globally unused or safe to delete. Card inspection
adds no further fixture objects; migration 0009 supplies its card storage.

### Frozen design packages

**First Frozen Design Package** delivered schema v1; new browser downloads now use
`karta-design-package` **schema 2**, `game_data::card_package::CardPackage`.
It freezes authored design content, including well-formed drafts, for offline
reading. The unchanged schema-1 `game_data::package::Package` reader and the new
schema-2 reader are intended for eventual engine
integration; no engine currently consumes it, and structural acceptance does not
establish gameplay acceptance or combat readiness. Effects, formulas, acquisition
eligibility, scripts and battle execution remain unevaluated.

1. Sign in as an enabled editor/administrator who owns the active world or is its
   collaborator. Follow **Export package** or open `/worlds/{world-slug}/export`.
   This authorized GET/HEAD only opens the entry page. Administrator status has
   **no global ownership bypass**.
2. Choose **Preview package**. The native CSRF-protected POST captures fresh content
   and shows the content SHA-256, root/record/binding counts, byte size and separate
   authoring diagnostics. Root and finding previews show at most 25 each, with
   remaining counts; the package is not truncated to that display limit.
3. Review, check **I reviewed this package and its draft diagnostics**, then choose
   **Confirm and download**. POST forms target the immutable world ID and are
   limited to **4 KiB**. Confirmation recaptures and binds to the reviewed package
   digest; changed inputs or missing confirmation show a fresh preview that must
   be confirmed again. Session/world permission is checked during capture and
   rechecked after processing before release, including on confirmed downloads.
4. Save the JSON attachment (`karta-{world-id}-{first-12-digest-characters}.json`).
   It is completely built in memory and its final bytes pass the actual shared
   reader before release. Responses are private/no-store; the server retains no
   package artifact or public download URL.

The `active-world-catalog-v1` policy selects the current active world, all current
unarchived local Types, Stats, move attributes, Moves and variants, its configured
roster/ruleset and unarchived chart, and current learnsets/unarchived presets of
active variants. Unused active definitions are included. Archived variants' children
are not independent roots. Exact transitive dependencies include required older or
archived revisions. Identity-only learnset/preset owners and species default forms
have explicit capture-time identity-to-exact-key bindings; these are not newly
authored historical pins. Shared content enters through explicit references, never
catalog membership, names or an implicit source-form learnset. Ordered entries and
repeated acquisitions/selections are preserved.

Schema 2 adds active local standalone-card and reusable-field roots, native card
roots mirroring typed Variant/Type/Move roots, complete same-revision components
and exact card/field reference closure. Its `typed` object retains schema 1 but is
a projection, **not necessarily a standalone v1 package**: custom references can
reach additional older/archived native cards. All typed constraints still apply;
the combined graph rejects missing dependencies and unrelated extras.

Capture resolves the world first in one ordinary SQLite read transaction, checks
session/permission on that connection and materializes content before ending the
transaction. Package-wide validation, diagnostics, hashing, serialization and
rendering follow outside it. Current Move/ruleset diagnostics use **policy 2**; saved presets keep
**policy 1 or 2**. Contradiction, Incomplete, ContextDifference and notEvaluated are
authoring findings, not structural failures: Missing, Unknown, NotApplicable, zero
and unmodeled/defined-empty drafts remain representable. Unsupported schema/policy,
invalid references/numbers, inconsistent embedded snapshots, required read failures
or budget gaps prevent export; there is no partial package.

The deterministic envelope has a manifest and canonical content, with a dedicated
SHA-256 domain marker over compact canonical content JSON. Keyed outer lists are
sorted; authored nested order remains meaningful. The reader allows JSON whitespace
but requires exact field order and value encodings. It neither serializes inspection
reports nor reuses their fingerprints. Checksums identify content, not authenticity.
Private accounts, sessions, world ownership/access data, actors and audits are
excluded. There is no generation timestamp; public archive-state fields remain.
Export changes no content/history or fixture objects, and keeps
complete setup at **65 objects/69 initial audit events**.

Validate a download offline, inside `nix develop`:

```bash
cargo run -p tools -- validate-package path/to/package.json
cargo run -p tools -- validate-package - < path/to/package.json
cargo run -p tools -- validate-package < path/to/package.json
```

`validate-package [path|-]` defaults to stdin, dispatches schema **1 or 2** to the
corresponding strict reader, needs no database/network and exits
nonzero on structural rejection. Success prints the digest, counts and separate
diagnostic counts. This CLI **validates only**; package creation is the browser
workflow above.

**Operational bounds:** capture shares inspection's defaults: 2,048 roots, 8,192
exact nodes, 65,536 reference occurrences, 1 MiB per preflighted snapshot, 16 MiB
total accounted snapshot bytes and a cooperative 10-second acquisition/anchor/
capture deadline. Package input and complete output are additionally capped at
**32 MiB**; schema 2 combines typed/card/field root (2,048), record (8,192) and
authored occurrence (65,536) budgets; native components count separately from typed
records. The reader bounds bindings too. At most
**two export jobs per service** hold admission through validation, serialization
and preview rendering. CPU work uses blocking workers. The SQLite progress callback,
caches and cancellation cleanup are shared with inspection; nested verification
is not a perfect cost meter, and five-second busy waits, filesystem and non-VM work
can outlast the deadline. These are operational bounds, not hard real-time or new
game cardinality rules. See the [v2 protocol and reader contract](docs/architecture/design-package-v2.md)
for exact ordered JSON, combined closure and dedicated v2 SHA-256 domain. The
[frozen historical v1 protocol](docs/architecture/design-package-v1.md) retains its
unchanged wire/token rules and standalone reader. See also
[ADR0015](docs/decisions/0015-first-frozen-design-package.md) for the decision.

### World fixtures

The cached reference subset contains **Fire, Flying, Dragon**, one shared Charizard
species and its base/Mega X forms with ordered typing. These are explicitly marked
temporary Pokémon placeholders, not a complete mechanics dataset. Source caches
under `fixtures/pokeapi/` retain endpoint kind, external id/URL, UTC retrieval time
and transformation version. Internal ids are independent of external ids.

The normal `seed-fixtures` command loads **all** bundled content atomically:
4 species, 6 forms, 3 shared Types, 2 worlds, 7 local Types, 7 variants,
1 shared chart, 2 world charts, 3 shared Stats, 7 world Stats, 2 stat rosters,
6 move attributes, 2 shared Moves, 4 local Moves, 2 rulesets, 3 learnsets and
4 named presets: **65 content objects and 69 initial audit events**. Any
bootstrap account, invitation changes, ownership, versions and audit entries
commit together; an error leaves none of that invocation's writes. No external
data is fetched.

Ownership is resolved inside the write transaction:

1. `--owner-user-id <id>` selects an existing enabled editor/administrator and
   must match any existing fixture-world ownership.
2. Otherwise, complete existing fixture worlds keep their common eligible owner.
3. With no users at all, `KARTA_DEV_AUTH=1` enables bootstrap of `admin` (or
   `--admin-subject <name>`). Disabled accounts also count as existing users.
4. Otherwise, the sole enabled administrator owns the new labs. With zero or
   multiple candidates, pass an explicit owner id from `admin list-users`.

`--owner-user-id` and `--admin-subject` are mutually exclusive. The latter only
names a potential bootstrap account; once users exist, normal ownership resolution
applies. Setup never promotes/enables accounts or transfers worlds.

For smaller, targeted setup batches, use:

```bash
cargo run -p web-app --bin admin -- seed-creature-fixtures
cargo run -p web-app --bin admin -- seed-reference-fixtures
```

The targeted `seed-world-fixtures` command still requires an existing eligible
owner and initializes the reference prerequisites if missing. It creates no
accounts. Copy the owner's immutable id from `/account` or `admin list-users`.
Run inside `nix develop`, using the same
`KARTA_DATABASE_URL` as the server:

```bash
cargo run -p web-app --bin admin -- list-users
cargo run -p web-app --bin admin -- seed-world-fixtures --owner-user-id <immutable-user-id>
```

Open **Worlds** to explore:

| World | Examples |
| --- | --- |
| **ClassicLab** (`/worlds/classiclab`) | Charizard Fire/Flying and Mega X Fire/Dragon |
| **VariantLab** (`/worlds/variantlab`) | Independent same-source Charizard variants, a three-Type variant, and world-exclusive Mist/Mistling |

Creation history is attributed to `bundled-fixture-loader`, including owner-only
initial access history. Complete setup reruns skip **65** matching content objects;
targeted world reruns skip 20 and create no charts, Stats, move/learning objects or
rulesets. Reruns preserve later edits, exact pins, typing, chart factors, sessions and
collaborator changes. A different owner, conflicting custom singleton, collisions,
drift or partial baselines reject the whole batch; existing worlds are never
repaired or overwritten. These synthetic labs are not official Pokémon editions,
player saves, encounter legality, battle transformations or engine packages.

Forms preserve submitted values after validation/authorization/storage failures.
Stale edits return **409**, display the newer record, and require **Save reviewed
changes**; another competing save produces another conflict. No automatic merging
or silent overwrite occurs. Authoring bodies are capped at **32 KiB**; slug/name/
description limits are 80/100/2,000 characters, with optional 500-character reasons.
Descriptions support plain-text paragraphs with LF line endings. The server is
authoritative for Unicode character limits; HTML's code-unit limits allow valid
non-BMP characters rather than prematurely truncating them.

### Object names, slugs and addresses

Slugs are generated by the server from display names; there is no manual slug
field. Accented and non-Latin names are transliterated with pinned `deunicode`
1.6.2: for example, **Flabébé Forest → `flabebe-forest`**. Separators collapse to
hyphens, addresses are capped at 80 ASCII characters, and an untransliterable name
uses `object`. Collisions get `-2`, `-3`, etc. within the existing uniqueness scope.

Object pages use current readable addresses:

```text
/creatures/emberling
/creatures/emberling/forms/emberling
/worlds/forest-world
/worlds/forest-world/variants/emberling-2
```

Edit, history, adoption and access GET links follow the same coordinates. Forms
are scoped to species; variants are scoped to worlds, so names can be reused in
different scopes without ambiguity. If a generated/stored slug is `new` or looks
exactly like a 32-character lowercase hexadecimal ID, its URL segment is prefixed
with `~` (such as `/worlds/~new`) to distinguish it from creation pages/ID links.
The marker is URL syntax, not stored data.

Renaming an object regenerates its slug atomically with the normal attributed
revision. Description-only edits and pin-only adoption keep the existing slug.
Existing manually assigned slugs also remain until the display name changes.
Parent renames change nested child addresses without rewriting their versions.

Canonical IDs remain immutable and identify all relationships and history. Old
ID GET/HEAD links, including `/forms/{id}`, temporarily redirect **307** to the
current readable address with suffix/query preserved and no-store caching. These
are durable permalinks; old slug paths have no historical alias and can become
404 or identify a new object if that slug is reused.

Save actions intentionally use canonical-ID targets so an open draft cannot edit
a replacement object after a rename/reuse. Successful saves use **303**, then
land on the readable address. Validation/conflict drafts retain their ID POST
address and entered values. Legacy manually submitted slug fields are ignored.
No migration or existing-data backfill is needed. See
`docs/decisions/0007-readable-object-addresses.md`.

### Existing database upgrade and initialization

For an existing development database, stop the old server and back up with
SQLite's backup facility (the example uses the default path; choose an unused
backup filename). Migrate/initialize deliberately, then restart the new binary:

```bash
sqlite3 -cmd ".timeout 5000" data/karta.db ".backup 'data/karta-before-authoring.db'"
cargo run -p web-app --bin admin -- migrate
KARTA_DEV_AUTH=1 cargo run -p web-app --bin admin -- seed-fixtures
```

Migration `0002_world_authoring.sql` adds typed content/version/access tables;
`0003_types.sql` adds Type versions, exact ordered bindings, provenance and full
snapshot sidecars. `0004_charts.sql` adds independent chart versions and typed
roster/cell/mapping seals. `0005_stats.sql` adds independent Stat/roster revisions;
`0006_stat_content.sql` adds exact base-value bindings and authoritative v3 creature
snapshots. `0007_moves_rulesets.sql` adds typed move-attribute, Move and world-ruleset
versions; `0008_learning.sql` adds independent learnset/preset versions and exact
acquisition bindings. `0009_cards.sql` adds same-revision card components,
standalone cards, reusable fields and immutable legacy-boundary bookkeeping.
New binaries require all **nine migrations**; older binaries reject
the newer schema. Migrations do not initialize content or alter existing auth
records. The
explicit fixture command preserves existing canonical creature ids as species
ids, creates default-form ids once, and attributes initial history to the named
`bundled-fixture-loader` system actor. Complete setup may bootstrap the first
development administrator; targeted seeds create no accounts. Matching seed
baselines are skipped after edits; drift/collisions roll back the entire seed batch.
An empty uninitialized library is supported.

The Types upgrade preserves old JSON bytes and immutable versions. Old form,
variant and world revisions retain their v1 meaning, including unmodeled typing.
The Types milestone introduced v2 full snapshots, linked to existing v1 metadata
projections and the same audit event. Missing/inconsistent full snapshots fail
closed rather than silently falling back. History reads both versions, including
v1-before/v2-after transitions. Species, Type and private access snapshots use their
own v1 schemas. Do not edit old migrations or add retrospective typing to history.

The chart upgrade preserves all existing v1/v2 bytes, users, sessions and pins.
World snapshots keep their existing Type/variant membership meaning; chart history
uses its own strict v1 schema and does not advance world revisions. The migration
creates no charts. Explicit complete setup adds the synthetic chart baselines to
existing labs without rewriting old history. Targeted creature/reference/world
seed commands retain their earlier scopes and do not initialize charts, Stats,
move attributes, Moves, rulesets, learnsets or presets. Back up,
stop the old binary, migrate, run complete setup and start the rebuilt server.

The Stats upgrade preserves existing v1/v2 bytes and treats their stats as
Unmodeled. New form/variant revisions require v3 full snapshots plus retained v1
metadata/v2 typing projections. Each layer preserves its own predecessor chain;
missing or inconsistent required seals fail closed. World history keeps its v1/v2
Type/variant membership meaning; definitions and stat rosters have independent v1
histories. Migration does not seed values. Explicit complete setup adds new sample
forms/variants, advancing lab membership without modifying original creature
baselines. That retained Stats-era baseline contains **44 objects**.

The Moves/learning upgrade preserves those 44 creation baselines, all existing
creature v1/v2/v3 history, accounts, sessions and world membership. Migrations 0007/
0008 create schema only; neither migration nor server startup seeds content.
Explicit complete setup adds **21 independent objects**, using the existing
StatsSample form/variant owners without advancing their or their worlds' revisions.
It adds 21 audit events to the previous 48. Complete reruns skip **65 objects** and
preserve later authored data. Partial new batches, wrong creation actors/baselines
and conflicting custom rulesets or owner learnsets reject the whole invocation;
even a last-preset entry/seal failure rolls back all groups and bootstrap changes.

The card upgrade preserves existing snapshot bytes, heads and audits. Pre-boundary
native revisions remain explicitly custom-data-unmodeled; migration does not invent
empty authored data, actors or events. Later revisions and new native creations
require same-event sealed components; missing post-boundary components fail closed.
No field definitions or standalone cards are implicitly seeded. Fresh fixture
creation adds technical component/child rows, while authored totals stay **65/69**.
No production database was migrated during this documentation task. See
[migration and writer details](docs/architecture/card-foundation.md#migration-0009-and-fixture-compatibility).

## Available workflows

- `/`: project overview.
- `/cards`, `/card-fields`, `/worlds/{world}/cards`, `/worlds/{world}/card-fields`:
  shared/local catalogs, with `/new`; card detail is `/{namespace}/{id}`, field
  detail `/{id}`, supporting `/edit`, `/history`, `/history/{revision}`.
  Standalone cards/fields also accept generated scoped slug aliases on GET/HEAD.
- `/worlds/{world}/inspection/cards` and `/inspection/cards/where-used` under the
  same world prefix: public captured card/field inventory and mixed bounded usage.
- `/creatures`: catalog and case-insensitive name search.
- `/creatures/{species-slug}`: shared species details.
- `/creatures/new`, `/creatures/{species-slug}/edit`: administrator authoring.
- `/creatures/{species-slug}/forms/new`, `/creatures/{species-slug}/forms/{form-slug}`:
  shared form creation/detail; form detail paths support `/edit`.
- `/worlds`, `/worlds/new`, `/worlds/{world-slug}`: world catalog and creation/detail.
- `/worlds/{world-slug}/inspection`: public read-only authoring inspection, native
  filters, coverage/fingerprint and paginated findings/inventory.
- `/worlds/{world-slug}/inspection/where-used`: world-scoped direct consumers and
  representative root paths for an exact revision or all captured versions; follow
  inspection links to supply typed target parameters.
- `/worlds/{world-slug}/export`: authorized GET/HEAD entry, native ID-bound POST
  preview and CSRF-protected confirmed JSON download for eligible owners/collaborators.
- `/worlds/{world-slug}/variants/{variant-slug}`: local detail, `/edit` and `/adopt`.
- `/types`, `/types/new`, `/types/{type-slug}`: shared Type catalog, creation and detail.
- `/worlds/{world-slug}/types/new`, `/worlds/{world-slug}/types/{type-slug}`:
  world-local Types; detail supports `/edit`, `/adopt` and history.
- Shared form and world variant detail paths support `/typing` for ordered assignments.
- `/charts`, `/charts/new`, `/charts/{chart-slug}`: shared chart catalog/create/detail.
- `/worlds/{world-slug}/charts`: world chart landing; the same path with `/new`
  creates its chart.
- `/worlds/{world-slug}/charts/{chart-slug}`: independent local chart detail.
- Shared/local chart details support `/edit`, `/rows/{attacking-Type-id}`, `/neutral`,
  `/history` and `/history/{revision}`; local details also support `/adopt`.
- `/stats`, `/stats/new`, `/stats/{stat-slug}`: shared Stat catalog/create/detail.
- `/worlds/{world-slug}/stats`: local Stat catalog; local definition details support
  `/edit`, `/adopt` and history. Shared definitions support `/edit` and history.
- `/worlds/{world-slug}/stat-roster`: singleton roster with `/edit` and history.
- Shared form and variant detail paths support `/stats`; variants also support
  `/stats/reconcile` and `/stats/source`. Form/variant history revisions expose
  `/stats` for pageable before/after exact historical values.
- `/move-attributes`, `/moves`: shared catalogs, with `/new` and `/{slug}` details.
  Details support `/edit`, `/history` and `/history/{revision}`.
- `/worlds/{world-slug}/move-attributes` and `/worlds/{world-slug}/moves`: local
  catalogs; `/new` and `/{slug}` details support editing, history and `/adopt`.
- `/worlds/{world-slug}/ruleset`: independent singleton configuration, with `/edit`,
  `/history` and `/history/{revision}`.
- `/forms/{id}/learnset`: durable ID entry point redirecting to the readable shared
  form address, e.g. `/creatures/emberling/forms/statssample/learnset`; supports
  `/edit`, `/history` and `/history/{revision}`.
- `/worlds/{world-slug}/variants/{variant-slug}/learnset`: local learnset, with
  `/edit`, `/adopt`, `/history` and `/history/{revision}`.
- `/worlds/{world-slug}/variants/{variant-slug}/movesets`: named preset list, with
  `/new`, `/{preset-slug}`, `/{preset-slug}/edit`, `/{preset-slug}/history`,
  `/{preset-slug}/history/{revision}` and `/{preset-slug}/validation`.
- Content detail URLs have `/history` and `/history/{revision}` views; lists are
  bounded to 50 changes per page.
- `/worlds/{world-slug}/access`: owner-only collaboration and private access history.
- `/login`, `/logout`, `/account`: placeholder sign-in and the signed-in account.
- `/healthz`: process health.
- `cargo run -p tools -- validate-fixtures [path.json]`: validate bundled or
  custom fixtures; errors exit nonzero. Custom files are checked, not imported.
- `cargo run -p tools -- validate-package [path|-]`: offline package validation
  (stdin by default), with structural acceptance and separate authoring diagnostics;
  no CLI package creation or engine gameplay acceptance.
- `cargo run -p web-app --bin admin -- <command>`: operator CLI:
  - `migrate` — apply pending migrations (deliberate; never automatic).
  - `seed-fixtures [--owner-user-id <id> | --admin-subject <name>]` — initialize
    all bundled content, including charts, Stats and the Moves/learning examples,
    atomically, with development admin bootstrap if no users exist.
  - `seed-creature-fixtures` — initialize only the original species/default forms.
  - `seed-reference-fixtures` — load the cached Type/Charizard subset.
  - `seed-world-fixtures --owner-user-id <immutable-user-id>` — load the complete
     synthetic lab worlds and reference prerequisites with explicit ownership.
  - `seed-card-workshop --owner-user-id <immutable-user-id>` — separately initialize
    the original illustrative Card Workshop (one world, seven cards, seven fields),
    atomically, under the system actor. Requires an existing enabled editor/admin;
    creates no accounts and preserves later edits on reruns.
  - `invite-user --subject <name> [--role <viewer|editor|administrator>] [--provider <id>] [--note <text>]` (default role: viewer).
  - `list-users`, `list-invites`.
  - `disable-user --id <immutable-user-id>`, `enable-user --id <immutable-user-id>`.
  - `--help` or `<command> --help`: usage without configuration/database access.

For example, copy the account id printed by `list-users`:

```bash
cargo run -p web-app --bin admin -- list-users
cargo run -p web-app --bin admin -- disable-user --id <immutable-user-id>
cargo run -p web-app --bin admin -- enable-user --id <immutable-user-id>
```

The CLI is trusted local operator tooling: access to the executable and database
authorizes these operations, independently of browser roles. No admin HTTP routes
are exposed. Invalid commands are rejected before opening the database.

The server and non-migration commands require an existing, compatible database.
Startup checks migration versions, success/checksums, and required schema columns
without applying migrations. Missing migrations give the explicit migration
command; incompatible history requires investigation instead of automatic repair.
Auth accounts, invites and sessions remain compatible through the additive
authoring migration. Back up an existing database using SQLite's backup
facilities before deliberately applying future migrations.

Initialization fixtures, templates and CSS are compiled into the executable;
rebuild after editing them. Live content is read from SQLite. The default database
path is relative to the working directory; set an absolute `KARTA_DATABASE_URL`
when running the executable elsewhere.

## Checks

Card Workshop verification passed: formatting, Clippy with warnings denied,
**462 Rust tests** (two browser-provision helpers intentionally ignored in normal
runs), and **104 keyboard browser tests in one complete run**. Verification used
single-job builds with `CARGO_PROFILE_DEV_DEBUG=0` and `CARGO_PROFILE_TEST_DEBUG=0`
to reduce memory pressure after an initial debug-symbol build timeout. Human visual,
screen-reader and actual browser-zoom assessment remain separate.

Inside `nix develop`:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Tests use temporary databases and cover fixture/browser behavior, malformed CLI
commands without side effects, schema compatibility, concurrent invite claims,
transaction rollback, disable/re-enable and session-creation races, persistence
across database reopening, expiry, cookie attributes, Origin/CSRF validation,
form limits, login throttling, and retryable storage/logout failures. A second
provider exercises the port using the existing development credential flow.
Authoring tests additionally cover direct-use-case scope/role checks, exact version
references, immutable history, typed snapshot validation, rollback after late
revision/audit faults, initialization baselines, explicit adoption, no-ops and
concurrent permission revocation. Migration 1→2 preserves existing sessions.
Types tests cover populated 2→3 upgrades with byte-preserved history, exact pins,
same-world scope, ordered/empty/unmodeled typing, sealed snapshots/bindings, late
fault rollback, fixture provenance/ownership/idempotence, and SQLite integrity.
Complete setup tests cover gated bootstrap, invitation handling, owner selection,
concurrent initialization, partial baselines, and late rollback of accounts plus
all content, including a failure sealing the last chart. Rerun checks preserve
edits, factors, grants, pins, archives and sessions. Chart tests cover exact rational
normalization, directed/Unknown/neutral semantics, scope/mapping/source adoption,
metadata-version compatibility, revision seals, fail-closed reads and populated
3→4 upgrades preserving every preexisting data table.
URL tests additionally cover scoped addresses, rename/reuse, reserved/ID-shaped
slugs, query-preserving redirects, HEAD behavior, unchanged mutation targets and
source-version display independent of current address coordinates. Automatic slug
allocation is checked under concurrent writes and preserves immutable IDs.
Moves/learning tests cover numeric states and intrinsic constraints, exact scope/
owner/dependency pins, repeated acquisitions and preset selections, policy-1
compatibility and policy-2 diagnostics/revalidation, impossible range intersections,
explicit whole-collection source replacement, counted archived-selection retention,
separate histories, immutable seals and populated 0006 upgrades. Complete-fixture
checks preserve all old table values and auth data, reject partial/custom/wrong
baselines, preserve later edits and roll back even at the final preset entry/seal.
Inspection tests cover independently expected roots/exact closure, archived/unused
inventory, explicit absences, old pins and repeated acquisition/selection sites,
policy preservation, scoped Type/chart context, private-free canonical fingerprints,
budget/unsupported/integrity gaps and no writes. A synchronized WAL race checks one
consistent capture before a competing edit and a fresh capture after independent
head changes. Regression coverage includes legacy companion/predecessor typed
metadata and headers, cooperative SQLite interruption, pool acquisition deadlines,
handler cleanup, rollback and cancellation-safe connection disposal.

### Headless keyboard browser tests (WSL/Linux)

From the project root:

```bash
nix develop .#browser-tests --command python tests/browser/run.py
```

This separate shell supplies Python Playwright, its matching driver, Chromium,
and fonts from the existing `flake.lock`. It runs headlessly: WSLg, a GUI, Xvfb,
and a display server are not required. No pip/npm or `playwright install` step
is needed. First use may download Nix packages and Rust build dependencies;
browser requests during the tests are restricted to the local test server.

The runner builds current native web/admin binaries with `--locked`. The source
inventory contains **104 keyboard-driven tests** (94 prior cases plus 10 Card
Workshop cases). `test_cards.py` and `test_card_workshop.py` each define five
workflows inherited at desktop (1280px) and narrow (320px) widths. It
uses fresh test-owned databases, servers on available loopback ports, and browser
contexts, independent of your running app or `KARTA_*` configuration. Those
processes and databases are cleaned up after each test. Custom `CARGO_TARGET_DIR`
locations are supported through Cargo metadata.

Coverage includes skip-link focus, forward/reverse Tab navigation, sign-in and
sign-out using Enter, retained username and correction after rejection, and a
keyboard-reachable retry after a real logout failure. Authoring coverage includes
two-world scoping, multiple/exclusive variants, library versions and adoption,
collaboration grant/revoke, repeated edit/access conflicts, readable history and
snapshots, Unicode field limits, automatic names/addresses, rename/collision and
ID-bound drafts after slug reuse. Assertions check focused controls are in view
(textareas may scroll the caret rather than the entire box), have a non-transparent
outline, and that pages have no
horizontal overflow. Navigation and typing use only keyboard events; locators
inspect state rather than clicking/focusing/filling controls.
The Types workflows also exercise native ordered dropdowns, 32 displayed slots,
Type version/adoption independence, typing conflicts and revoked grants, and the
two owned fixture labs without external fetching, plus the complete seed command,
the bootstrap administrator's first sign-in and preservation of its later edits.
Chart workflows cover responsive matrix/pair browsing, explicit mapping/creation
and adoption previews, row errors/conflicts, neutral fill, history, canonical
metadata identity and revoked collaboration without retargeting creature pins.
Moves/learning workflows cover native paginated editors, bounded one-field prose
loading, Unknown/NotApplicable/zero, source replacement previews, ruleset/preset
diagnostics, exact-context reconciliation, retained authorization/conflict drafts
and independent history.
Inspection workflows cover public keyboard entry, GET filters/pagination, retained
saved policy and ad-hoc comparison, exact/all-captured usage, repeated slots and
representative paths, freshness after a concurrent native edit/rename, escaping,
partial coverage/privacy and rejected mutation routes, with no inspection writes.
Export workflows cover authorized keyboard entry, preview/review/download,
attachment envelope/checksum checks, changed-input reconfirmation and revoked
collaboration at both widths, using temporary test-owned downloads.

Card workflows cover Unicode loaded prose/history, all five field kinds and raw
errors, explicit multi-row copy/adoption, native metadata carry-forward and archive/
revoked drafts. The world source-copy workflow also follows public card inspection,
exact field usage and representative root paths using the keyboard. Five web tests
and six pure mixed-usage tests cover further card-inspection contracts.
Workshop workflows cover named selections, typed values, invalid/dirty draft
retention, exact historical names, named source mappings/adoption, explicit setup,
edited reruns, inspection and offline-checksummed downloads at both widths.

Failures save screenshots in a unique ignored `target/browser-tests/run-*`
directory. Screenshots can be opened from Windows for diagnosis. No traces,
session tokens, cookie dumps or browser storage state are recorded. The suite
uses only synthetic accounts and does not replace `cargo test --workspace`.

This is document-level headless Chromium coverage. The 320px viewport checks
reflow, not actual 200% browser zoom; human readability/contrast assessment,
screen-reader behavior, address-bar navigation and other browsers still need
separate verification. See `docs/decisions/0005-headless-browser-tests.md`.

Manual keyboard check: Tab from the address bar to the skip link, then to Account;
sign in using the labelled username field and Enter, reach Sign out using Tab,
and activate it with Enter. Verify focus/layout at a narrow viewport and 200%
zoom. Also check that a sign-in error retains the username and that the logout
failure page's retry button is keyboard-accessible.

## Structure and dependency purposes

- `crates/game-data`: typed fixture records, domain invariants and pure authoring
  validation; `moves.rs`, `rulesets.rs` and `learning.rs` model the ADR0012 aggregates.
  `inspection.rs` defines public typed captures/edges/limits;
  `inspection_analysis.rs` supplies pure findings and bounded where-used traversal.
  `package.rs` and `package/` define the versioned offline design-package reader,
  exact closure validation, canonical serialization and content checksum.
  `cards.rs`, `card_fields.rs`, `card_inspection.rs` and `card_package.rs` with
  `card_package/` provide same-identity components, fields, mixed usage and v2 reader.
  `deunicode` supplies deterministic name-to-ASCII transliteration for automatic
  slugs; its version is pinned and existing addresses are not recomputed on reads.
- `crates/web-app`: catalog use cases, the identity-provider port, accounts and
  sessions, SQLx persistence, Axum routing, Askama HTML, custom CSS.
  - `src/auth/`: the provider port and the placeholder adapter.
  - `src/accounts.rs`: invite-only membership and opaque sessions.
  - `src/persistence.rs` + `migrations/`: SQLite queries and versioned migrations.
  - `src/application/authoring.rs`: scoped policy, transactions, versions and adoption.
  - `src/persistence/authoring.rs`: typed rows/queries and strict snapshot readers.
  - `src/web/authoring.rs`: native authoring/history forms and error rendering.
  - `src/application/cards.rs`, `src/persistence/cards.rs`, `src/web/cards.rs`:
    card/field policy, same-event `stage`/`seal_native` bridge and operation forms,
    with exact attributed history. `src/web/card_inspection.rs` renders the captured
    public card/field inventory and mixed where-used.
  - `src/web/addresses.rs`: pre-routing readable GET/HEAD addresses and ID compatibility.
  - `src/application/types.rs` + `src/persistence/types.rs`: versioned Type policy,
    exact typing, compatible full snapshots, cached reference/world initialization.
  - `src/web/types.rs`: native Type, adoption and ordered-typing forms.
  - `src/application/charts.rs` + `src/persistence/charts.rs`: exact pairwise charts,
    independent revisions, source mappings, scoped edits and sealed snapshots.
  - `src/web/charts.rs`: responsive matrix browsing, row editing and explicit previews.
  - `src/application/moves.rs` + `src/persistence/moves.rs`: independent numeric
    move attributes, Moves, world rulesets and their exact typed revision seals.
  - `src/application/learning.rs` + `src/persistence/learning.rs`: independent
    learnsets, named presets, source adoption and exact historical validation.
  - `src/web/moves.rs` + `src/web/learning.rs`: native bounded editors, explicit
    previews/confirmations, configuration/learning histories and report rendering.
  - `src/application/inspection.rs`, `src/persistence/inspection.rs` and
    `src/web/inspection.rs`: bounded consistent read capture, public-only fingerprint,
    off-handler analysis/path work and native read-only reports.
  - `src/application/export.rs` and `src/web/export.rs`: explicit export policy,
    bounded preview/confirmation and private attachment release; checked package
    capture shares `src/persistence/inspection.rs` infrastructure.
  - `src/application/fixtures.rs` + `chart_fixtures.rs`, `stat_fixtures.rs` and
    `move_fixtures.rs`: complete atomic development setup.
  - `src/bin/admin.rs`: migration and invite operator CLI.
- `crates/tools`: fixture and offline design-package validators.
- `tests/browser`: Python `unittest` + Playwright for isolated real-browser
  keyboard workflows; test-only dependencies live in the optional Nix shell.
- `src/web/login_limit.rs`: bounded process-local login request budget.
- SQLx provides SQLite access and migrations; SHA-256 hashes session tokens,
  complete public inspection captures and frozen package content (separate contracts);
  `getrandom` supplies tokens; Serde decodes fixtures. The POC that used a
  self-hosted Polycentric server lives outside this repository (see below).

## Next milestones and boundaries

The delivered foundation includes automatic names/addresses, Type definitions and
exact typing, independent pairwise charts, Stats/rosters/base values, move attributes
and Moves, ruleset configuration, independent learnsets and named presets with
exact-context authoring reports, world inspection and scoped dependency visibility,
and frozen design-package export with offline shared readers, active catalog root
policy and explicit owner/collaborator permission. The **Card Definition Foundation**
is delivered: native same-identity cards,
standalone definitions, reusable fields, card inspection and schema-2 packages.
Further card-content/world authoring and fine-tuning come next. **Engine integration
and the battle simulator are the absolute final steps**, superseding earlier
engine-next recommendations. Player instances, decks/hands/discards, attachment/
resource execution and deck-building execution are not implemented.
The [pinned product roadmap](docs/architecture/roadmap.md) retains the follow-up
directions: derived-stat formulas/modifiers, authoring recovery, engine integration
and package publication/migration, items/abilities/natures, evolution/transformation
rules, maps/encounters, source contexts/import adapters and deterministic Battle Lab slices.
The reference benchmark is documented in
`docs/architecture/reference-coverage.md`; Pokémon-specific limits belong to
versioned rulesets, not universal database constraints. Only a small cached Pokémon
metadata/typing subset is supplied; a general import/update-preview adapter and
complete mechanics model remain future work. World edit revisions still do not pin
all independent content; frozen exports capture those roots separately.
Archive/revert and ownership transfer/recovery also require explicit future
workflows. See decision `0006-world-aware-authoring`.

The independent Polycentric proof of concept was extracted from this repository to
`/home/eter/experiments/polycentric` and its local server stack was removed. It is
deferred, not adopted; decisions 0002 and 0003 record the status and the seam.

See `PROJECT_GUIDELINES.md` for the full project reference and `AGENTS.md` for
contributor instructions.
