# 0010 — Independently versioned effectiveness charts

## Context and alternatives

The maintainer approved pairwise effectiveness authoring after Types, including
canonical-identity compatibility across metadata revisions. Storing relationships
on Types would scatter a chart across revisions; embedding them in world content
would couple unrelated edits. Live shared-data overlays would silently alter world
rules. Use an independent aggregate with effective local data instead.

## Decision

- Shared charts have immutable identities, name-generated slugs, metadata, ordered
  exact shared-Type roster pins, and directed known cells. Administrators author
  them. A world has at most one local chart, independently versioned and editable
  only by its eligible owner/collaborators. Shared charts may have multiple names.
- Local charts have exact same-world Type roster pins, optional exact shared-chart
  source, and explicit source-to-local roster mappings. Each shared identity maps
  to at most one local identity and vice versa. A mapped local Type must have that
  shared Type identity as its own source, but metadata source versions may differ.
  Missing mappings are allowed and never inferred from names or provenance.
- Chart lookup addresses canonical Type identity, not metadata revision. A variant
  pinning local Fire v1 can use the same Fire identity in a chart roster at v2;
  both exact pins and their names remain intact. Type metadata/adoption never
  edits charts or variant assignments. A mechanically distinct category needs a
  different Type identity. Shared/local identities are never interchangeable.
- A chart roster has at most 32 distinct identities, with contiguous explicit
  order. This is an independent software work bound (1024 directed pairs), not a
  game rule. Empty rosters are valid drafts. Diagonal and reverse pairs are ordinary
  independent authored relationships. Missing pairs are Unknown; endpoints outside
  the roster are Not covered. Neither means neutral or immunity.
- Store sparse known cells with exact reduced nonnegative rational factors.
  Numerator/denominator inputs are bounded at 1,000,000; denominator is positive;
  normalize before no-op comparison, including zero to 0/1. No floating point.
  Explicit 1/1 is neutral and 0/1 is no effect. Checked pairwise lookup only;
  multi-Type composition, damage, rounding and executable battle rules are deferred.
- Independent chart revisions seal complete metadata, roster snapshots, cells,
  source and mappings. Typed relational rows carry queried fields and exact FKs;
  JSON is the immutable snapshot, not the live relationship database. Live heads,
  immutable revisions and audit commit together under `BEGIN IMMEDIATE`.
- Updating metadata/roster keeps known cells whose identities remain. Removed
  members lose their cells/mappings only in the new revision, with explicit form
  help; old history remains. New members start Unknown. A row save replaces that
  attacking Type's cells atomically; blank entries mean Unknown. Filling all
  remaining Unknown cells with neutral is a separate explicit confirmed command.
- Source adoption compares old shared source, current effective local chart, new
  source and proposed result. The editor explicitly chooses mappings and either
  retains all local cells or applies source cells to mapped pairs. Applying source
  also makes mapped pairs Unknown when the source pair is Unknown; unmapped pairs
  remain local. A GET preview uses the selected policy/mappings before final POST.
  Source adoption must advance the same shared chart pin and never updates Types,
  variant assignments or unmapped local relationships.
- All mutations carry the aggregate starting revision; stale revisions take
  precedence over dependency validation and require explicit conflict review.
  No-ops produce no revision/audit. Session, role and world grants are checked on
  the locked transaction, with no global administrator bypass for local charts.

## Persistence, addressing and compatibility

- Add migration 0004, leaving migrations 0001–0003 and all old JSON bytes unchanged.
  Chart revision snapshots have their own strict v1 schema. Existing world v1/v2
  snapshots retain their documented Type/variant membership meaning and do not
  acquire retrospective chart membership. The one-chart-per-world relationship
  is queried directly; future content packages must explicitly pin chart revisions.
- Shared/local chart tables and typed roster/cell/mapping tables enforce exact
  dependency scope. Sealed revision children cannot be inserted/updated/deleted
  afterward. Readers validate stored snapshot/typed-row/dependency agreement and
  fail closed on unsupported or inconsistent history rather than showing defaults.
- GET/HEAD use `/charts/{slug}` and `/worlds/{world-slug}/charts/{chart-slug}`,
  with `/edit`, `/rows/{attacking-Type-id}`, `/neutral`, `/adopt`, `/history` and
  `/history/{revision}` as appropriate. Existing ID compatibility, reserved slug
  escaping, raw query preservation and ID-bound POST principles apply. Charts
  have their own public history reader; old history-kind contracts are unchanged.
- New binaries require migration 0004; migrate deliberately after a SQLite backup.
  No migration or server startup seeds charts or reinterprets historical typing.

## Browser workflow

Use semantic read-only tables with row/column headers at desktop and a vertical
pair list at narrow widths. Edit one attacking row through native labeled fraction
inputs. Show explicit Unknown/Neutral/No effect text, not color-only states. Bound
forms to 32 KiB, retain all draft strings/errors/reasons, show newer saved state
on conflicts, and require reviewed resubmission. Avoid a custom spreadsheet widget
and additional dependencies. Creation/roster forms choose exact pins explicitly;
source selection/preview GETs are deliberate reloads before editing.

## Fixtures and verification

Add a small explicitly synthetic shared lab chart and independent ClassicLab and
VariantLab charts, including original Mist relationships and deliberately different
values. They demonstrate authoring, not official Pokémon effectiveness or a full
ruleset. Extend the complete `seed-fixtures` transaction with all three charts and
immutable creation-baseline checks. Existing targeted creature/reference/world
commands keep their documented scopes and do not seed charts. Reruns preserve
later chart edits and all existing content/accounts/sessions. A late chart failure
rolls back bootstrap, invitations, old fixture groups and chart rows together.

Test exact numeric normalization/bounds, Unknown/neutral/outside-roster distinctions,
directionality/diagonals, metadata-version compatibility, exact references and scope,
roles/grant revocation, stale/no-op behavior, mapping ambiguity, adoption and source
removal, snapshot sealing and fail-closed reads, migration byte preservation, full
seed/reruns/late rollback, readable addresses and keyboard workflows at both widths.
Run fmt, Clippy with warnings denied, workspace tests and the headless browser suite.

The dedicated Plan agent is unavailable. A read-only exploration/planning review
preceded approval; bounded general agents may implement the agreed contracts.
