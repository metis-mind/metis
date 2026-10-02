# The task layer: the single implementation site for every repeatable piece of
# automation in this repository. Local development and git hooks
# (.pre-commit-config.yaml) are just callers; CI joins when workflows return
# (GitHub migration, ADR-0020). Rules: no version numbers here (reference the
# SSOT of the layer below); no bashisms or duplicated arguments in recipes —
# each tool's parameters live in that tool's own config file.

set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

export RUSTDOCFLAGS := "-D warnings"

# List all commands (just --list is living documentation)
default:
    @just --list

# Manual hook install for non-Nix shells — inside the devShell this happens
# automatically on entry (git cannot ship hooks with a clone)
setup:
    prek install --hook-type pre-commit --hook-type commit-msg

# In CI the pre-commit framework fails the job if files were modified by this
# (see the quality-gate job), so the check-mode twin is only exercised there.

# Format Rust (rustfmt) and markdown (dprint; config: dprint.json) — write mode
fmt:
    cargo fmt --all
    dprint fmt

# Static checks: rustfmt --check, dprint check (markdown), clippy (-D warnings)
lint:
    cargo fmt --all -- --check
    dprint check
    cargo clippy --workspace --all-targets -- -D warnings

# Tests: nextest (process isolation) + doctests (README/rustdoc examples are compiled too)
test:
    cargo nextest run --workspace --all-targets
    cargo test --doc --workspace

# Build docs (rustdoc warnings are errors; RUSTDOCFLAGS injected at the top of this file)
doc:
    cargo doc --no-deps --workspace

# Dependency policy: advisories / licenses / duplicate versions / sources (see deny.toml).
# The two -D lints are the staleness fuses: an advisory ignore or per-crate
# license exception whose crate has left the tree fails CI instead of rotting.
deny:
    cargo deny check -D advisory-not-detected -D license-exception-not-encountered

# Snapshot review: approve diffs one by one; CI is read-only and never auto-accepts
snapshot-review:
    cargo insta review

# Smoke check for agent-facing docs: SKILL.md frontmatter, size budgets, and
# pointer integrity (AGENTS.md / CLAUDE.md / .claude/skills). Std-only Rust in
# crates/xtask; runs on every platform `just ci` runs on.
agent-check:
    cargo run -q -p xtask -- agent-check

# Bump rust-toolchain.toml's channel to the latest stable release; requires
# curl. The wrapping weekly workflow returns with the GitHub migration.
toolchain-bump:
    cargo run -q -p xtask -- bump-toolchain

# Secret scan of a diff (added lines): token / private-key / .env shapes, hard
# fail. `pr-guard --staged` is the local pre-commit half; a CI caller returns
# with the GitHub migration.
pr-guard pr="":
    cargo run -q -p xtask -- pr-guard {{pr}}

# The local full quality gate ≡ CI (modulo matrix dimensions); run before committing
ci: lint test doc deny agent-check
    @echo "just ci: all green ✅"

# Add an internal crate (unpublished by default; rules in CONTRIBUTING.md "Adding a crate")
new-crate name:
    cargo run -q -p xtask -- new-crate {{name}}
