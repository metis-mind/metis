# CONTRIBUTING

## Contributing status

Metis is open source, but the core repository is **not accepting code pull requests yet** — the architecture is still landing, and every change must pass an agent-oriented development protocol (see _Agent-assisted development_ below) that external PRs cannot practically satisfy. This policy is temporary and relaxes once the core stabilizes; it is not a one-way door.

- **Issues and Discussions are welcome**: bug reports, design questions, use-case proposals.
- **Ecosystem contributions** (Luau plugins) will flow through the `metis-mind/marketplace` repository once it opens — plugin submissions are machine-checked by schema CI.

## Environment setup

```bash
direnv allow   # or: nix develop — entering the devShell also arms the git hooks
just ci        # verify everything is green
```

Git hooks (pre-commit + commit-msg) arm themselves on devShell entry — git
cannot ship hooks with a clone, so in a non-Nix shell run `just setup` once
(`prek uninstall` to remove them).

All day-to-day commands: `just --list` (living documentation, evolves with the repo). Core entries: `just fmt` / `just lint` / `just test` / `just ci`.

## Commit messages

Modified Conventional Commits (SSOT: the `check-commit` subcommand of `crates/xtask`):

    type(scope)!: subject

- `type`: one of `feat fix docs style refactor perf test build ci chore revert` (lowercase)
- `scope`: optional, lowercase (crate name, `cli`, …); `!` marks breaking (or a `BREAKING CHANGE:` footer)
- `subject`: pure-ASCII English; the whole header is at most 100 chars
- body: any language, no line-width limit; separated from the header by one blank line. No HTML comments, no task lists (`- [ ]`), no bare `---` lines; a prose paragraph runs at most 7 lines — split longer bodies into paragraphs or use lists
- footer (`TOKEN: value` / `TOKEN #value`): the block is preceded by a blank line. Blessed token: `BREAKING CHANGE:`

Maintainer commits land by direct push to `main` with no internal PR flow ([ADR-0020](docs/decisions/0020-open-source-hosting.md)), so every message must be landable as-is — it IS the history. Enforcement is local: the commit-msg hook (armed on devShell entry; `just setup` in non-Nix shells). The pre-commit hook additionally runs a secret scan over the staged diff (`xtask pr-guard --staged`); documented example secrets can carry a `pr-guard:allow` marker on their line.

Semver mapping (consumed when release engineering returns): `fix` → PATCH, `feat` → MINOR, `!` → MAJOR.

## Adding a dependency

1. Declare the version in the root `Cargo.toml`'s `[workspace.dependencies]` (the only place in the repo);
2. Member crates inherit with `dep.workspace = true` and may only add `features` / `optional`;
3. `just deny` enforces license and source policy; a new license requires discussion before extending the `deny.toml` allow-list. The full judgment procedure lives in the `adding-dependencies` skill — ask the maintainer first.

## Adding a crate

```bash
just new-crate <name>
```

This creates an **internal crate** (`version = "0.0.0"`, `publish = false`) with no semver burden.

## Tests and snapshots

- Write tests at the lowest layer that can catch the bug; CLI behavior goes in `tests/` (assert_cmd), pure logic in unit tests.
- Large-output assertions (help text, diagnostics, serialized formats) use insta snapshots: snapshots are committed and reviewed; never bulk-accept. Nondeterministic fields (timestamps, paths, UUIDs) must be filtered/redacted before snapshotting.
- Update snapshots with `just snapshot-review` — read each diff before approving.

## Releasing and remote CI

Deferred. Release engineering and the remote periodic workflows return as a follow-up to the GitHub migration — see [ADR-0020](docs/decisions/0020-open-source-hosting.md). Until then every gate runs locally through `just`.

## Agent-assisted development

This repository is a dual human/agent artifact (1+N: one human, N agents — agents are the primary developers). The agent surface:

- `AGENTS.md` — always-on constraints (commands, NEVER rules, style). Incremental information only: every rule must pass "would an agent err without this?".
- `.agents/skills/<name>/SKILL.md` — task-activated procedures (progressive disclosure). Constraints live in AGENTS.md; procedures live in skills; reference knowledge lives in `docs/` (linked, never copied).
- `.claude/skills` symlinks to `.agents/skills` — one source of truth for every host.
- `just agent-check` (part of `just ci`) smoke-validates this surface: frontmatter shape, name/dir match, size budgets, pointer integrity.

Repeated agent mistakes become permanent rules via the `rule-maintenance` skill — in the same change as the fix, not as repeated chat corrections.

## Documentation discipline

Hand-written docs carry exactly three things: **intent** (why), **rationale** (ADRs), and **entry points** (runnable commands). Facts that code can state itself (parameter tables, version numbers, command lists) are never hand-copied into prose; architecture decisions go to `docs/decisions/` and everywhere else references the number only.

Language ([ADR-0012](docs/decisions/0012-docs-governance.md)): `docs/` (decisions / design / research) is Chinese-canonical — the maintainer reviews in Chinese. Code, rustdoc, AGENTS.md, `.agents/skills/`, README and CONTRIBUTING are English-only; no `*.zh-CN.md` translations are maintained. `HANDOFF.md` at the repo root is a transient, gitignored session-handoff artifact — never commit it.
