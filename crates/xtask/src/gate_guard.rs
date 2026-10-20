//! Gate-surface guard (testing-strategy D9): a commit that touches
//! gate-surface files must carry a `Gate-Surface:` trailer naming every
//! touched gate path. The trailer is the human-confirmation marker for the
//! ASK rule (AGENTS.md NEVER); this commit-msg hook is its machine half.
//!
//! The gate list is hardcoded in [`GATE_PATHS`] — the prose SSOT is
//! docs/design/testing-strategy.md §4.2 and the two change in the same commit
//! (either side of that edit is itself gate surface). Two §4.2 exemption
//! surfaces stay outside this hook on purpose: insta snapshot bulk-acceptance
//! is already a process gate (two-stage `just snapshot-review`), and hosting
//! settings live outside git.
//!
//! Boundary honesty (doctrine 3): this is a local hook — `--no-verify`
//! bypasses it, and the trust anchor stays the main ruleset's audit trail
//! plus remote review. Invocations with an empty staged diff (`--amend`
//! without new staging, `--allow-empty`) pass trivially. A merge that brings
//! gate-surface changes is intercepted like any other commit — deliberate:
//! a merge is no confirmation either; the failed attempt leaves
//! `MERGE_HEAD` for the retry with a trailer.

use std::process::{Command, ExitCode};

/// Path-level gate surface. Prose SSOT: docs/design/testing-strategy.md §4.2
/// — changes land on both sides in the same commit. A trailing `/` marks a
/// directory prefix.
const GATE_PATHS: &[&str] = &[
    ".github/workflows/",
    ".github/dependabot.yml",
    "justfile",
    "rust-toolchain.toml",
    ".pre-commit-config.yaml",
    "deny.toml",
    "dprint.json",
    ".config/cov-floor.toml",
    ".cargo/mutants.toml",
    "crates/xtask/",
];

/// Corpus deletions are gate surface: `proptest-regressions/` is regression
/// inventory (D5), removing entries weakens the replay net.
const CORPUS_DIR: &str = "proptest-regressions/";
/// The mutation-gate exemption file is gate surface (D4) via `GATE_PATHS`
/// above. This content scan stays as a dormant fuse: `#[mutants::skip]`
/// cannot compile on the stable toolchain (it needs nightly
/// `register_tool`), so the live exemption list moved to
/// `.cargo/mutants.toml` (testing-strategy §4.2, 2026-10-20) — if the
/// toolchain ever goes nightly, this scan re-arms for free.
const MUTANTS_SKIP: &str = "#[mutants::skip";

/// The commit-message trailer carrying the human-confirmation file list.
const TRAILER: &str = "Gate-Surface:";

/// Entry point: `gate-guard [FILE]` (`-` or no argument reads stdin) —
/// the commit-msg hook hands over the message file path.
pub fn run(args: &[String]) -> ExitCode {
    if args.len() > 1 {
        eprintln!("usage: gate-guard [FILE]  ('-' or no arg = stdin)");
        return ExitCode::from(2);
    }
    let message = match args.first().map(String::as_str) {
        None | Some("-") => read_stdin(),
        Some(path) => match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(err) => {
                eprintln!("error: cannot read {path}: {err}");
                return ExitCode::from(2);
            }
        },
    };
    let name_status = match git(&["diff", "--cached", "--name-status"]) {
        Ok(out) => out,
        Err(code) => return code,
    };
    let diff = match git(&["diff", "--cached"]) {
        Ok(out) => out,
        Err(code) => return code,
    };
    let (changed, deleted) = parse_name_status(&name_status);
    let missing = uncovered(&message, &changed, &deleted, &diff);
    if missing.is_empty() {
        return ExitCode::SUCCESS;
    }
    eprintln!(
        "gate-guard: the commit touches gate-surface paths the message does not acknowledge:"
    );
    for path in &missing {
        eprintln!("  - {path}");
    }
    eprintln!();
    eprintln!("add a trailer naming every touched gate path (flush left), e.g.:");
    eprintln!();
    eprintln!("{TRAILER} {}", missing.join(", "));
    eprintln!();
    eprintln!("The trailer is the human-confirmation marker for the ASK rule (AGENTS.md");
    eprintln!("NEVER). Gate list SSOT: docs/design/testing-strategy.md §4.2.");
    ExitCode::FAILURE
}

