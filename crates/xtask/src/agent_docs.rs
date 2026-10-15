//! Smoke check for agent-facing docs ("can the host even load it?") plus the
//! structural anti-bloat gates of the doc lifecycle (ADR-0033; doctrine:
//! docs/design/doc-lifecycle.md):
//!
//! - every `.agents/skills/<name>/SKILL.md` has frontmatter whose `name` equals
//!   the directory name, whose `description` is 1..=1024 chars, whose keys stay
//!   within [`ALLOWED_KEYS`], and whose body fits the progressive-disclosure budget;
//! - `.claude/skills` points at `.agents/skills` (single source of truth —
//!   tolerates git's text-file fallback on platforms without symlink support);
//! - `AGENTS.md` exists, stays within the always-on line budget, and carries
//!   its freshness note;
//! - the AGENTS.md doc map has no orphans (every `docs/design/*.md` and
//!   `docs/research/*.md` file plus `docs/backlog.md` appears in it —
//!   docs/decisions is exempt: the number sequence is the index) and no reverse
//!   danglers (every `docs/...` path the map mentions exists);
//! - internal `.md` link targets in `docs/**` and `AGENTS.md` resolve (file
//!   part only; anchors stay unchecked) — a file move that breaks an inbound
//!   link fails here, which is the mechanical backbone of the in-place archive
//!   model (frozen ADR bodies can never be re-pointed);
//! - every active `docs/design/*.md` carries `Last reviewed: YYYY-MM` in its
//!   header; archived docs (`📦 已归档`) are exempt, and the archive marker and
//!   the map entry's `archived` annotation must agree in both directions;
//! - `CLAUDE.md` is only a reference to AGENTS.md, never a second copy.
//!
//! All checks are structural. Whether a date was honestly bumped or a summary
//! stayed faithful is a semantic judgment, and the machine makes no such
//! promise — that residue belongs to the gardener patrol and human spot-checks.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// The repository root is baked in at compile time, so the check works from
/// any working directory (hooks, CI jobs and the bootstrap app alike).
const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

// ~5k tokens of body text per skill: level-2 progressive-disclosure budget
const SKILL_BODY_BUDGET_CHARS: usize = 20_000;
const AGENTS_LINE_BUDGET: usize = 250;
// The SKILL.md standard guarantees only `name` and `description` are loaded at
// level 1; anything else would be metadata no host reads (see ADR-0005).
const ALLOWED_KEYS: [&str; 5] = [
    "name",
    "description",
    "license",
    "allowed-tools",
    "metadata",
];

/// Entry point: `agent-check` (no arguments).
pub fn run(args: &[String]) -> ExitCode {
    if !args.is_empty() {
        eprintln!("usage: agent-check  (takes no arguments)");
        return ExitCode::from(2);
    }
    let root = Path::new(ROOT);
    let mut failures: Vec<String> = Vec::new();

    let mut skill_files: Vec<PathBuf> = Vec::new();
    let skills_dir = root.join(".agents/skills");
    if let Ok(entries) = std::fs::read_dir(&skills_dir) {
        for entry in entries.flatten() {
            let candidate = entry.path().join("SKILL.md");
            if candidate.is_file() {
                skill_files.push(candidate);
            }
        }
    }
    skill_files.sort();
    if skill_files.is_empty() {
        failures
            .push(".agents/skills: no skills found (expected at least one */SKILL.md)".to_string());
    }
    for skill_file in &skill_files {
        check_skill(skill_file, root, &mut failures);
    }

    check_claude_skills_link(root, &mut failures);
    check_agents_md(root, &mut failures);
    check_claude_md(root, &mut failures);
    if let Ok(agents_text) = std::fs::read_to_string(root.join("AGENTS.md")) {
        check_doc_map_orphans(root, &agents_text, &mut failures);
        check_doc_map_dangling(root, &agents_text, &mut failures);
        check_archive_consistency(root, &agents_text, &mut failures);
    }
    check_internal_links(root, &mut failures);
    check_design_freshness(root, &mut failures);

    if failures.is_empty() {
        println!(
            "agent-doc smoke check ok: {} skill(s), AGENTS.md, CLAUDE.md, .claude/skills, docs/ integrity",
            skill_files.len()
        );
        return ExitCode::SUCCESS;
    }
    eprintln!("agent-doc smoke check FAILED:");
    for failure in &failures {
        eprintln!("  - {failure}");
    }
    ExitCode::FAILURE
}

