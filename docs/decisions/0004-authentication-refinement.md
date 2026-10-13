# 0004 — Authentication reliability and session lifecycle

## Context

Review of the development authentication foundation found ambiguous CLI startup,
read-before-write WAL contention, weak schema checks, and misleading responses to
storage failures. The maintainer approved an authentication-only refinement and
selected permanent session revocation when disabling an account.

## Alternatives

- Suspend sessions while disabled and restore them on re-enable. Rejected: old
  credentials should not regain access after the operator disables an account.
- Retry deferred transactions after snapshot conflicts. Prefer acquiring the
  writer first: fewer moving parts, with the existing bounded busy timeout.
- Add an auth framework or external rate-limit service. Unnecessary for this
  local placeholder; use the existing libraries and bounded process-local state.

## Decision

- Use SQLx-managed `BEGIN IMMEDIATE` transactions for invite claims, invitations,
  session creation, and disable/revoke. Keep transactions short; no unbounded
  retries. Session creation checks the enabled account under the same write lock.
- Disable the account and revoke all its sessions atomically. Re-enabling requires
  a new sign-in. Unknown account ids fail explicitly; repeated logout is safe.
- Membership operations are trusted local operator CLI use cases, authorized by
  access to the executable and database, not by browser-supplied identities.
  These operations must not be exposed as HTTP handlers without application-layer
  administrator authorization. Authentication use cases enforce invite membership
  and enabled-account policy themselves.
- Validate commands before configuration/database access. Only `migrate` creates
  the database. Startup and other commands require an existing database and check
  migration versions, success, checksums, and required columns without migrating.
- Session lifetime defaults to seven days, accepts 1–2,592,000 seconds (30 days),
  and uses checked expiry arithmetic. Validate it at configuration and service
  construction boundaries.
- Authentication forms are limited to 4 KiB. Login POSTs share one fixed window of
  30 attempts per 60 seconds per server process. Rate limiting runs before form
  parsing/provider/database work, counts rejected attempts too, and returns 429
  with `Retry-After`. Its storage is constant-sized; restarting resets the window.
- Storage and randomness failures produce explicit temporary-unavailability
  responses. Failed logout preserves cookies for retry and never claims success.
  Validate CSRF token shape and reject duplicate cookies.
- Log operation and database error category/code only, never SQL error text that
  could include data, credentials, tokens, or raw requests.

## Consequences and compatibility

No schema migration or dependency is required. Existing accounts, invite records,
and session hashes remain compatible. Databases with incompatible migration
history now fail startup rather than failing later during authentication. Existing
session rows are revoked when an account is next disabled; deploying this change
does not modify existing data automatically.

The global login limit is suitable for the local preview, but one caller can
consume the shared allowance. Revisit client-aware limiting before team hosting.
The provider remains an explicitly insecure development placeholder, gated by
`KARTA_DEV_AUTH=1`. A future provider needs credential-specific browser bootstrap
and explicit identity mapping to preserve existing local user ids; the current
provider port does not implement that migration.

The requested Plan/Build agent types are unavailable in this harness; the plan
was reviewed directly and approved before implementation.
