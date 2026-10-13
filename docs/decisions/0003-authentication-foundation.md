# 0003 — Authentication foundation and identity-provider seam

Refined by decision 0004, which records session lifecycle and clarifies the
provider integration boundary.

## Context

The self-hosted Polycentric experiment (decision 0002) required its own Rust
server, PostgreSQL, Kafka, and object storage. That infrastructure is larger
than the wiki foundation needs and was pulling the main project out of scope.
The maintainer asked to extract it, keep Karta Yaxu clean, and still build the
authentication foundation that later milestones depend on.

The project needs, before it can attribute edits: invite-only individual
accounts, server-side sessions, and a stable local user identity that revision
history can reference. Passwordless Polycentric login remains the intended
long-term direction.

## Alternatives

1. Keep the self-hosted Polycentric server inside this repository. Rejected:
   multi-service infrastructure and operational burden disproportionate to the
   current milestone; it obscured the application foundation.
2. Depend on FUTO-operated Harbor servers as the interim identity provider.
   Rejected for now: reintroduces an external dependency and another party's
   availability, contradicts the control and zero-cost goals, and the verified
   browser integration needed a large WASM/Expo toolchain.
3. Ship a clearly labelled local placeholder authentication behind an
   application-layer seam, and defer Polycentric to its own project. Selected.

## Decision

- The Polycentric experiment is moved out of the repository to
  `/home/eter/experiments/polycentric`, untracked, with its local stack removed.
  Decision 0002 is retained as history but superseded.
- Authentication is a real application boundary. A single identity-provider port
  maps a provider-specific credential to a canonical `Subject` (stable provider
  id, provider subject, display label). Use cases and routes depend on the port,
  never on a concrete provider.
- Ship one development-only **placeholder** provider. It has no passwords: an
  operator invites identities, and first sign-in creates the local account. It is
  gated behind an explicit environment flag and labelled in code and UI as
  insecure. It must not be exposed beyond local development.
- Persistence uses SQLite via SQLx: `users` (immutable id, provider + provider
  subject, role, disabled flag), `sessions` (hashed opaque token, expiry,
  revocation), and `invites`. Foreign keys are enabled per connection.
- Sessions use opaque random tokens stored only as a hash; cookies are
  `HttpOnly` and `SameSite=Strict`, with `Secure` enabled when configured for
  HTTPS. The actor id is taken only from the session, never from a form field.
- A future Polycentric provider implements the same port and adds its own
  credential-specific browser bootstrap. Preserving existing local user ids
  requires an explicit identity mapping; switching adapters alone is insufficient.

## Consequences

- The placeholder is not secure, stores no passwords, and requires explicit
  opt-in. Because there are no passwords yet, no password hashing is
  introduced; if passwords are ever added, an established implementation such
  as Argon2id is required.
- Local loopback HTTP omits `Secure`; this exception is env-controlled and does
  not apply under HTTPS. CSRF protection and Origin checks cover cookie-based
  mutations.
- Roles (viewer/editor/administrator) and invite-only membership start here but
  account management is a CLI, not an admin UI, in this milestone.
- Bot and moderation flows from the Polycentric protocol are out of scope.
- The stable local user id remains the attribution key. Identity linking/mapping
  and the real provider's credential contract need their own decision before that
  integration; this foundation settles neither.