fn check_skill(path: &Path, root: &Path, failures: &mut Vec<String>) {
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned();
    let Ok(text) = std::fs::read_to_string(path) else {
        failures.push(format!("{rel}: unreadable"));
        return;
    };
    let (frontmatter_text, body) = match split_frontmatter(&text) {
        Ok(parts) => parts,
        Err(reason) => {
            failures.push(format!("{rel}: {reason}"));
            return;
        }
    };

    let frontmatter = parse_frontmatter(&frontmatter_text, &rel, failures);

    let unknown: Vec<&String> = frontmatter
        .keys()
        .filter(|key| !ALLOWED_KEYS.contains(&key.as_str()))
        .collect();
    if !unknown.is_empty() {
        failures.push(format!(
            "{rel}: unsupported frontmatter keys {unknown:?} — only {ALLOWED_KEYS:?} are allowed; activation conditions belong in `description`, not custom fields"
        ));
    }

    let dir_name = path
        .parent()
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    match frontmatter.get("name") {
        None => failures.push(format!("{rel}: frontmatter requires `name`")),
        Some(name) if *name != dir_name => failures.push(format!(
            "{rel}: name {name:?} must equal directory name {dir_name:?}"
        )),
        Some(_) => {}
    }

    match frontmatter.get("description") {
        None => failures.push(format!("{rel}: frontmatter requires `description`")),
        Some(description) if !(1..=1024).contains(&description.chars().count()) => {
            failures.push(format!(
                "{rel}: description must be 1..1024 chars (got {})",
                description.chars().count()
            ));
        }
        Some(_) => {}
    }

    if body.chars().count() > SKILL_BODY_BUDGET_CHARS {
        failures.push(format!(
            "{rel}: body is {} chars, exceeding the {SKILL_BODY_BUDGET_CHARS}-char budget — move detail into reference files inside the skill directory",
            body.chars().count()
        ));
    }
}

/// Split raw file text into (frontmatter, body), keeping the two failure
/// modes apart for actionable messages. Windows checkouts may carry CRLF
/// line endings (git autocrlf); YAML hosts accept both, so normalize before
/// the line-oriented split.
fn split_frontmatter(text: &str) -> Result<(String, String), &'static str> {
    let text = text.replace("\r\n", "\n");
    let Some(rest) = text.strip_prefix("---\n") else {
        return Err("missing YAML frontmatter (must start with ---)");
    };
    let Some(end) = rest.find("\n---\n") else {
        return Err("frontmatter is not closed with ---");
    };
    Ok((rest[..end].to_string(), rest[end + 5..].to_string()))
}

/// Parse the single-line `key: value` subset this checker supports. There is
/// no YAML parser in std, so validate against what strict YAML hosts reject:
/// an unquoted `: ` (or trailing `:`) ends the scalar there, and a leading
/// indicator char starts a construct this line-based format forbids.
fn parse_frontmatter(text: &str, rel: &str, failures: &mut Vec<String>) -> HashMap<String, String> {
    let mut frontmatter = HashMap::new();
    for line in text.lines() {
        let stripped = line.trim();
        if stripped.is_empty() || stripped.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            failures.push(format!("{rel}: unparseable frontmatter line: {line:?}"));
            continue;
        };
        let key = key.trim().to_string();
        let value = value.trim();
        if value.starts_with('"') || value.starts_with('\'') {
            let quote = value.as_bytes()[0];
            if value.len() < 2 || !value.ends_with(char::from(quote)) {
                failures.push(format!("{rel}: unterminated quoted scalar for {key:?}"));
                continue;
            }
        } else if !value.is_empty() {
            if value.contains(": ") || value.ends_with(':') {
                failures.push(format!(
                    "{rel}: plain scalar for {key:?} contains ':' — strict YAML hosts reject this; wrap the value in double quotes"
                ));
                continue;
            }
            if ">|&*!%@`[{".contains(value.as_bytes()[0] as char) {
                failures.push(format!(
                    "{rel}: value for {key:?} starts with a YAML indicator — only single-line plain or quoted scalars are supported in SKILL.md frontmatter"
                ));
                continue;
            }
        }
        frontmatter.insert(key, value.trim_matches(['"', '\'']).to_string());
    }
    frontmatter
}