fn read_stdin() -> String {
    use std::io::Read as _;
    let mut text = String::new();
    if let Err(err) = std::io::stdin().read_to_string(&mut text) {
        eprintln!("error: cannot read stdin: {err}");
        std::process::exit(2);
    }
    text
}

fn git(args: &[&str]) -> Result<String, ExitCode> {
    match Command::new("git").args(args).output() {
        Ok(output) if output.status.success() => {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        }
        Ok(output) => {
            eprintln!(
                "error: git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr)
            );
            Err(ExitCode::from(2))
        }
        Err(err) => {
            eprintln!("error: cannot run git: {err}");
            Err(ExitCode::from(2))
        }
    }
}

/// `git diff --cached --name-status` → (all touched paths, deleted paths).
/// A rename line (`R100\told\tnew`) counts under its new path — a rename
/// moving a gate file out of its listed path slips the hook; accepted
/// boundary (the threat model is forgetfulness, and the same rename breaks
/// CI loudly anyway).
fn parse_name_status(output: &str) -> (Vec<String>, Vec<String>) {
    let mut changed = Vec::new();
    let mut deleted = Vec::new();
    for line in output.lines() {
        let mut columns = line.split('\t');
        let Some(status) = columns.next() else {
            continue;
        };
        let Some(path) = columns.next_back() else {
            continue;
        };
        changed.push(path.to_string());
        if status.starts_with('D') {
            deleted.push(path.to_string());
        }
    }
    (changed, deleted)
}

/// Whether `path` is on the path-level gate list.
fn is_gate_path(path: &str) -> bool {
    GATE_PATHS.iter().any(|gate| match gate.strip_suffix('/') {
        Some(dir) => path == dir || path.starts_with(gate),
        None => path == *gate,
    })
}

/// Whether a trailer token covers a touched path: exact match, or a
/// directory token covering everything beneath it (`crates/xtask/` and
/// `crates/xtask` are equivalent).
fn covers(token: &str, path: &str) -> bool {
    let dir = token.strip_suffix('/').unwrap_or(token);
    !dir.is_empty()
        && (path == dir
            || path
                .strip_prefix(dir)
                .is_some_and(|rest| rest.starts_with('/')))
}

/// Paths whose staged diff adds a `#[mutants::skip]` line. Tracks the
/// `+++ b/<path>` header; removals never trip the scan (removing a skip
/// re-arms the gate). The attribute is anchored to line start (modulo
/// indentation) so prose mentions in comments/docs stay silent.
fn mutants_skip_paths(diff: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut current = String::new();
    for line in diff.lines() {
        if let Some(rest) = line.strip_prefix("+++ ") {
            current = rest.strip_prefix("b/").unwrap_or("").to_string();
        } else if let Some(added) = line.strip_prefix('+')
            && added.trim_start().starts_with(MUTANTS_SKIP)
            && !paths.contains(&current)
        {
            paths.push(current.clone());
        }
    }
    paths
}

/// All tokens carried by `Gate-Surface:` trailers in the message
/// (comma-separated; several trailers accumulate).
fn trailer_tokens(message: &str) -> Vec<String> {
    message
        .lines()
        .filter_map(|line| line.strip_prefix(TRAILER))
        .flat_map(|rest| rest.split(','))
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect()
}

