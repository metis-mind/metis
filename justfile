# The task layer: the single implementation site for every repeatable piece of
# automation in this repository. Local development, git hooks
# (.pre-commit-config.yaml) and CI (.github/workflows/) are all just callers.
# Rules: no version numbers here (reference the SSOT of the layer below); no
# bashisms or duplicated arguments in recipes — each tool's parameters live in
# that tool's own config file.

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

# Static checks: rustfmt --check, dprint check (markdown), clippy (-D warnings),
# actionlint on workflow YAML, cargo-machete on unused dependencies.
# No path argument for actionlint: it discovers .github/workflows/ itself, because
# the Windows shell (PowerShell) does not expand globs for native commands.
lint:
    cargo fmt --all -- --check
    dprint check
    cargo machete
    actionlint
    cargo clippy --workspace --all-targets -- -D warnings

# Tests: cargo-insta drives nextest (process isolation) + doctests (README/rustdoc
# examples are compiled too), then handles snapshot orphans — rejected in CI,
# deleted locally (D12). `--all-targets` is gone: cargo-insta forwards post-`--`
# options to the test binaries, not the runner; benches/examples re-add the
# equivalent when they appear.
test:
    cargo insta test --workspace --test-runner nextest --unreferenced=auto

# Build docs (rustdoc warnings are errors; RUSTDOCFLAGS injected at the top of this file)
doc:
    cargo doc --no-deps --workspace

# Dependency policy: advisories / licenses / duplicate versions / sources (see deny.toml).
# The three -D lints are the staleness fuses: an advisory ignore, per-crate
# license exception or duplicate skip whose target has left the tree fails CI
# instead of rotting.
deny:
    cargo deny check -D advisory-not-detected -D license-exception-not-encountered -D unmatched-skip

# Snapshot review: approve diffs one by one; CI is read-only and never auto-accepts
snapshot-review:
    cargo insta review

# Kernel-crate line-coverage floors (values: .config/cov-floor.toml — gate surface)
cov-floor:
    cargo run -q -p xtask -- cov-floor

# Test-bloat telemetry (D12, diagnostic only): product/test line ratio + test counts
test-telemetry:
    cargo run -q -p xtask -- test-telemetry

# Smoke check for agent-facing docs plus the structural docs/ integrity gates
# of the doc lifecycle (ADR-0033): doc map, internal links, freshness, archive
# consistency. Check semantics SSOT = the crates/xtask agent_docs module doc.
# Std-only Rust; runs on every platform `just ci` runs on.
agent-check:
    cargo run -q -p xtask -- agent-check

# Bump rust-toolchain.toml's channel to the latest stable release; requires
# curl. Wrapped by the weekly toolchain-update workflow (.github/workflows/).
toolchain-bump:
    cargo run -q -p xtask -- bump-toolchain

# Secret scan of a diff (added lines): token / private-key / .env shapes, hard
# fail. `pr-guard --staged` is the local pre-commit half; the CI caller is
# .github/workflows/pr-guard.yml.
pr-guard pr="":
    cargo run -q -p xtask -- pr-guard {{pr}}

# The local full quality gate ≡ CI (.github/workflows/ci.yml runs the same
# recipes); run before committing
ci: lint test doc deny agent-check cov-floor
    @echo "just ci: all green ✅"

# Slow track (testing-strategy §2.4): llvm-cov workspace report (HTML under
# target/llvm-cov/html + stdout summary — `report` reuses the run's merged
# profile: --summary-only cannot combine with --html) + test-bloat telemetry.
# Local on-demand; failures file issues, never block commits. The remote
# weekly run lands with the first cargo-mutants weekly leg.
deep: test-telemetry
    cargo llvm-cov --workspace --html
    cargo llvm-cov report --summary-only

# Kani proofs for the fiber kernel (testing-strategy §2.1, implementation-level
# rung of the L4 ladder; proofs live next to the code under cfg(kani)). Needs
# kani-verifier installed — see CONTRIBUTING "Formal verification (Kani)".
deep-kani:
    cargo kani -p metis-fiber-core

# Add an internal crate (unpublished by default; rules in CONTRIBUTING.md "Adding a crate")
new-crate name:
    cargo run -q -p xtask -- new-crate {{name}}