fn check_claude_skills_link(root: &Path, failures: &mut Vec<String>) {
    let link = root.join(".claude/skills");
    let expected = Path::new("../.agents/skills");
    match std::fs::symlink_metadata(&link) {
        Ok(metadata) if metadata.file_type().is_symlink() => match std::fs::read_link(&link) {
            Ok(target) if target == expected => {}
            Ok(target) => failures.push(format!(
                ".claude/skills: symlink must point to ../.agents/skills (got {})",
                target.display()
            )),
            Err(err) => failures.push(format!(".claude/skills: unreadable symlink: {err}")),
        },
        Ok(metadata) if metadata.is_file() => {
            // git on platforms without symlink support materializes a text
            // file containing the link target; accept that degradation.
            match std::fs::read_to_string(&link) {
                Ok(content) if content.trim() == "../.agents/skills" => {}
                Ok(content) => failures.push(format!(
                    ".claude/skills: unexpected pointer content {content:?}"
                )),
                Err(err) => failures.push(format!(".claude/skills: unreadable: {err}")),
            }
        }
        _ => failures
            .push(".claude/skills: missing — it must symlink to ../.agents/skills".to_string()),
    }
}

fn check_agents_md(root: &Path, failures: &mut Vec<String>) {
    let path = root.join("AGENTS.md");
    let Ok(text) = std::fs::read_to_string(&path) else {
        failures.push("AGENTS.md: missing".to_string());
        return;
    };
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() > AGENTS_LINE_BUDGET {
        failures.push(format!(
            "AGENTS.md: {} lines exceeds the {AGENTS_LINE_BUDGET}-line budget",
            lines.len()
        ));
    }
    if !lines
        .iter()
        .take(5)
        .any(|line| line.contains("Last reviewed"))
    {
        failures.push(
            "AGENTS.md: missing the freshness note (Last reviewed) in the header".to_string(),
        );
    }
}

/// Doc-map orphan check: the AGENTS.md Documentation section is the map an
/// agent reads first; a committed doc absent from it is invisible in
/// practice. Every `docs/*.md`, `docs/design/*.md` and `docs/research/*.md`
/// file must therefore appear as a relative path in AGENTS.md.
/// (docs/decisions is exempt: the four-digit number sequence is the index.)
fn check_doc_map_orphans(root: &Path, agents_text: &str, failures: &mut Vec<String>) {
    let mut docs: Vec<String> = Vec::new();
    for dir in ["docs", "docs/design", "docs/research"] {
        let Ok(entries) = std::fs::read_dir(root.join(dir)) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                docs.push(format!("{dir}/{}", entry.file_name().to_string_lossy()));
            }
        }
    }
    docs.sort();
    for rel in find_doc_map_orphans(&docs, agents_text) {
        failures.push(format!(
            "{rel}: absent from AGENTS.md (doc-map orphan — every committed doc needs a map entry)"
        ));
    }
}

fn find_doc_map_orphans(docs: &[String], agents_text: &str) -> Vec<String> {
    docs.iter()
        .filter(|rel| !agents_text.contains(rel.as_str()))
        .cloned()
        .collect()
}

/// Reverse dangling check: a map entry pointing at a deleted doc is the same
/// rot as an orphan, one direction over. Every `docs/...` path token mentioned
/// in AGENTS.md must exist (file or directory).
fn check_doc_map_dangling(root: &Path, agents_text: &str, failures: &mut Vec<String>) {
    for token in extract_doc_paths(agents_text) {
        if !root.join(&token).exists() {
            failures.push(format!(
                "AGENTS.md: {token} does not exist (reverse dangling — map entry points at a deleted doc)"
            ));
        }
    }
}

