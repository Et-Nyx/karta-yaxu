# 0002 — Independent Polycentric proof of concept

> **Status: moved out of scope (deferred).** The proof of concept was extracted
> from this repository to `/home/eter/experiments/polycentric` and the local
> server stack was removed. The main project now ships a placeholder
> authentication foundation with a seam for a future Polycentric identity
> provider; see decision 0003. This record is retained for history only.

## Context

The maintainer approved a local proof of concept for self-hosted Polycentric
identity creation, passwordless authentication, device pairing, and optional
Harbor interoperability. This is an experiment, separate from wiki authoring.

## Alternatives

1. Depend on FUTO-operated services: does not satisfy independent operation.
2. Write a new identity server: unnecessarily takes ownership of protocol rules.
3. Run the upstream server and reuse upstream verification and protocol types:
   selected for the experiment.

## Decision

Pin Harbor to `1432692567cc0718f1dd3f63fb8b73e6238ac6a6` from
`https://code.futo.org/harbor/harbor.git`. Keep the upstream checkout, runtime
state, and private identity backups outside tracked source. Reuse its MIT-licensed
common Rust library for JWT and genesis-rooted identity-chain verification and
its generated browser protocol types. Do not copy the separately licensed app.

Keep the experiment in `experiments/polycentric`, with its own dependencies and
lockfiles. The existing wiki remains a separate application and keeps SQLite as
its planned game-data store. The upstream protocol service owns PostgreSQL,
Kafka and S3-compatible blob storage. No public deployment is authorized here.

The browser creates and holds keys. The application backend accepts proofs only
for its own one-time login challenge, validates the current identity chain from
a fixed operator-configured local protocol server, checks explicit membership,
and issues an opaque expiring session. Membership is not granted merely by
creating an identity. Avoid accepting arbitrary backend fetch URLs from clients.

The owned server is still trusted to provide fresh identity history: signatures
prove authenticity, not completeness. Local verification must fail on gaps and
observed rollback rather than quietly accepting a stale valid prefix. State
limits and timeouts bound validation and network work. Session expiry and logout
are part of the experiment; durable team account management is later work.

A focused browser client may build protocol envelopes using upstream generated
types and established crypto libraries, with upstream validation as the
compatibility check. The UI must distinguish device authorization from a single
login. Pairing and backup formats need real interoperability evidence before
being called Harbor compatible. External servers are opt-in, not defaults.

## Consequences and acceptance

This adds substantial optional infrastructure and browser tooling. It does not
establish a zero-cost hosting solution. Prove local creation, login, membership
denial, logout, recovery and pairing before promoting this into the main app.
Test independence with FUTO endpoints unavailable and report separately any
Harbor-client tests that could not run. Never substitute a mock for live-server
evidence without labeling it.

The Plan/Build agent types are unavailable. A general planning reviewer examined
the pinned Rust verifier and protocol APIs; implementation uses the available
agents in their place. Record actual checks and blockers in the experiment README.
