# 0009 — Complete atomic development fixture setup

## Context

The maintainer approved making `admin seed-fixtures` initialize all bundled
content, including an immediately usable administrator on an empty development
database. The former creature-only command left Types/worlds absent and required
a browser sign-in between setup steps. Decision 0008 deliberately required a real
existing owner; this decision supersedes that restriction only for the explicit
development bootstrap described below. Other accounts remain invite-only.

## Alternatives

- Keep separate manual steps: explicit but unnecessarily awkward for local setup.
- Call the existing seed methods sequentially: rejected because each commits its
  own transaction and a late error would leave partial content or an account.
- Bootstrap whenever no enabled administrator exists: rejected because existing
  users, including disabled accounts, must not cause implicit privilege recovery.

## Decision

- `admin seed-fixtures [--owner-user-id <id> | --admin-subject <name>]` loads the
  original creatures/default forms, cached reference Types/forms, and both lab
  worlds with local Types/variants. The options are mutually exclusive.
- Validate arguments and bundled manifests before database writes. Only explicit
  `admin migrate` creates/migrates a database; setup requires a compatible schema.
- Resolve ownership under the same `BEGIN IMMEDIATE` lock as all writes:
  1. An explicit owner must be an existing enabled editor/administrator. Existing
     fixture worlds must already belong to that owner.
  2. Otherwise retain the verified common owner of complete existing lab worlds,
     provided that owner is still enabled and eligible. Never transfer ownership.
  3. If there are no users at all, bootstrap a placeholder administrator only with
     development authentication enabled (`KARTA_DEV_AUTH=1`, also accepting the
     existing configuration spellings `true`/`yes`). The default username is
     `admin`; `--admin-subject` uses normal placeholder username normalization.
  4. Otherwise select the sole enabled administrator. Zero or multiple candidates
     require explicit `--owner-user-id`; no existing account is promoted/enabled.
- `--admin-subject` names only a potential bootstrap identity. Once users exist it
  does not select, rename, or create another account; normal ownership resolution
  applies, so rerunning the same setup command remains idempotent.
- Bootstrap creates a random immutable user id and no session. If a matching
  pending placeholder administrator invitation exists, consume it; otherwise
  create and consume an invitation to record operator-authorized membership. A matching
  viewer/editor invitation rejects setup rather than changing its role. Other
  invitations are unchanged. Bootstrap is a trusted operator action, not a web
  endpoint or authentication-provider proof.
- A single application use case coordinates account creation, invitation changes,
  original/reference/world content, ownership, versions, provenance and audit in
  one transaction. Shared transaction-scoped helpers also support targeted seed
  wrappers. Print successful account/setup information only after commit.
- Retain `seed-reference-fixtures` and `seed-world-fixtures --owner-user-id` with
  their existing scopes and no account creation. Add `seed-creature-fixtures` for
  the old creature-only behavior. No schema or dependency changes.
- Compare immutable fixture creation baselines; preserve edits, pins, archives,
  sessions and collaboration grants. Original-only/reference-only databases are
  valid inputs. Partial reference batches, partial lab worlds, inconsistent owners,
  provenance drift and collisions reject the operation without repair.

## Consequences and verification

Fresh development setup is migrate, seed, run, then sign in as the printed
bootstrap subject. Existing databases retain their users and fixture ownership.
Content remains attributed to `bundled-fixture-loader`; the bootstrap user owns
worlds rather than impersonating the content author. The placeholder provider is
still development-only and passwordless.

Regression tests cover bootstrap gating/normalization, invitation consumption and
conflicts, owner selection/eligibility, retained ownership, concurrency, late
rollback of all content/account/invitation changes, reruns preserving edits and
grants, and first browser sign-in resolving the same bootstrap user at desktop and
320px. Run formatting, Clippy, workspace tests and the headless browser suite.

The dedicated Plan agent is unavailable. A read-only general-agent design review
served as its substitute before implementation, per the repository workflow.