/// Extract `docs/...` path tokens from prose: start at each `docs/` occurrence
/// and extend over path characters only (CJK punctuation, backticks and ASCII
/// delimiters all terminate the token).
fn extract_doc_paths(text: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut rest = text;
    while let Some(idx) = rest.find("docs/") {
        let after = &rest[idx..];
        // The matched prefix "docs/" is itself five path characters, so the
        // scan always advances and cannot loop.
        let end = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | '-')))
            .unwrap_or(after.len());
        let token = after[..end].trim_end_matches(['/', '.']).to_string();
        if token != "docs" && !tokens.contains(&token) {
            tokens.push(token);
        }
        rest = &after[end..];
    }
    tokens.sort();
    tokens
}

/// Internal link integrity: relative `.md` link targets in `docs/**.md` and
/// AGENTS.md must resolve. Only the file part is checked — anchors are
/// prose-level and stay unchecked (the machine promises structure, not
/// semantics). External URLs, `mailto:` and pure anchors are out of scope.
fn check_internal_links(root: &Path, failures: &mut Vec<String>) {
    let mut files: Vec<PathBuf> = Vec::new();
    collect_md_files(&root.join("docs"), &mut files);
    files.push(root.join("AGENTS.md"));
    files.sort();
    for file in files {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let rel = file
            .strip_prefix(root)
            .unwrap_or(&file)
            .to_string_lossy()
            .into_owned();
        let parent = file.parent().unwrap_or(root);
        for target in find_md_link_targets(&text) {
            if !parent.join(&target).exists() {
                failures.push(format!("{rel}: broken internal link -> {target}"));
            }
        }
    }
}

fn collect_md_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_md_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "md") {
            out.push(path);
        }
    }
}

/// Pull the file parts of relative markdown link targets: `](target)` where
/// the target is not external (`scheme://`, `mailto:`), not a pure anchor
/// (`#...`), and names a `.md` file. `#anchor` suffixes are stripped. Fenced
/// code blocks are excluded — literal markup examples are not links. Targets
/// carrying a title (`](path "title")`) are skipped rather than misparsed.
fn find_md_link_targets(text: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut in_fence = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let mut rest = line;
        while let Some(idx) = rest.find("](") {
            let after = &rest[idx + 2..];
            let Some(close) = after.find(')') else {
                break;
            };
            let raw = after[..close].trim();
            rest = &after[close + 1..];
            let raw = raw
                .strip_prefix('<')
                .and_then(|r| r.strip_suffix('>'))
                .unwrap_or(raw);
            if raw.starts_with('#') || raw.contains("://") || raw.starts_with("mailto:") {
                continue;
            }
            // An empty file part (pure anchor) has no extension and is
            // skipped by the same predicate.
            let file_part = raw.split('#').next().unwrap_or("").trim();
            let is_md_file = Path::new(file_part)
                .extension()
                .is_some_and(|ext| ext == "md");
            if !is_md_file {
                continue;
            }
            targets.push(file_part.to_string());
        }
    }
    targets
}

/// Freshness metadata: every active `docs/design/*.md` carries
/// `Last reviewed: YYYY-MM` in its header. Archived docs are exempt —
/// archiving freezes the content. CI promises presence and format only;
/// whether the date was honestly bumped on content changes is reconciled by
/// the gardener patrol against git history.
fn check_design_freshness(root: &Path, failures: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(root.join("docs/design")) else {
        return;
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "md"))
        .collect();
    files.sort();
    for file in files {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let header = header_of(&text);
        if is_archived(header) {
            continue;
        }
        let rel = file
            .strip_prefix(root)
            .unwrap_or(&file)
            .to_string_lossy()
            .into_owned();
        if !has_freshness(header) {
            failures.push(format!(
                "{rel}: missing `Last reviewed: YYYY-MM` in the header (active design docs carry freshness metadata; bump on every content change)"
            ));
        }
    }
}

