# 0013 — Authoring validation policy 2 and archived selection boundaries

## Context and alternatives

The maintainer approved fixes for four reproduced ADR0012 review findings:
impossible required numeric intersections were not diagnosed; optional maximum/
repeat policies were called incomplete; new presets could select archived Moves;
and retained archived chart pins prevented unrelated ruleset edits.

Rewriting validator policy 1 in place would reinterpret saved historical reports.
Rejecting newly recognized contradictory rulesets during reads would also make
previously valid diagnostic drafts unreadable. Instead preserve policy 1 and add
an explicitly identified policy 2. This needs no schema or snapshot rewrite.

## Decision

- Policy 1 retains its original findings and deterministic input/message ordering.
  Unknown future policies remain explicitly unsupported, never silently evaluated.
- New presets use policy 2. Ordinary edits retain a preset's saved policy. Editors
  may explicitly request current-policy revalidation during the existing preview/
  confirmation workflow; it creates a new attributed revision when policy changes.
  Expected-revision, permissions and proposal fingerprint checks still apply.
- Ad-hoc Move reports use current policy 2 and identify it. Policy 2 diagnoses an
  impossible Required Known expectation from the intersection of intrinsic and
  additional inclusive numeric bounds and their combined whole-number constraint,
  even if the Move value is Unknown/absent or a preset selects no acquisitions.
  Use checked/exact integer arithmetic, including signed boundary rounding for
  integral feasibility. Optional absent attributes need not have a numeric value.
- Structurally well-formed contradictory configurations remain saveable drafts;
  contradictions belong in reports, not corruption errors on historical reads.
- Under policy 2, no configured maximum means no size restriction, not incomplete
  data. Unspecified repeat policy belongs to checks not evaluated. These choices
  never imply combat readiness or an executable effect/eligibility implementation.
- Existing fixture creation snapshots retain policy 1 and exact bytes. Trusted
  setup recognizes supported historical policies rather than requiring the latest
  policy; reruns never upgrade fixture heads or histories.
- Preset writes check newly selected acquisitions against selectable unarchived
  Move versions while holding the write transaction. Retention is counted by exact
  acquisition identity and Move pin, including multiplicity, so reorder/removal/
  unrelated edits retain existing archived selections but cannot add extra copies
  or retarget to archived Moves. Historical reads remain archive-independent.
- Ruleset preview and save distinguish an unchanged exact chart pin from a new
  chart selection. Retained pins use checked historical readers; additions and
  retargets still require unarchived head and selected versions.

## Consequences and checks

Historical diagnostics remain reproducible, and old drafts remain editable. An
explicit revalidation can change diagnostic categories without changing content
dependencies. No migration, dependency, account, fixture-baseline or live-data
change is needed. Record the policy in UI reports, previews and history.

Regression coverage includes disjoint/negative/fractional/integral/boundary ranges,
empty presets, optional policies, unchanged policy-1 reports/fixtures, explicit
revalidation and stale/rollback behavior, archived new/retained/duplicate selections,
and retained/new chart pins in both browser adapters and use cases. Run formatting,
Clippy with warnings denied, workspace and relevant browser workflows. Test servers
are temporary and stop after verification.

The dedicated Plan agent is unavailable. The preceding read-only review and its
four temporary reproductions supplied the assessment; bounded implementation uses
the approved contracts recorded here.