/// The touched gate items the message does not acknowledge: gate-listed
/// paths, corpus deletions and added `#[mutants::skip]` lines, minus
/// everything a trailer token covers.
fn uncovered(message: &str, changed: &[String], deleted: &[String], diff: &str) -> Vec<String> {
    let mut items: Vec<String> = changed
        .iter()
        .filter(|path| is_gate_path(path))
        .cloned()
        .collect();
    items.extend(
        deleted
            .iter()
            .filter(|path| path.contains(CORPUS_DIR))
            .cloned(),
    );
    items.extend(mutants_skip_paths(diff));
    let tokens = trailer_tokens(message);
    items.retain(|item| !tokens.iter().any(|token| covers(token, item)));
    items.sort();
    items.dedup();
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn gate_paths_match_exact_files_and_dir_prefixes() {
        assert!(is_gate_path("justfile"));
        assert!(is_gate_path(".github/workflows/ci.yml"));
        assert!(is_gate_path("crates/xtask/src/main.rs"));
        assert!(!is_gate_path("justfile2"));
        assert!(!is_gate_path("crates/metis/src/main.rs"));
        assert!(!is_gate_path("docs/backlog.md"));
    }

    #[test]
    fn trailer_tokens_accumulate_and_trim() {
        assert!(trailer_tokens("feat: x\n\nbody").is_empty());
        let tokens = trailer_tokens("feat: x\n\nbody\n\nGate-Surface: justfile, crates/xtask/\n");
        assert_eq!(tokens, strings(&["justfile", "crates/xtask/"]));
        let two = trailer_tokens("Gate-Surface: justfile\n\nGate-Surface: deny.toml");
        assert_eq!(two, strings(&["justfile", "deny.toml"]));
        // exact, case-sensitive token — trailers are machine-checked
        assert!(trailer_tokens("gate-surface: justfile").is_empty());
    }

    #[test]
    fn coverage_is_exact_or_directory_descent() {
        assert!(covers("justfile", "justfile"));
        assert!(covers("crates/xtask/", "crates/xtask/src/main.rs"));
        assert!(covers("crates/xtask", "crates/xtask/src/main.rs"));
        assert!(!covers("justfile", "justfile2"));
        assert!(!covers("crates/xtask", "crates/xtask2/main.rs"));
        assert!(!covers("", "justfile"));
    }

    #[test]
    fn parse_name_status_splits_kinds_and_renames() {
        let (changed, deleted) =
            parse_name_status("M\tjustfile\nD\ta/proptest-regressions/seed.txt\nR100\told\tnew\n");
        assert_eq!(
            changed,
            strings(&["justfile", "a/proptest-regressions/seed.txt", "new"])
        );
        assert_eq!(deleted, strings(&["a/proptest-regressions/seed.txt"]));
    }

    #[test]
    fn mutants_skip_flags_added_lines_only() {
        let diff =
            "+++ b/src/lib.rs\n@@ -1 +1 @@\n+#[mutants::skip] reason\n-#[mutants::skip] old\n";
        assert_eq!(mutants_skip_paths(diff), strings(&["src/lib.rs"]));
        let removal_only = "+++ b/src/lib.rs\n@@ -1 +0 @@\n-#[mutants::skip] gone\n";
        assert!(mutants_skip_paths(removal_only).is_empty());
        // prose mentions in added comment/doc lines stay silent
        let doc_mention =
            "+++ b/src/lib.rs\n@@ -1 +1 @@\n+/// `#[mutants::skip]` is gate surface\n";
        assert!(mutants_skip_paths(doc_mention).is_empty());
    }

    #[test]
    fn uncovered_combines_path_and_content_triggers() {
        let message = "feat: x\n\nbody";
        // plain source change: no gate surface
        assert!(uncovered(message, &strings(&["crates/metis/src/main.rs"]), &[], "").is_empty());
        // gate file without trailer
        assert_eq!(
            uncovered(message, &strings(&["justfile"]), &[], ""),
            strings(&["justfile"])
        );
        // corpus deletion and mutants-skip addition are gate events too
        let diff = "+++ b/src/lib.rs\n@@ -1 +1 @@\n+#[mutants::skip] reason\n";
        assert_eq!(
            uncovered(
                message,
                &strings(&["src/lib.rs"]),
                &strings(&["x/proptest-regressions/a.txt"]),
                diff
            ),
            strings(&["src/lib.rs", "x/proptest-regressions/a.txt"])
        );
        // a covering trailer clears everything, directory tokens included
        let acknowledged =
            "feat: x\n\nbody\n\nGate-Surface: justfile, src/lib.rs, x/proptest-regressions/";
        assert!(
            uncovered(
                acknowledged,
                &strings(&["justfile", "src/lib.rs"]),
                &strings(&["x/proptest-regressions/a.txt"]),
                diff
            )
            .is_empty()
        );
    }
}
