# Creature Collector Design Wiki and Battle Lab

Project reference for maintainers, contributors, and AI agents.

Document date: 2026-10-12. Status: initial architecture proposal, ready for review and incremental implementation.

## 1. Scope and decision status

The project is an educational web application that serves as the main design database and wiki for an indie creature collector game. Designers and developers must be able to browse, create, and edit game objects through the browser, inspect changes attributed to individual users, export validated data to the game engine, and test combat in PvE and PvP simulations.

### Confirmed requirements and preferences

- The game engine is being written in Rust and Lua.
- Development takes place on NixOS Linux.
- The maintainer has experience with HTML, CSS, SQLite, and programming, but is new to deploying a complete web product alone.
- Control, learning, fine tuning, and reliability take priority over convenience or speed.
- Keep systems simple and apply Unix and suckless principles pragmatically.
- Aim for no monetary hosting cost, while acknowledging operational limitations.
- Use PokeAPI data as temporary development fixtures before original game data exists.
- Support team editing, per-user history, engine export, and eventually PvE and PvP combat testing.

### Proposed defaults, requiring confirmation during implementation

The earlier assistant replies are not available in this reference's source context. Therefore the exact libraries, repository layout, hosting approach, and policies below are recommendations rather than a claim that every choice was previously agreed.

Use a modular Rust application, SQLite, server-rendered HTML, modest CSS, and small amounts of browser JavaScript. Share a deterministic Rust combat library with the game engine and embed Lua only where game mechanics need scripting. Start with one server process and one durable database. Select hosting only after confirming persistence and resource requirements.

## 2. Product boundaries

### Initial product

1. Invite-only accounts and role-based access.
2. Creature and move listings, detail pages, and validated edit forms.
3. Search, filters, references between objects, and readable documentation.
4. Transactional revision history with attribution and conflict detection.
5. Versioned JSON export validated by an engine-side importer.
6. A repeatable local development environment and tested backup restoration.

### Later capabilities

- Abilities, items, types, status effects, evolution rules, encounters, and richer mechanics.
- Deterministic simulation through a CLI, followed by a browser PvE interface.
- Turn-based PvP with server authority, reconnection, and replay.
- Lua-authored effects once their execution contract is defined.

Keep real-time collaborative document editing, public signup, microservices, a plugin marketplace, and distributed simulation outside the initial scope. Add them only when a concrete requirement justifies their maintenance cost.

## 3. Technology direction

| Concern | Proposed choice | Reason and boundary |
| --- | --- | --- |
| Backend | Rust with Axum and Tokio | Align with the engine; explicit types and errors; HTTP and later WebSocket support. |
| Database | SQLite through SQLx | Familiar SQL, transactional writes, migrations, and few operational dependencies. |
| HTML | Askama templates | Render pages on the server and use normal links and forms. |
| Browser behavior | Small JavaScript modules; consider HTMX for partial updates | Add interactivity where it improves usability. TypeScript is optional as browser code grows. |
| Styling | Semantic HTML and a small CSS token/component system | Retain control while ensuring consistent forms, tables, focus states, and spacing. |
| Serialization | Serde and versioned JSON | A readable, language-neutral engine boundary. |
| Combat | Shared Rust library | One mechanics implementation reused by CLI, server, and engine. |
| Scripting | Lua through mlua, matching the engine's Lua runtime | Flexible effects behind a narrow, deterministic host API. |
| Development | Nix flake/dev shell and locked Rust toolchain | Reproducible tools on NixOS. |
| Packaging | Native executable first; container if the host requires it | Avoid maintaining two deployment paths without a need. |

These choices are a starting point, not a reason to add every dependency immediately. Review current official documentation and select compatible versions at implementation time. Commit Cargo.lock, flake.lock, and a Rust toolchain specification; avoid floating production builds.

### Why Rust and Lua rather than a full TypeScript stack?

Rust fits the backend and shared mechanics because the engine already uses it. This reduces duplicated rule implementations and creates useful learning overlap. The cost is a steeper learning curve, more compilation time, and more care around asynchronous code and ownership.

Lua is appropriate for curated game effects and balance rules. Keep HTTP handling, authorization, persistence, and protocol validation in Rust. Browser HTML and CSS still matter: using Rust and Lua on the server does not remove the browser's need for JavaScript when implementing rich interaction. Neither TypeScript nor WebAssembly is mandatory for the first version.

### UI framework policy