/// Archive consistency, both directions: a doc whose header carries the
/// archive marker must have an `archived` annotation on its AGENTS.md map
/// line, and an `archived` map annotation must correspond to a marked doc.
/// Applies to design/research; decisions use `superseded by` markers instead.
/// A doc absent from the map is the orphan check's job, not this one's.
fn check_archive_consistency(root: &Path, agents_text: &str, failures: &mut Vec<String>) {
    for dir in ["docs/design", "docs/research"] {
        let Ok(entries) = std::fs::read_dir(root.join(dir)) else {
            continue;
        };
        let mut names: Vec<String> = entries
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                (path.is_file() && path.extension().is_some_and(|ext| ext == "md"))
                    .then(|| entry.file_name().to_string_lossy().into_owned())
            })
            .collect();
        names.sort();
        for name in names {
            let rel = format!("{dir}/{name}");
            let Ok(text) = std::fs::read_to_string(root.join(&rel)) else {
                continue;
            };
            let doc_archived = is_archived(header_of(&text));
            let Some(map_line) = find_map_entry(agents_text, &rel) else {
                continue;
            };
            let map_archived = map_line.contains("archived");
            match (doc_archived, map_archived) {
                (true, false) => failures.push(format!(
                    "{rel}: archived doc without an `archived` annotation on its AGENTS.md map line"
                )),
                (false, true) => failures.push(format!(
                    "{rel}: AGENTS.md map line says `archived` but the doc header lacks the archive marker (📦 已归档)"
                )),
                _ => {}
            }
        }
    }
}

/// The header is where status lines live: the first handful of lines.
fn header_of(text: &str) -> &str {
    let mut end = text.len();
    for (count, (idx, _)) in text.match_indices('\n').enumerate() {
        if count == 9 {
            end = idx;
            break;
        }
    }
    &text[..end]
}

fn is_archived(header: &str) -> bool {
    header.contains("📦 已归档")
}

/// `Last reviewed: ` followed by a plausible `YYYY-MM`.
fn has_freshness(text: &str) -> bool {
    const MARKER: &str = "Last reviewed: ";
    let Some(idx) = text.find(MARKER) else {
        return false;
    };
    let date: String = text[idx + MARKER.len()..].chars().take(7).collect();
    let bytes = date.as_bytes();
    bytes.len() == 7
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && (1..=12).contains(&date[5..7].parse::<u32>().unwrap_or(0))
}

/// The map entry for a doc is the line whose first backtick-quoted token is
/// exactly the doc's path — prose mentions of the path elsewhere (e.g. in the
/// section intro line) are not the entry.
fn find_map_entry<'a>(agents_text: &'a str, rel: &str) -> Option<&'a str> {
    agents_text.lines().find(|line| {
        let Some(start) = line.find('`') else {
            return false;
        };
        let Some(end) = line[start + 1..].find('`') else {
            return false;
        };
        &line[start + 1..start + 1 + end] == rel
    })
}

