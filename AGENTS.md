# AGENTS.md

<!-- Freshness: Commands tracks the justfile; NEVER tracks the generated-file list; Skills tracks .agents/skills/; Documentation tracks the docs/ file inventory (map/link/freshness/archive checks in `just agent-check`); Style and Reporting are unenforced discipline (commit-messages excepted — commit-msg hook). Last reviewed: 2026-10 -->

## Commands (just is the only entry point; never bypass the quality gates)

- Full quality gate: `just ci` (run before every commit; remote CI runs the same recipes on push/PR — `.github/workflows/ci.yml`)
- Format / static checks: `just fmt` / `just lint` (rustfmt + dprint for markdown + actionlint for workflow YAML)
- All tests: `just test`; single test: `cargo nextest run -p metis <name>`
- Iterate with the narrowest loop first (`cargo check -p <crate>`, scoped nextest); finish with `just ci`
- Snapshot updates: `just snapshot-review` (two-stage: state the expected change first, then approve each diff against it by hand; never bulk-accept — ADR-0031 doctrine 5)
- New crate: `just new-crate <name>`
- Dependency policy: `just deny`; docs build: `just doc`; agent-doc smoke check + docs/ integrity (map orphans/dangling, internal links, freshness, archive): `just agent-check`
- Toolchain bump: `just toolchain-bump` (wrapped by the weekly toolchain-update workflow; bot PRs don't trigger CI — the in-job `just ci` is the gate)

## Environment

- Toolchains come from flake.nix: `direnv allow` or `nix develop`. `rust-toolchain.toml` is the version SSOT — no other file may restate toolchain versions.
- The agent terminal spawns a bare shell without the direnv hook, so `.envrc` never loads and `cargo`/`just` resolve to the user profile instead of flake.nix. Prefix such commands with `direnv exec .` when the flake toolchain is needed (e.g. `direnv exec . just ci`); this includes `git commit`, whose prek hooks run `just fmt` and `just lint`.
- `just deny` needs network access to fetch the RustSec advisory DB; when offline, skip that one item and run the rest.
- Native Windows development goes through WSL2 (Nix has no native Windows support).

## Skills (procedures live in .agents/skills/; constraints stay in this file)

- `self-review` — the pre-commit pass over your working diff: mechanical sweep yourself, judgment pass delegated to a fresh subagent (author blindness is structural)
- `adding-dependencies` — required procedure before touching any third-party dependency
- `rule-maintenance` — how to update AGENTS.md and skills when rules change or mistakes repeat
- `doc-maintenance` — writing/reviewing docs: three-question filter, language policy, drift hunting, lifecycle patrol
- `code-simplification` — behavior-preserving dedup/dead-code/abstraction cleanup, always as its own change
- `chinese-doc-style` — docs/ Chinese prose style: fidelity-first rewriting, task modes, 「」/spacing/one-paragraph-per-line typography
- `explain-diff` — maintainer-facing rich HTML walkthrough of a change + in-chat quiz, for reading large diffs
- `station-run` — implementation station lifecycle: pre-flight (anchors / dependencies / granularity re-check), test layering obligations, closeout acceptance pack (T10.x stations; exemplar = T10.1)

Done = `just ci` green + self-review of the working diff.

## NEVER

- NEVER hand-edit generated files: `Cargo.lock` (the generated-files list grows back when release engineering returns).
- NEVER hand-write version numbers: the single storage point is `[workspace.package] version` in the root `Cargo.toml`.
- NEVER commit code that hasn't passed `just ci`; NEVER assemble ad-hoc check pipelines that bypass just.
- NEVER push tags or cut releases — release engineering is deferred (ADR-0020, form decided in ADR-0030); version intent lives in commit messages only.
- NEVER add agent-attribution footers (`Co-Authored-By`, "Generated with …") to commits.
- NEVER force-push or rewrite shared history unless the human explicitly asks.
- ASK before touching: new third-party dependencies (see the `adding-dependencies` skill), any file on the gate-surface list (`docs/design/testing-strategy.md` §4.2 — CI workflows, justfile, hook configs, `crates/xtask`, deny/dprint config, test-exemption surfaces), hosting/branch-protection settings.

## Style (the part lints can't enforce)

- Time / randomness / IO are always injected as constructor parameters; no hidden `now()` / `rand()` / global state (test determinism depends on this seam).
- When the maintainer asks what a piece of code means or why it exists, the explanation lands as a brief comment at that spot in the same change — chat answers rot. Comments still pass the SSOT bar: intent, constraints and tradeoffs, never a restatement of mechanism.
- Error split: library crates use `thiserror` enums (callers branch on failure modes); the binary's top level reports with `miette` (three-part `code` + `help`); functions that propagate errors don't log — log once at the handling site.
- Commit messages: modified Conventional Commits — `type(scope): subject`, lowercase type/scope, pure-ASCII English subject, header ≤ 100 chars, blank line before body/footer (`fix`→PATCH, `feat`→MINOR, `!`→MAJOR). Enforced at the commit-msg hook (SSOT: the `check-commit` subcommand of `crates/xtask`); no PR flow — commits push directly to `main`, so every message must be landable as-is.
- Language (ADR-0012): code, rustdoc, AGENTS.md, skills, README/CONTRIBUTING are English; `docs/` (decisions / design / research) is Chinese-canonical.
- Docs co-evolve in the same change as the behavior change. A rule you needed but couldn't find is a missing rule: add it via the `rule-maintenance` skill in that change, instead of re-deriving it next session.
- Spark capture: a dense forward-looking passage from the maintainer (meta-observation / structural analogy / future requirement / self-noted forgetting risk), or an explicit 记录/拎起来, must be captured verbatim-first into the spark register of `docs/backlog.md` — key sentences quoted, context anchored, dated, consumption point named. Quotes stay verbatim while pending (compression is decay); consumption solidifies the content into the relevant committed doc or code and the register entry is then deleted outright — the destination is the SSOT, and the ledger's own git history preserves everything else; unconsumed sparks never vanish silently (unenforced — the feedback loop is the maintainer reporting a miss).

## Reporting (agent → maintainer replies; ste-zh core, unenforced style discipline)

- Fixed status words only: 已完成 / 部分完成 / 未开始 / 进行中 / 已验证 / 未验证 / 失败 / 跳过 / 阻塞 / 未确认 — and every status names its scope (已完成 of what, 已验证 over what). Work status ≠ verification status — 已完成 never implies 已验证.
- Verification claims carry four parts: status, method, result, scope. A check not run is 未验证; a claim without evidence is 未确认 plus how to confirm. Reading a source never counts as running verification.
- Evidence is locatable: `path:line`, commit SHA, full command.
- Decision points go to numbered options, the recommended one first, marked 「（推荐）」.
- Verification claims prefer agent-unforgeable anchors (CI run URL, commit SHA) over self-reported results.
- Every item presented for ratification must pass the actionable admission test: it asks the maintainer for a decision or an action; pure announcements demote to `docs/backlog.md` and never occupy a ratification batch.
- Uncertainty is never papered over with 「可能 / 大概 / 应该是」 — mark 「未确认」 and name the confirmation method. (Core absorbed from github.com/dualface/ste-zh, MIT.)

## Documentation

- `docs/decisions/` — ADRs, one decision per file `NNNN-title.md`; numbers never reused; the four-digit number sequence is the index (individual ADRs are not listed here). Once accepted the body is frozen — overturning means a new ADR superseding (mark the old one `superseded by ADR-NNNN`, never delete). Partial supersede (one clause/cell, not the whole ADR) follows the same rule: the new ADR names the exact clause it replaces, the old ADR's Status line records the partial supersede, body stays frozen. Reference by number, never restate rationale.
- `docs/backlog.md` — the run-state ledger (Chinese, agent-maintained): task queue + open-issue register + spark register + todo panorama graph. Session start reads it right after this file; session close updates the task-block status line in the same commit (protocol: `docs/design/operating-protocol.md` §2).
- `docs/design/` — living design docs with a status header; confirmed decision points get promoted to ADRs; `archived` entries are retired from default reading (lifecycle doctrine: `docs/design/doc-lifecycle.md`):
  - `docs/design/fiber.md` — fiber runtime model (frozen, ADR-0010/0019/0023)
  - `docs/design/context-events.md` — context surface and event system (frozen, ADR-0011/0013–0017/0022)
  - `docs/design/service-registry.md` — service registry and inject (frozen, ADR-0018)
  - `docs/design/seam-capabilities.md` — seam capability set (frozen, ADR-0025)
  - `docs/design/luau-abi.md` — Luau ABI (frozen, ADR-0026)
  - `docs/design/config-format.md` — config format (frozen, ADR-0027)
  - `docs/design/instance-model.md` — instance model (frozen, ADR-0028)
  - `docs/design/journal.md` — journal: core operation/audit log (frozen, ADR-0029)
  - `docs/design/testing-strategy.md` — testing and quality strategy (frozen, ADR-0031)
  - `docs/design/operating-protocol.md` — 1+N operating protocol (frozen, ADR-0032)
  - `docs/design/doc-lifecycle.md` — doc lifecycle and anti-bloat (frozen, ADR-0033)
  - `docs/design/session-model.md` — session model (archived 2026-10-14: absorbed into instance-model; skip unless archaeology)
- `docs/research/` — point-in-time reports; corrections land as addenda, never rewrite conclusions:
  - `docs/research/cordis-research.md` — Cordis/Harness survey (project-direction input)
  - `docs/research/journal-replay-research.md` — journal/replay research (ADR-0029 input)
  - `docs/research/wasm-research.md` — wasm carrier research with addenda 1–4 (ADR-0024 spike evidence)
  - `docs/research/luau-toolchain-spike.md` — Luau toolchain spike results
  - `docs/research/review-economics-research.md` — review-economics report RQ1–RQ8 (ADR-0031/0032 input)
  - `docs/research/review-economics-admission-check.md` — independent source spot-check of the report (19/19 pass)
  - `docs/research/review-economics-cross-verification.md` — the research team's self-verification archive
  - `docs/research/review-economics-rq6-inventory.md` — RQ6 in-repo mechanism inventory
  - `docs/research/testing-practice-oracle.md` — testing-practice supplement: oracles
  - `docs/research/testing-practice-formal-verification.md` — testing-practice supplement: formal methods and DST
  - `docs/research/testing-practice-parallel-engineering.md` — testing-practice supplement: parallel testing engineering
  - `docs/research/handoff-era-archive.md` — verbatim archive of the final HANDOFF.md (retired session-handoff regime, ADR-0032)
