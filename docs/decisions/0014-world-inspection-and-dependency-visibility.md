# 0014 — Consistent world inspection and scoped dependency visibility

## Context and alternatives

The maintainer approved one foundation-focused milestone after Moves/learning:
world-wide authoring inspection and world-scoped where-used views. Inspect all
current world content, including unused/archived local objects, with the current
public content visibility. Export follows separately; this is neither a complete
world package nor an executable ruleset or combat/export readiness check.

A world revision does not capture independent charts, rosters, rulesets, learnsets
or presets. Composing public services that open separate transactions could mix
different moments. Identity-only deduplication would lose exact versions and
selection multiplicity. Use one bounded read snapshot and concrete typed records.

## Decision

### Inventory and exact closure

- Resolve the world as the first read inside one ordinary SQLite read transaction.
  Enumerate all local Types, Stats, move attributes, Moves, variants, singleton
  chart/Stat-roster/ruleset, world learnsets and variant presets on that connection.
  Explicitly record absent singletons/learnsets and empty owner preset inventories.
- World root edges are observations of current membership/ownership, not a claim
  that the world revision historically pins every child revision. Current roots
  are classified active/archived separately; older exact versions are dependencies.
- Expand exact sources, assignments, roster/chart contexts, mappings, acquisitions
  and preset contexts. Shared content enters only through explicit dependencies;
  a source form does not implicitly contribute its current independent learnset.
  Preserve every acquisition and preset slot as a distinct edge/site. Selection
  edges identify equipped acquisitions separately from the full learnset closure.
- Keys include kind, scope, immutable identity and revision. Identity-only owner
  backlinks are distinguished from exact pins and do not recursively expand into
  unrelated current content. Where-used counts do not treat catalog membership or
  ownership as meaningful content use.
- Use existing checked persistence readers, with narrow adapters for older
  Species/Type/world/creature readers to verify public snapshots, projections,
  exact parents and current-head agreement. Current membership must agree with
  the current supported world snapshot; old v1 snapshots retain their meaning.
  Historical metadata is never compared with a renamed current head's metadata.
  Verification companions check their own typed metadata and required header,
  including the immediate predecessor; this is not an all-history integrity scan.
- Capture public DTOs explicitly. Never serialize WorldRecord (owner/access data),
  users, sessions, grants, raw audit/history or raw snapshot JSON into the report,
  fingerprint or logs. Internal integrity checks may inspect then discard audit.
- Materialize all input and end the read transaction before pure analysis,
  fingerprinting, path traversal or rendering. Inspection writes no revision,
  audit, account, cached report or schema data.

### Bounded work and honest failures

- Server-owned initial inspection budgets: 2048 current roots, 8192 exact nodes,
  65536 edge occurrences, 1 MiB per preflighted snapshot, 16 MiB accounted snapshot
  bytes, and a 10-second capture deadline. These are operational inspection limits,
  not authoring/game cardinality rules and cannot be raised by query parameters.
- Bound inventory queries before materialization and preflight known snapshots/
  child counts (including known current/predecessor companions) before expensive
  verification. Cache graph read success/failure by exact key for this request.
  Existing readers may repeat nested source/head verification: the outer cache is
  not a perfect SQL/CPU/memory meter. Check deadlines around verified async reads;
  never bypass validation to finish within budget.
- A safe SQLite progress callback checks the deadline about every 1000 VM
  instructions and interrupts in-flight VM work. Normal exits remove it and roll
  back; service connections close on drop so cancellation cannot return a poisoned
  deadline handler to authoring. This is cooperative, not hard real-time: SQLite's
  five-second busy waits, filesystem stalls and non-VM work can delay cleanup.
- An unusable world anchor is unavailable. A failed child or exhausted budget
  yields explicit partial coverage with identified gaps, not Unknown game data or
  a successful full inspection. Partial captures have no complete fingerprint.
- For complete captures, existing SHA-256 hashes a versioned deterministic vector
  of sanitized public data, roots, absences, exact nodes, ordered edge sites and
  head observations used for diagnostics. Exclude time, URLs, filters, pagination,
  access/account/audit data and rendered messages. It is an inspection fingerprint,
  not a package manifest, database checksum or tamper-proof claim.
- At most two public inspection jobs run per service, including blocking analysis/
  fingerprint/path work; admission remains held until their result is released.
  CPU work runs off async handlers. Validator leaves share immutable exact evidence
  rather than deep-copying a complete input vector per message. Each rendered
  finding shows at most 25 evidence nodes with an explicit remaining count; the
  full typed evidence and captured inventories remain available to analysis.

### Findings and usage

- Findings have typed categories/codes, exact subjects/evidence, edge locations and
  readable explanations: contradictions, incomplete inputs, context differences
  and checks not evaluated. Capture gaps remain separate from domain findings.
- Reuse policy-2 configuration/Move diagnostics. Saved presets retain their saved
  validator policy and exact context; an optional clearly labeled current-policy
  comparison uses the same exact inputs and writes nothing. Do not parse message
  strings to infer relationships or silently adopt/revalidate anything.
- Missing/zero, Unknown/NotApplicable, unmodeled/defined-empty and chart Unknown/
  outside-roster remain distinct. Chart compatibility uses scoped canonical Type
  identity across metadata versions. An older pin or retained archived dependency
  is informational, not automatically invalid. Optional unconfigured contexts are
  not invented combat requirements. Formula/effect/eligibility checks stay inert.
- Where-used examines this captured world's current consumers and retained exact
  dependencies, with all captured versions or one exact revision. It is not global
  or all-history search. Show direct sites and bounded canonical shortest root
  paths with relationship labels. Path work/results/depth are bounded, with
  explicit truncation; never enumerate every combinatorial path.
- "Unused" means no observed incoming content use in this captured world, never
  globally unused/safe to delete. Partial results say no uses found in the inspected
  portion rather than unused. Full learnset context and actual preset selection
  are visibly distinct.

### Native UI and freshness

- Public GET/HEAD pages at /worlds/{world}/inspection and its /where-used suffix
  follow current readable scoped addressing, raw query preservation and ID links.
  There are no mutation endpoints or implicit repair operations.
- Native filters and 25-entry pagination show captured revisions, coverage,
  fingerprint, archived roots, gaps and exact-history/where-used links. Query kind,
  scope, version, view/category and page are allowlisted/typed. Links/labels come
  from captured DTOs or canonical IDs, never fresh post-transaction data reads.
- Reports are transient: each request recaptures. Navigation carries the prior
  complete fingerprint; changed input gets a prominent refreshed-result notice,
  never a mixed-page snapshot claim. Explicit Refresh starts a new capture.
  Filters do not mutate content or confer export permissions.

## Checks and consequences

Test independently expected inventory/edges, unused/archived roots, exact closure
without unrelated shared content, repeated acquisition/selection sites, stable
private-free fingerprints, missing/unsupported/inconsistent snapshots, each budget,
historical renames, policy-1 preservation, Type identity compatibility, chart/roster
context distinctions, bounded direct/root usage, and no writes on success/failure.
A synchronized two-connection WAL race must prove one capture is wholly before a
competing edit and a subsequent capture wholly after it, including independent
singleton/learning changes that do not advance world revision. Browser tests cover
keyboard navigation, filtering, freshness, escaping and 320px reflow.

No migration, dependency, fixture object or export schema is required. Existing
synthetic labs supply diagnostics. General archive/revert, cross-world comparison,
ownership recovery, package publication and mechanics remain pinned follow-ups.
The dedicated Plan agent is unavailable; a read-only architecture assessment and
maintainer approval preceded this record and bounded module implementation.