fn check_claude_md(root: &Path, failures: &mut Vec<String>) {
    let path = root.join("CLAUDE.md");
    let Ok(text) = std::fs::read_to_string(&path) else {
        failures.push("CLAUDE.md: missing (must be a one-line reference to AGENTS.md)".to_string());
        return;
    };
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("<!--"))
        .collect();
    if lines != ["@AGENTS.md"] {
        failures.push(
            "CLAUDE.md: must contain only the @AGENTS.md reference (SSOT — never a copy)"
                .to_string(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> (HashMap<String, String>, Vec<String>) {
        let mut failures = Vec::new();
        let frontmatter = parse_frontmatter(text, "test/SKILL.md", &mut failures);
        (frontmatter, failures)
    }

    #[test]
    fn split_tolerates_crlf_checkouts() {
        // git materializes CRLF on Windows checkouts (autocrlf); YAML hosts
        // accept both endings, so the checker must too.
        let (frontmatter, body) =
            split_frontmatter("---\r\nname: x\r\n---\r\nbody\r\n").expect("valid frontmatter");
        assert_eq!(frontmatter, "name: x");
        assert_eq!(body, "body\n");
    }

    #[test]
    fn split_distinguishes_open_and_close_failures() {
        assert_eq!(
            split_frontmatter("name: x\n").unwrap_err(),
            "missing YAML frontmatter (must start with ---)"
        );
        assert_eq!(
            split_frontmatter("---\nname: x\n").unwrap_err(),
            "frontmatter is not closed with ---"
        );
    }

    #[test]
    fn doc_map_orphans_flags_only_unreferenced_docs() {
        let docs = vec![
            "docs/design/fiber.md".to_string(),
            "docs/research/cordis-research.md".to_string(),
            "docs/backlog.md".to_string(),
        ];
        let agents = "- `docs/design/fiber.md` — fiber model\n- `docs/backlog.md` — ledger\n";
        assert_eq!(
            find_doc_map_orphans(&docs, agents),
            vec!["docs/research/cordis-research.md".to_string()]
        );
        let fully_mapped = "docs/design/fiber.md docs/research/cordis-research.md docs/backlog.md";
        assert!(find_doc_map_orphans(&docs, fully_mapped).is_empty());
    }

    #[test]
    fn doc_paths_extract_files_and_dirs_only() {
        let text = "- `docs/design/fiber.md` — x\n- `docs/decisions/` — ADRs\n地图见 docs/backlog.md。重复 docs/design/fiber.md";
        assert_eq!(
            extract_doc_paths(text),
            vec![
                "docs/backlog.md".to_string(),
                "docs/decisions".to_string(),
                "docs/design/fiber.md".to_string(),
            ]
        );
        // A bare `docs/` prefix never yields a token.
        assert!(extract_doc_paths("住 docs/ 之下").is_empty());
    }

    #[test]
    fn md_link_targets_skip_external_anchors_and_fences() {
        let text = "见 [ADR](0012-docs-governance.md) 与 [节](fiber.md#s1) 与\n\
                    [外链](https://example.com/x.md) 与 [纯锚](#s1) 与 [图](../assets/x.png)。\n\
                    ```\n[示例](fake.md)\n```\n\
                    [带标题](title.md \"题注\") 跳过";
        assert_eq!(
            find_md_link_targets(text),
            vec![
                "0012-docs-governance.md".to_string(),
                "fiber.md".to_string(),
            ]
        );
    }

    #[test]
    fn freshness_accepts_plausible_months_only() {
        assert!(has_freshness("> 状态：living\n> Last reviewed: 2026-10\n"));
        assert!(!has_freshness("> Last reviewed: 2026/10\n"));
        assert!(!has_freshness("> Last reviewed: 2026-13\n"));
        assert!(!has_freshness("> Last reviewed: 2026-1\n"));
        assert!(!has_freshness("> 状态：living，无日期\n"));
    }

    #[test]
    fn archived_marker_detected_in_header() {
        assert!(is_archived("> 状态：📦 已归档（2026-10-14）\n"));
        assert!(!is_archived("> 状态：living design doc\n"));
    }

    #[test]
    fn map_entry_is_the_line_whose_first_backtick_token_is_the_path() {
        let agents = "- `docs/design/` — intro mentioning `docs/design/fiber.md` in prose\n  - `docs/design/fiber.md` — fiber model (archived 2026-10-14)\n";
        assert_eq!(
            find_map_entry(agents, "docs/design/fiber.md"),
            Some("  - `docs/design/fiber.md` — fiber model (archived 2026-10-14)")
        );
        assert_eq!(find_map_entry(agents, "docs/design/absent.md"), None);
    }

    #[test]
    fn plain_scalars_parse() {
        let (frontmatter, failures) = parse("name: pr-preflight\ndescription: short\n");
        assert!(failures.is_empty());
        assert_eq!(frontmatter["description"], "short");
    }

    #[test]
    fn unquoted_colon_space_is_rejected() {
        let (_, failures) = parse("description: layer on top: diff self-review\n");
        assert_eq!(failures.len(), 1);
        assert!(failures[0].contains("wrap the value in double quotes"));
    }

    #[test]
    fn quoted_colon_space_is_accepted() {
        let (frontmatter, failures) = parse("description: \"layer on top: diff\"\n");
        assert!(failures.is_empty());
        assert_eq!(frontmatter["description"], "layer on top: diff");
    }

    #[test]
    fn unterminated_quote_and_indicators_are_rejected() {
        let (_, failures) = parse("description: \"never closed\n");
        assert_eq!(failures.len(), 1);
        let (_, failures) = parse("description: >- folded\n");
        assert_eq!(failures.len(), 1);
    }
}
