---
name: station-run
description: "Use when working an implementation station (T10.x and later) — the per-station runbook covering pre-flight (anchors, dependency closure, granularity re-check), the implementation loop (commit discipline, test layering, D5/F-class obligations), and closeout (D11 test-provenance declaration, acceptance pack assembly, ledger update). Exemplar run = T10.1."
---

# Station run (implementation station lifecycle)

A station is the acceptance unit of T10: one row in the T10 block of
`docs/backlog.md` (boundary, dependencies, anchors), accepted in one review
sitting per the granularity rule (`docs/design/operating-protocol.md` §2
任务粒度). This skill is procedure only — it points at the sources of truth
and never restates them: station content = the anchored design docs; test
mechanisms = `docs/design/testing-strategy.md`; process = operating-protocol.

One station at a time in the default 1+1 mode; parallel stations require a
concurrency window (operating-protocol §3 + appendix A).

## 1. Pre-flight (before writing code)

1. **Read the station row.** Boundary (crate / file set), dependency line,
   anchors. Confirm dependency closure on the panorama graph: every edge
   **into** this station must originate at a ✅ station (equivalently: every
   station on the row's dependency line is ✅). If a dependency is unmet,
   stop and report — do not start.
2. **Read the anchored registration surfaces** in the design docs and extract
   this station's concrete checklist. A dangling anchor or a registered item
   contradicting the frozen design is an issue to surface, never something to
   silently reinterpret.
3. **Granularity re-check.** Walk the checklist and estimate the review
   surface. If the station no longer fits one acceptance sitting, the remedy
   is a split proposed as an issue batch (任务粒度 rule) — never dilute the
   acceptance instead.
4. **Plan the D11 provenance split** for the crate (human-written/reviewed
   tests vs agent-written + mutation gate; testing-strategy §2.2). It shapes
   test work from day one; the final declaration lands at closeout.
5. **Gate-surface check.** Touching any file in testing-strategy §4.2 = HARD
   human-review gate: the explicit approval must exist before the change
   (T10.0a/b are entirely this shape).
6. **Closeout-infrastructure check.** If a closeout obligation names a
   T10.0a/b-owned gate piece (mutation gate, llvm-cov floor check), verify that
   piece exists before starting; if it does not, stop and report — closeout
   is the most expensive place to discover it.

## 2. Implementation loop

- **Commit discipline** — sizes per self-review skill §4; one logical change
  per commit; `just ci` green plus a self-review pass (author sweep +
  delegated judgment) before every commit.
- **Test placement by layer** (testing-strategy §2): L0 unit/snapshot for
  everything; L1 proptest for pure cores; L2 model-based where a reference
  model exists; L3 corpus where fuzzing exists for the surface; L4/Kani only
  where the station row assigns it.
- **D5 replay discipline** — failure logs carry seeds; never "rerun and
  hope"; journal-entry sequences go into insta snapshots; snapshots prove
  "behavior unchanged", never "behavior correct".
- **Determinism seam** — time / randomness / IO are constructor parameters
  (AGENTS.md Style); tests are built on this seam, not around it.
- **F-class** (testing-strategy §3.F) — grow the station's crash-window list
  as you implement; every window gets a fault-point × recovery-promise row.
- **实现期配置 values** — fill each with the mechanism station it belongs to;
  record chosen values where the registering doc expects them.
- **Slow-track legs** (testing-strategy §2.4) — `just deep` is created by
  T10.0b; when the station row assigns slow-track legs (mutants / Kani), run
  them and report outcomes in the acceptance pack. Slow-track failures never
  block commits: they are filed as issues carrying the failure fingerprint
  (test name + first-failing commit + repro command). The llvm-cov floor
  check lives in the fast track (§3.C; demoted if it gets slow).

## 3. Closeout (acceptance pack)

Assemble all of the following, then present as one batch (operating-protocol
§2 loop):

1. **Decision batch** — every decision the station surfaced, actionable
   format (numbered, recommendation first). What the maintainer did not
   object to is not restated.
2. **Summary** — what was built, where the behavior lives, what was
   deliberately not done (extension slots). Offer `explain-diff` when the
   diff is large.
3. **Audit surface** — full commit list with SHAs + CI run URLs (unforgeable
   anchors). The maintainer samples ≥1 commit for full-diff review (non-zero
   floor) and hand-implements or debugs ≥1 snippet (calibration quota).
   Provide the frame; **never pre-select the sample** — the reviewed party
   does not own the sampling frame (doctrine 3).
4. **D11 declaration** — the requirement is testing-strategy §2.2's
   (kernel crates declare composition at closeout); the location is this
   skill's convention, to be confirmed or amended by the T10.1 exemplar: a
   `Testing provenance (D11)` section in the crate's root rustdoc stating
   the composition (human-written/reviewed vs agent-written + mutation
   gate) and, for agent-written suites, the mutation-gate result (§3.C).
5. **F-class update** — §3.F crash-window list and matrix current for this
   station.
6. **Coverage floor** — kernel-list crates report the llvm-cov number
   (fast-track check, §3.C); the first kernel station (T10.1) measures and
   pins the floor value into the check built by T10.0a (ratchet: only
   rises).
7. **Ledger update** — station row ✅ with SHAs; T10 block status line (one
   of the ten fixed status words, done scope, next step, open items,
   review-surface three numbers); regenerate the panorama graph (completed
   stations leave it). Mid-station session ends still update the T10 block
   status line (operating-protocol §2 会话收尾序列).

## Rationalizations that fail

| Rationalization                                   | Reality                                                                           |
| ------------------------------------------------- | --------------------------------------------------------------------------------- |
| "`just ci` green = station done"                  | Green is the machine floor; the pack above is the done state.                     |
| "This checklist item is minor — skip it"          | Registered items are frozen design; skipping needs a decision point, not silence. |
| "Too small to self-review"                        | The discipline is per-commit, no floor.                                           |
| Pre-selecting the audit sample for the maintainer | That is the reviewed party choosing the sampling frame — doctrine 3 violation.    |
| "Snapshots show it works"                         | Snapshots show behavior didn't change; correctness is the tests' job.             |

---

Exemplar run = T10.1 (metis-value): the zero-dependency first station,
carrying the firsts (floor pinning, first D11 declaration, first F-class
list). What the exemplar teaches lands back in this skill in the same change.