Start with native browser controls and a coherent CSS design system. If writing accessible components becomes a burden, adopt a small maintained styling/component solution after checking its accessibility and license. Do not choose a large frontend framework merely to render CRUD forms. Reassess when the battle interface has enough client state to justify it.

### Docker policy

Docker is optional. Nix describes development dependencies; a container describes a deployable runtime environment. They solve related but distinct problems. Begin with a native release binary and a service definition when the target supports that model. If the target requires a container, build a small image, run as a non-root user, mount durable data separately, and never bake secrets or the live SQLite file into the image.

## 4. Architecture: modular monolith

Keep one deployable application with explicit internal boundaries. Separate pure domain behavior from HTTP, storage, templates, and external services.

| Layer | Responsibility | Must not own |
| --- | --- | --- |
| Domain | Object types, invariants, battle state transitions | HTTP requests, templates, database connections |
| Application | Use cases, authorization policy, transaction orchestration | Page layout or SQL scattered through routes |
| Persistence | Queries, migrations, durable state | Combat rules or browser interactions |
| Web | Routing, request parsing, sessions, forms, response rendering | An independent copy of game mechanics |
| Adapters | PokeAPI import, engine export, script integration | Core rules that depend on a specific external service |

HTTP handlers translate inputs into application commands. Application services validate policy and coordinate repositories. Repository implementations translate to SQL. Introduce interfaces at real boundaries; avoid a generic repository framework or a trait for every function.

The shared combat library must build and run without a web server or database. Keep mutable application globals out of the domain. Explicitly pass rules, state, commands, and deterministic randomness.

One server is a deployment simplicity choice and a single availability dependency. Reliability comes from recoverability, tested backups, clear failure handling, and reproducible releases; it does not imply high availability.

## 5. Suggested repository structure

```text
creature-lab/
  PROJECT_GUIDELINES.md
  AGENTS.md
  README.md
  Cargo.toml
  Cargo.lock
  rust-toolchain.toml
  flake.nix
  flake.lock
  crates/
    game-data/              # Shared typed definitions and validation
    battle-core/            # Pure Rust mechanics; no HTTP or SQL
    game-scripting/         # Lua runtime integration, when needed
    web-app/
      src/
        main.rs
        config.rs
        domain/
        application/
        persistence/
        web/
      migrations/
      templates/
      static/
        css/
        js/
    tools/                  # Import/export and simulation CLI
  schemas/                  # Export schema and compatibility documentation
  rules/                    # Reviewed Lua scripts, if adopted
  fixtures/                 # Small reproducible test data; provenance
  tests/                    # Cross-component and end-to-end checks
  docs/
    architecture/
    decisions/              # Short architecture decision records
    operations/             # Deploy, backup, restore, and incident runbooks
  deploy/                   # Service/container config for chosen target
```

Create directories and crates only when they have code or documentation to hold. Start with game-data, web-app, and a tools command; introduce battle-core when simulation work begins. Keep runtime data, database files, credentials, and generated exports outside tracked source.

## 6. Data model and schema rules

Model the concepts the game actually needs. A reasonable starting set is users, sessions, creatures, moves, types, relationship tables, object revisions, and audit events. Abilities and scripts can follow when their semantics are known.

- Use immutable object identifiers distinct from editable display names and slugs.
- Never use a PokeAPI identifier as the application's canonical identifier.
- Store reference relationships with foreign keys or explicitly validated typed references.
- Put frequently queried fields in typed columns; avoid one universal entity table containing arbitrary JSON.
- Use JSON for revision snapshots or deliberately flexible, schema-validated effect parameters.
- Establish numeric ranges, required fields, uniqueness, and reference rules in domain validation and appropriate database constraints.
- Add indexes for actual list, filter, relationship, and history queries.
- Preserve UTC timestamps; display local time where useful.
- Represent archived/deleted status explicitly for objects referenced by exports or history.
- Version database migrations, export schemas, rulesets, and battle protocols separately.

Example creature fields: id, slug, display_name, description, base_stats, type references, move relationships, revision, archived_at, created_at, updated_at. The exact stat names and ranges remain a game-design decision.

### SQLite operating rules

Enable foreign keys on every connection. Consider WAL mode for overlapping reads and writes on a single machine, use a bounded busy timeout, and keep write transactions short. SQLite still has one writer at a time; it is suitable for the proposed small team, not a guarantee of unlimited edit throughput.

Keep the database on a durable local filesystem appropriate for SQLite locking. Do not put a WAL database on a shared network filesystem or assume multiple independent app replicas can safely share it. Bound the connection pool and keep blocking database or CPU work from stalling asynchronous request handling.

