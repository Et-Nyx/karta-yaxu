# 0007 — Name-generated slugs and readable object addresses

## Context

The maintainer requested slug-based object page addresses and automatic slugs
derived from display names, including regeneration on rename. They approved the
small `deunicode` transliteration dependency. Canonical ids still identify durable
objects, references, versions, permissions and audit records.

## Alternatives

- Replace canonical ids with names/slugs: rejected; renaming must not alter identity.
- Make form slugs globally unique: unnecessary; forms belong to species and
  variants belong to worlds, with existing scoped uniqueness constraints.
- Redirect or resolve mutations through mutable names: rejected for generated
  forms; an open draft must remain bound to its original immutable target even
  when a slug is renamed or reused.
- Store historical aliases: deferred. They conflict with allowed slug reuse and
  require a separate durable alias policy/migration.

## Decision

### Automatic slugs

- User-created records generate a lowercase ASCII slug from their normalized
  display name using pinned `deunicode` 1.6.2, separators collapsed to hyphens,
  with the existing 80-character cap. Empty transliteration falls back to `object`.
- On a real display-name change, regenerate within the same write transaction;
  on unchanged names, preserve the existing slug, including older manually assigned
  values and collision suffixes. Description-only edits and pin-only adoption do
  not rename an address.
- Resolve collisions in the existing scope: species/world globally, forms within
  their species, variants within their world. Choose the first available suffix
  (`name`, `name-2`, …), trimming the base to fit. Exclude the edited object's id.
  Candidate search is bounded, and existing unique constraints remain authoritative.
- Browser forms no longer let editors choose a slug. Legacy submitted slug fields
  are ignored by user mutation use cases. Fixtures and historical snapshots retain
  their explicitly stored slugs and validation rules; initialization does not
  rewrite established baselines. No automatic database backfill is performed.

### Readable addresses

- Species: `/creatures/{species-slug}`.
- Forms: `/creatures/{species-slug}/forms/{form-slug}`.
- Worlds: `/worlds/{world-slug}`.
- Variants: `/worlds/{world-slug}/variants/{variant-slug}`.
- Existing edit/history/adoption/access GET suffixes follow these coordinates.
  Always derive coordinates from current rows, never pinned historical slugs.
- An outer adapter resolves these GET/HEAD addresses before the unchanged
  ID-oriented inner router matches. Use indexed, parameterized persistence reads
  through application methods, with exact parent scope and consistent read snapshots.
- Legacy ID GET/HEAD links temporarily redirect (307, no-store) to current readable
  addresses, preserving suffix and raw query. Existing stable ID links survive
  renames. No response HTML rewriting or new routing framework is introduced.
- Literal creation paths retain `new`. Encode a slug as `~{slug}` when it is `new`
  or exactly 32 lowercase hexadecimal characters. `~` is a URL discriminator, not
  stored content. Unescaped 32hex means id with no slug fallback; escaped 32hex
  means slug, avoiding identity collisions. Decode segments once, reject encoded
  separators/invalid escapes, and normalize accepted encoded characters.
- Non-GET/HEAD requests pass unchanged. Generated mutation forms keep canonical-ID
  actions; successful POSTs use 303 then land on readable GET addresses. Failed
  POST drafts intentionally retain their ID address for immutable target binding.
  No mutation is performed by a slug lookup or redirect.

## Consequences and compatibility

No database migration, identity rewrite or permission change is required. Existing
slugs remain until a name changes; new slugs and renames are normally attributed
content mutations with optimistic concurrency. Old slug addresses may become 404
or name a replacement object if reused; canonical-ID permalinks remain durable.
Parent renames update nested child addresses without changing child definition
versions or source pins. Transliteration versions are pinned through Cargo.lock;
changing the library later does not rewrite existing names/addresses automatically.

The requested Plan/Build agent types are unavailable; read-only route assessment
preceded this record. Available general agents may build bounded modules.
