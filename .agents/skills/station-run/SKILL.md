---
name: station-run
description: "Use when working an implementation station (a row in the backlog ledger's station table) — the per-station runbook covering pre-flight (anchors, dependency closure, granularity re-check), the implementation loop (commit discipline, test layering, D5/F-class obligations), and closeout (D11 test-provenance declaration, acceptance pack assembly, ledger update)."
---

# Station run (implementation station lifecycle)

A station is the acceptance unit of an implementation wave: one row in the
station table (站表) of `docs/backlog.md` (boundary, dependencies, anchors), accepted in one review
sitting per the granularity rule (`docs/design/operating-protocol.md` §2
任务粒度). This skill is procedure only — it points at the sources of truth
and never restates them: station content = the anchored design docs; test
mechanisms = `docs/design/testing-strategy.md`; process = operating-protocol.

One station at a time in the default 1+1 mode; parallel stations require a
concurrency window (operating-protocol §3 + appendix A). Closeout ends the
station — it never starts the next one: the maintainer paces station
starts, and a ✅ row is a state, not a launch signal.

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
   (a quality-infrastructure station is entirely this shape).
6. **Closeout-infrastructure check.** If a closeout obligation names a
   gate piece owned by another station (mutation gate, llvm-cov floor check), verify that
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
- **Slow-track legs** (testing-strategy §2.4) — `just deep`'s owner lives in
  the station table; when the station row assigns slow-track legs (mutants / Kani), run
  them and report outcomes in the acceptance pack. Slow-track failures never
  block commits: they are filed as issues carrying the failure fingerprint
  (test name + first-failing commit + repro command). The llvm-cov floor
  check lives in the fast track (§3.C; demoted if it gets slow).

## 3. Closeout (acceptance pack)

Assemble all of the following, then present as one batch (operating-protocol
§2 loop). Artifacts split by approval dependency: fact records (the D11
declaration) ship with the implementation commit; ratification-dependent
artifacts (the floor pin, the ledger ✅ row) ride this batch.

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
   (kernel crates declare composition at closeout); the location
   (exemplar-confirmed): a `Testing provenance (D11)` section in the crate's
   root rustdoc stating the composition (human-written/reviewed vs
   agent-written + mutation gate) and, for agent-written suites, the
   mutation-audit result (§3.C). The audit's house invocation is
   `cargo mutants -p <crate> --test-tool nextest`; its artifacts
   (`mutants.out`, `mutants.out.old`) are local-only and gitignored. The
   declaration cites the tool version, the caught/unviable/missed counts,
   and the repro command.
5. **F-class update** — §3.F crash-window list and matrix current for this
   station.
6. **Coverage floor** — kernel-list crates report the llvm-cov number
   (fast-track check, §3.C); the first kernel station measures and
   pins the floor value into the floor check (ratchet: only
   rises). Pin at 0.1 granularity at or below the measured value (the
   floor check's epsilon is built for that granularity) and name
   uncoverable-by-construction lines in the acceptance pack. The floor
   file is gate surface: the pin rides the acceptance batch (ratified
   value + `Gate-Surface:` trailer), never the implementation commit.
7. **Ledger update** — station row ✅ with SHAs; the task block status line (one
   of the ten fixed status words, done scope, next step, open items,
   review-surface three numbers); regenerate the panorama graph (completed
   stations leave it). Mid-station session ends still update the task block
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

Exemplar run: complete. Its teachings are folded into §3 above (D11
declaration shape and house invocation, floor-pin mechanics, the
approval-dependency split); the firsts are now ordinary closeout items.
