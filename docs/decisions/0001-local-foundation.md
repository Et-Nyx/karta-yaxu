# 0001 — Karta Yaxu local foundation

## Context

The maintainer approved the proposed Rust/server-rendered stack and selected a
local read-only creature browser as the first milestone. The product name is
**Karta Yaxu**. No game stat model has been selected.

## Alternatives

- A separate JavaScript frontend adds an API and build toolchain before needed.
- Django offers more built-in authoring features but introduces another backend language.
- A database immediately adds migration and attribution work to a read-only prototype.

## Decision

Use Axum/Tokio for HTTP and shutdown, Askama for escaped HTML, and Serde/serde_json
for local fixture decoding. Tower is a test-only dependency for router workflow
tests. Use a pinned Nixpkgs development shell and matching Rust toolchain file.
Keep game-data independent of HTTP; the web application owns catalog use cases;
tools validates the same fixture format. No repository abstraction is needed yet.

The initial internal fixture format contains an immutable opaque ID, a separate
slug, display name, and description. IDs are assigned once and are not derived
from names. Reject duplicate IDs/slugs, unknown fields, malformed IDs/slugs, and
blank or oversized text. Keep validated fields private. This is not an engine
export contract or a settled game schema.

Use a small set of invented, explicitly labeled development placeholders first.
PokeAPI remains the planned import adapter; it needs its own provenance and
idempotency work. Embed fixtures and CSS in the binary for cwd-independent runs.
The local browser binds only to loopback and provides no mutation endpoints.

## Consequences

Changes to bundled fixtures/templates/CSS require rebuilding. No database,
accounts, audit history, engine export, or battle mechanics exist in this slice.
SQLite/SQLx will be introduced with the attributed authoring workflow. Private
versus public wiki access remains open; this local fixture preview settles no
team access policy. There are no migrations or existing data compatibility effects.

Official Axum 0.8.9, Askama 0.16.1, Tokio 1.53.2 and serde_json 1.0.154 docs
were reviewed during implementation; Cargo.lock records resolved versions.
The requested Plan/Build agent types were unavailable, so planning and building
were performed directly. The reference file currently exists under the spelling
`PROJECT_GUIDEINES.md`; its contents are unchanged.