Consider PostgreSQL only when measured contention, deployment topology, or operational needs justify migration. Choosing a managed remote database solely to fit a free hosting tier changes the initial SQLite architecture and should be recorded as a decision.

## 7. Editing, audit history, and conflicts

Every application-mediated game-data mutation must commit its object update, revision, and audit event in one transaction. A failed write must leave none of them partially applied.

Record at least:

- Audit/revision identifier and object type/id.
- Authenticated actor id, timestamp, and action.
- Previous and resulting revision numbers.
- Before/after snapshots or a documented reconstructable representation.
- Optional change reason and a request correlation id.

Attribute imports to the initiating user with an import source or to an explicit system actor. Never accept the actor id from an editable browser field. Disable accounts without losing their historical identity.

Use optimistic concurrency: each edit sends the revision it started from. Update only when that revision is still current; on conflict, show the newer record and let the editor compare and resubmit. Do not silently overwrite someone else's change.

Reverting creates a new attributed revision, rather than deleting history. History should show readable field changes and support inspecting earlier snapshots. Keep this simpler than full event sourcing: the normal tables remain the current state, and revisions support review and recovery.

Audit logs are append-only through the application, but a database administrator can modify the database. Do not describe this as tamper-proof. If stronger evidence is needed, design independent retention or integrity controls explicitly.

## 8. Authentication and authorization

Start invite-only. Use individual accounts so edits can be attributed. Choose a maintained authentication library or a suitable external identity integration; document the choice before exposing team write access. If storing passwords locally, use an established password hashing implementation such as Argon2id, with safe parameters and a documented reset path.

| Role | Default capability |
| --- | --- |
| Viewer | Read objects and history; run allowed simulations |
| Editor | Viewer access plus create/edit/archive permitted game data |
| Administrator | Manage membership and roles; authorize sensitive operational actions |

Specify who can export and publish scripts separately. A designer's ability to edit numbers must not automatically grant arbitrary Lua execution.

Use server-side sessions with opaque random tokens, expiration, logout/revocation, and Secure, HttpOnly, appropriately SameSite cookies under HTTPS. Apply CSRF protection to cookie-authenticated mutations. Enforce authorization inside each use case, including JSON endpoints and WebSocket commands.

Escape rendered text, sanitize any supported rich text, use parameterized SQL, validate payload sizes, and limit abusive login/simulation requests. Do not log passwords, session tokens, secrets, or sensitive raw requests. Recovery procedures must not rely on a casually exposed admin endpoint.

## 9. Interface and accessibility

The primary users include nontechnical designers. Make everyday work obvious:

- Consistent navigation: Creatures, Moves, Types, other objects, Battle Lab, History.
- Searchable/filterable lists with understandable empty states and pagination.
- Detail pages with clear references and direct edit links.
- Labeled forms with help text, units, sensible defaults, and errors beside fields.
- Save/cancel controls, preservation of entered values on error, and unsaved-change prompts where needed.
- Clear archive/delete language and confirmation for consequential destructive actions.
- Human-readable change history showing who changed what and when.
- Explicit loading, failure, reconnecting, and conflict states.

Prefer semantic HTML, keyboard access, visible focus, sufficient contrast, and responsive layouts. Never communicate validation or combat state using color alone. Test the central browse/edit/export workflows with someone unfamiliar with the implementation. A small usable design system matters more than decorative complexity.

## 10. Engine export contract

SQLite is the authoring store. The engine consumes a versioned data package, not a live production database or an HTTP dependency.

An export should contain schema_version, content/ruleset identifiers, canonical object ids, validated object collections, and script hashes or reviewed script assets where applicable. Include build/export provenance in a manifest.

1. Authorize the request and read a consistent database snapshot.
2. Validate all references, allowed ranges, required fields, and script compatibility.
3. Serialize in deterministic order with documented numeric representations.
4. Produce a manifest and checksums; keep generation timestamps separate from canonical content hashing where necessary.
5. Validate through the real engine-side importer or its shared parsing library.
6. Make the resulting package available atomically, only after validation succeeds.

Define whether exports include only active objects and how dependencies of archived objects are handled. Do not export sessions, credentials, user profile data, or administrative history in game-content packages.

Round-trip tests should prove that an exported fixture can be imported and used by the game. Export schema changes require compatibility decisions and migration notes. A battle must identify the exact content package, ruleset, scripts, and simulation version it used.

## 11. PokeAPI placeholder strategy

Treat PokeAPI as an import adapter, not a runtime dependency for browsing or combat.

