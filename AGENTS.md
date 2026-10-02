# AGENTS.md

<!-- Freshness: Commands tracks the justfile; NEVER tracks the generated-file list; Skills tracks .agents/skills/. Last reviewed: 2026-10 -->

## Commands (just is the only entry point; never bypass the quality gates)

- Full quality gate: `just ci` (run before every commit; remote CI returns with the GitHub migration)
- Format / static checks: `just fmt` / `just lint` (rustfmt + dprint for markdown)
- All tests: `just test`; single test: `cargo nextest run -p metis <name>`
- Iterate with the narrowest loop first (`cargo check -p <crate>`, scoped nextest); finish with `just ci`
- Snapshot updates: `just snapshot-review` (approve each diff by hand; never bulk-accept)
- New crate: `just new-crate <name>`
- Dependency policy: `just deny`; docs build: `just doc`; agent-doc smoke check: `just agent-check`
- Toolchain bump: `just toolchain-bump` (the wrapping weekly workflow returns with the GitHub migration)

## Environment

- Toolchains come from flake.nix: `direnv allow` or `nix develop`. `rust-toolchain.toml` is the version SSOT — no other file may restate toolchain versions.
- The agent terminal spawns a bare shell without the direnv hook, so `.envrc` never loads and `cargo`/`just` resolve to the user profile instead of flake.nix. Prefix such commands with `direnv exec .` when the flake toolchain is needed (e.g. `direnv exec . just ci`); this includes `git commit`, whose prek hooks run `just fmt` and `just lint`.
- `just deny` needs network access to fetch the RustSec advisory DB; when offline, skip that one item and run the rest.
- Native Windows development goes through WSL2 (Nix has no native Windows support).

## Skills (procedures live in .agents/skills/; constraints stay in this file)

- `self-review` — the pre-commit pass over your working diff: mechanical sweep yourself, judgment pass delegated to a fresh subagent (author blindness is structural)
- `adding-dependencies` — required procedure before touching any third-party dependency
- `rule-maintenance` — how to update AGENTS.md and skills when rules change or mistakes repeat
- `doc-maintenance` — writing/reviewing docs: three-question filter, language policy, drift hunting
- `code-simplification` — behavior-preserving dedup/dead-code/abstraction cleanup, always as its own change

Done = `just ci` green + self-review of the working diff.

## NEVER

- NEVER hand-edit generated files: `Cargo.lock` (the generated-files list grows back when release engineering returns).
- NEVER hand-write version numbers: the single storage point is `[workspace.package] version` in the root `Cargo.toml`.
- NEVER commit code that hasn't passed `just ci`; NEVER assemble ad-hoc check pipelines that bypass just.
- NEVER push tags or cut releases — release engineering is deferred (ADR-0020); version intent lives in commit messages only.
- NEVER add agent-attribution footers (`Co-Authored-By`, "Generated with …") to commits.
- NEVER force-push or rewrite shared history unless the human explicitly asks.
- NEVER commit `HANDOFF.md` — transient session-handoff artifact (gitignored).
- ASK before touching: new third-party dependencies (see the `adding-dependencies` skill), any CI workflow files, hosting/branch-protection settings.

## Style (the part lints can't enforce)

- Time / randomness / IO are always injected as constructor parameters; no hidden `now()` / `rand()` / global state (test determinism depends on this seam).
- When the maintainer asks what a piece of code means or why it exists, the explanation lands as a brief comment at that spot in the same change — chat answers rot. Comments still pass the SSOT bar: intent, constraints and tradeoffs, never a restatement of mechanism.
- Error split: library crates use `thiserror` enums (callers branch on failure modes); the binary's top level reports with `miette` (three-part `code` + `help`); functions that propagate errors don't log — log once at the handling site.
- Commit messages: modified Conventional Commits — `type(scope): subject`, lowercase type/scope, pure-ASCII English subject, header ≤ 100 chars, blank line before body/footer (`fix`→PATCH, `feat`→MINOR, `!`→MAJOR). Enforced at the commit-msg hook (SSOT: the `check-commit` subcommand of `crates/xtask`); no PR flow — commits push directly to `main`, so every message must be landable as-is.
- Language (ADR-0012): code, rustdoc, AGENTS.md, skills, README/CONTRIBUTING are English; `docs/` (decisions / design / research) is Chinese-canonical.
- Docs co-evolve in the same change as the behavior change. A rule you needed but couldn't find is a missing rule: add it via the `rule-maintenance` skill in that change, instead of re-deriving it next session.

## Documentation

- `docs/decisions/` — ADRs, one decision per file `NNNN-title.md`; numbers never reused; once accepted the body is frozen — overturning means a new ADR superseding (mark the old one `superseded by ADR-NNNN`, never delete). Partial supersede (one clause/cell, not the whole ADR) follows the same rule: the new ADR names the exact clause it replaces, the old ADR's Status line records the partial supersede, body stays frozen. Reference by number, never restate rationale.
- `docs/design/` — living design docs with a status header; confirmed decision points get promoted to ADRs.
- `docs/research/` — point-in-time reports; corrections land as addenda, never rewrite conclusions.
- `HANDOFF.md` (repo root, gitignored) — the transient session handoff: read it first when it exists; rewrite it when the session's state changes.