- Import a small curated subset for early fixtures.
- Cache source responses or retain reproducible fixture snapshots where permitted.
- Store source name, external id/URL, retrieval time, and transformation version.
- Map imported data into internal types and clearly label it as placeholder content.
- Use timeouts, bounded retries, and conservative request behavior.
- Keep import idempotent and preview changes before replacing locally edited values.
- Disable external fetching in normal tests; use local fixtures.

Review current PokeAPI usage guidance and the rights associated with source data, names, and artwork before redistributing them. API availability does not establish permission to ship Pokémon assets in the final game. Keep placeholder removal straightforward and separate original assets from imported ones.

## 12. Combat simulation contract

Design battle-core around explicit inputs and deterministic transitions:

```text
step(rules, content_snapshot, battle_state, validated_command, rng_state)
    -> updated_state + ordered_events + updated_rng_state
```

The concrete Rust API may differ. The invariant is that all dependencies affecting outcomes are explicit.

- No database calls, network access, wall-clock reads, or ambient randomness inside mechanics.
- Use a chosen, versioned RNG algorithm with a recorded seed/state.
- Prefer integer or fixed-point arithmetic where cross-runtime determinism matters.
- Define ordering for ties and effect resolution; never rely on hash-map iteration order.
- Validate commands against current state and allowed actions.
- Emit structured events for damage, effects, state changes, and battle completion.
- Bound simulation work and define how invalid rules or script errors terminate a run.
- Store initial state, commands, rules/data versions, and RNG information for replay.

A seed alone does not guarantee a replay across code changes. Verify determinism with fixed fixtures, state/event hashes, and replay tests for the supported simulation version.

### PvE sequence

First implement a CLI simulation, then a browser battle against a simple legal-action policy. Add smarter AI only after mechanics and observability are stable. Test custom data without mutating a published ruleset in the middle of a running battle.

### PvP sequence

Begin turn-based. The server owns battle state and validates every command. Clients submit intent; they do not submit authoritative damage or resulting state.

Implement match authorization, turn ownership, command ids, duplicate handling, sequence numbers, bounded messages, reconnect snapshots, and an explicit timeout/forfeit policy. Use WebSockets when useful; ordinary request/response can prototype turn submission first. Persist committed turns and replay information so restarts have a defined recovery behavior.

Expose only the battle information a participant is allowed to see. Pin a rules/content snapshot at match creation so design edits cannot change an ongoing match.

## 13. Lua execution rules

Share the engine's exact Lua dialect/runtime assumptions. Lua, LuaJIT, and Luau are not interchangeable compatibility targets.

Use Lua for narrow effects and formulas only when declarative data is insufficient. Pass a documented context and bounded host functions; define allowed values, event emission, error handling, and state mutation semantics.

- Start with trusted, code-reviewed scripts stored in version control.
- Do not provide filesystem, shell, network, module loading, or secrets to scripts.
- Do not expose nondeterministic time or randomness; provide host-controlled RNG.
- Limit instructions/execution, memory, recursion, and generated event counts as applicable.
- Validate scripts before use, pin versions/hashes, and test their effects and failure behavior.
- Commit a battle transition only after all required script work succeeds, or apply an explicitly documented failure policy.

Do not assume mlua automatically makes arbitrary Lua safe. Its documented sandbox method is specific to Luau; other Lua variants require explicit controls. Resource limits and restricted globals are useful controls, but accepting hostile scripts requires a separate security design, potentially including process isolation. Defer arbitrary script uploads from the browser.

## 14. Unix and suckless principles in practice

| Principle | Application |
| --- | --- |
| Do one thing well | Keep authoring, validation, export, and combat as explicit modules and CLI commands. |
| Prefer composable tools | Let CLI commands consume/produce files or stdin/stdout where practical. |
| Use inspectable formats | SQL migrations, JSON packages, Lua scripts, Markdown documentation. |
| Make behavior explicit | Typed configuration, clear errors, visible state transitions. |
| Avoid needless dependencies | Add a library to solve a concrete problem; track its purpose. |
| Keep interfaces small | Narrow script API, stable export schema, minimal domain services. |
| Optimize for maintenance | Clear code and tested invariants before clever abstractions. |

Simplicity does not justify homemade cryptography, weakened authorization, absent backups, or inaccessible forms. A well-maintained library can reduce total complexity even when it adds a dependency.

## 15. NixOS development workflow

Provide a flake-backed development shell with Rust/Cargo, formatter, Clippy, SQLite CLI, and only the native build dependencies the selected libraries require. Add Lua tooling if scripting is introduced. Node is optional until frontend tooling actually requires it.

Document a clean-checkout workflow:

```bash
nix develop
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

These commands become required checks once the workspace exists. Provide explicit project commands for initializing/migrating a local database, seeding fixtures, starting the server, importing placeholders, exporting data, and running simulation. Document the actual commands rather than leaving aspirational examples in the final README.

Use an ignored local configuration file or environment variables for development secrets; commit only a safe example. Keep dev/test/prod databases separate. If SQLx compile-time checked queries are adopted, define the online or offline metadata workflow so builds do not depend on access to production.

## 16. Deployment and zero-cost constraints

No hosting provider is selected by this document. Free tiers, availability, quotas, persistence rules, and signup requirements change; verify current terms before committing to a target.

There is no unconditional promise of a reliable, permanently free public service. Distinguish zero additional spending from zero operating cost. An existing computer still uses power and connectivity; cloud free tiers impose limits and may disappear.

| Approach | Benefit | Constraint to resolve |
| --- | --- | --- |
| Existing machine, private team access | Full Rust/SQLite control with potentially no new subscription | Uptime, power, connectivity, secure access, and backups remain your responsibility. |
| Verified free cloud runtime with persistent disk | Public access without owning a server | Confirm Rust execution, durable disk, WebSockets, sleep policies, quotas, and no automatic billing. |
| Static hosting plus remote services | Simple static delivery | Cannot directly host this native Rust/SQLite application; requires a deliberate architecture change. |

Evaluate a deployment target against native executable/container support, durable local storage, HTTPS, WebSockets, CPU/memory limits, sleep/restart behavior, backup export, regions/latency, and spending controls. Reject ephemeral-only storage for the live SQLite database. Do not assume a persistent volume is included in a free runtime.

Start local, then expose a private team pilot, then publish only when auth, backups, and recovery work. Prefer a provider subdomain if avoiding domain-registration costs. If no suitable free target exists, keep the pilot private/on existing hardware or explicitly revise the architecture and reliability expectations.

### Release procedure

1. Run required checks and build a reproducible release.
2. Create a consistent database backup and retain the prior application artifact.
3. Review and apply migrations deliberately; avoid automatic destructive migrations.
4. Deploy with durable data mounted outside the release artifact.
5. Check readiness, login, a representative edit/history workflow, and export.
6. Observe errors and resource usage; execute the documented rollback if needed.

Code rollback does not automatically reverse schema changes. Prefer compatible staged migrations; document when restoration is required and what recent edits it would lose.

## 17. Operations, backups, and recovery

- Keep structured application logs with request ids and useful error context.
- Separate operational logs from durable user audit history.
- Provide liveness and readiness checks; readiness should reflect usable critical dependencies.
- Monitor or periodically inspect disk space, database growth, failed writes, and backup freshness.
- Set bounded HTTP, import, script, and simulation resource budgets.
- Use graceful shutdown and define what happens to in-flight edits and battles.
- Back up before migrations and on a documented schedule; retain copies off the serving machine.
- Use SQLite's consistent backup facilities or a properly quiesced procedure. Do not simply copy the main database file during live WAL writes.
- Include scripts/assets/configuration needed for recovery, while storing secrets separately and securely.
- Restore backups into a separate environment and verify data, attribution, and export before relying on the procedure.

Initial pilot targets to confirm: accept at most one day's lost edits, and aim to restore service within a few hours when the maintainer is available. These are design targets, not guarantees. Improve backup frequency if the team cannot accept that loss.

## 18. Testing priorities

Test meaningful invariants and user workflows, rather than mirroring implementation details.

| Area | Required evidence |
| --- | --- |
| Domain | Invalid values/references rejected; correct object rules |
| Persistence | Migration from supported versions; transactional rollback and constraints |
| Audit | Successful mutation attributed; failed mutation leaves no partial history |
| Authorization | Each role's allowed and denied operations tested server-side |
| Editing | Stale revision cannot silently overwrite a newer edit |
| Export | Consistent package, valid references, engine importer acceptance |
| Combat | Deterministic replay, legal commands, bounded termination, known scenarios |
| Lua | Approved effects, resource exhaustion, and error policy |
| PvP | Duplicate/reordered commands, reconnect, wrong user, restart behavior |
| Operations | Backup restoration and representative post-deploy smoke checks |
| UI | Keyboard browse/edit, readable errors, data retained after validation failure |

Use temporary databases and local fixtures. Add a small end-to-end test for the central editing workflow once the UI stabilizes. Keep network-dependent checks separate from normal tests. Benchmark only when measurements can answer a concrete performance concern.

## 19. Incremental roadmap and acceptance gates

### Phase 0 — Foundation

Record the stack decisions, initial object model, authentication approach, and export contract. Create the Nix/Cargo environment and a small server skeleton.

Gate: a clean checkout builds, tests, starts locally, and renders a page using documented steps.

### Phase 1 — Reliable authoring

Implement accounts, roles, creatures/moves, forms, references, history, optimistic conflicts, migrations, and backup/restore.

Gate: two users can edit safely; unauthorized writes fail; every successful mutation is attributed; a backup restores correctly.

### Phase 2 — Data interchange and team pilot

Add curated PokeAPI fixtures, validated export, engine import, and a deployment on a verified target.

Gate: the engine consumes an export; the wiki works without PokeAPI; persistence survives restart; the pilot satisfies the documented cost constraints.

### Phase 3 — Combat core and PvE

Implement deterministic battle-core, replay fixtures, a CLI, then a browser PvE interface. Introduce Lua only if required by real mechanics.

Gate: known battles replay identically for the pinned version; invalid commands and scripts fail predictably; custom content is tested against pinned snapshots.

### Phase 4 — PvP

Add server-owned matches, authenticated commands, reconnection, sequencing, timeouts, and restart recovery.

Gate: disconnects and duplicate commands cannot corrupt a match; participants see only permitted information; completed battles replay.

### Phase 5 — Refine from evidence

Improve accessibility, editor workflows, balance tools, and performance based on team feedback. Reconsider database or deployment topology only when observed limitations justify it.

## 20. Instructions for AI agents and contributors

1. Read this reference, AGENTS.md when present, and relevant decision records before changing architecture.
2. Distinguish confirmed requirements from proposed defaults. Record material choices in a short decision record with context, alternatives, decision, and consequences.
3. Work in small reviewable changes with a clear user-visible goal.
4. Preserve public data identifiers, export compatibility, audit attribution, and deterministic battle behavior.
5. Keep routes thin, business rules explicit, and shared mechanics inside the shared core.
6. Never weaken server-side authorization, bypass history, silently overwrite edits, or introduce arbitrary script execution for convenience.
7. Do not add infrastructure or dependencies without explaining the concrete requirement.
8. Do not touch production data, execute destructive migrations, publish a deployment, or create paid resources without appropriate task authorization.
9. Keep secrets and personal account data out of source, fixtures, exports, and logs.
10. Run checks appropriate to the change; report what passed, what was not run, and remaining limitations.
11. Update the relevant schema, runbook, README, or decision record when behavior changes.
12. If a requested change conflicts with these guidelines, explain the conflict and propose the smallest coherent solution. The user's explicit current instruction takes precedence.

Definition of done: the intended workflow works; invariants and relevant checks pass; error paths are understandable; compatibility and migration effects are documented; no unexplained dependency or deployment cost has been introduced.

## 21. Decisions still open

- Exact Rust library versions and frontend enhancement approach.
- Authentication provider/library, invitation, password reset, and account recovery.
- Creature stat model, mechanics scope, and Lua dialect/runtime compatibility.
- Editor permission to export, archive, or publish scripts/content.
- Wiki visibility: private team, public read-only, or mixed visibility.
- Hosting target meeting durable SQLite storage and spending constraints.
- Battle timing, timeout rules, hidden information, and restart guarantees.
- Backup frequency/retention and acceptable data loss.
- Engine export schema, package publication workflow, and compatibility policy.

Resolve each when its implementation phase requires it; do not block the initial skeleton on every future battle-system detail.

## 22. Official technical references

Consult current documentation before implementing library-specific behavior:

- Axum: https://docs.rs/axum/latest/axum/
- Axum WebSockets: https://docs.rs/axum/latest/axum/extract/ws/
- SQLite WAL and concurrency: https://www.sqlite.org/wal.html
- SQLite online backup API: https://www.sqlite.org/backup.html
- SQLite corruption and backup pitfalls: https://www.sqlite.org/howtocorrupt.html
- mlua runtime and sandbox limitations: https://docs.rs/mlua/latest/mlua/
- mlua Lua configuration and resource controls: https://docs.rs/mlua/latest/mlua/struct.Lua.html

The sources above inform the relevant SQLite, Axum, and mlua constraints. The repository structure, roadmap, policies, and remaining stack selections are project recommendations, not claims made by those sources.
