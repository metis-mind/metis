//! Test-bloat telemetry (testing-strategy D12 / §3.H): per-crate product vs
//! test line counts and `#[test]` counts, printed as a diagnostic table for
//! the slow-track report. Diagnostic only — no gate, no history (growth-rate
//! history is deliberately not built: 2026-10-15 Q5 ratification).
//!
//! Counting is deliberately approximate: files under `tests/` are all test
//! lines; inline `#[cfg(test)]` modules in `src/` are found by brace
//! tracking, which string/comment braces can skew, and the `#[test]` count
//! is a raw substring match (mentions in prose count too). The numbers are a
//! bloat signal, not a measurement instrument — that is all §3.H asks of them.

use std::process::ExitCode;

/// Entry point: `test-telemetry` (no arguments).
pub fn run(args: &[String]) -> ExitCode {
    if !args.is_empty() {
        eprintln!("usage: test-telemetry  (no arguments)");
        return ExitCode::from(2);
    }
    let crates = match std::fs::read_dir("crates") {
        Ok(entries) => entries,
        Err(err) => {
            eprintln!("error: cannot read crates/: {err}");
            return ExitCode::from(2);
        }
    };
    let mut rows = Vec::new();
    for entry in crates {
        let Ok(entry) = entry else { continue };
        let dir = entry.path();
        if !dir.is_dir() || !dir.join("Cargo.toml").exists() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let mut counts = Counts::default();
        for file in rust_files(&dir.join("src")) {
            counts.add_source(&file);
        }
        for file in rust_files(&dir.join("tests")) {
            counts.add_test_file(&file);
        }
        rows.push((name, counts));
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    println!(
        "{:<24} {:>8} {:>8} {:>7} {:>7}",
        "crate", "product", "test", "ratio", "#[test]"
    );
    let mut total = Counts::default();
    for (name, counts) in &rows {
        println!("{name:<24} {}", counts.row());
        total.product += counts.product;
        total.test += counts.test;
        total.tests += counts.tests;
    }
    println!("{:<24} {}", "TOTAL", total.row());
    ExitCode::SUCCESS
}

/// Line counts for one crate.
#[derive(Default)]
struct Counts {
    /// non-test lines under src/
    product: usize,
    /// lines under tests/ plus inline `#[cfg(test)]` module lines
    test: usize,
    /// `#[test]` occurrences anywhere in the crate
    tests: usize,
}

impl Counts {
    /// Diagnostic ratio: line counts never approach 2^52, so the usize→f64
    /// casts here are exact for our purposes.
    #[allow(clippy::cast_precision_loss)]
    fn row(&self) -> String {
        let ratio = if self.product == 0 {
            "-".to_string()
        } else {
            format!("{:.2}", self.test as f64 / self.product as f64)
        };
        format!(
            "{:>8} {:>8} {:>7} {:>7}",
            self.product, self.test, ratio, self.tests
        )
    }

    fn add_source(&mut self, path: &std::path::Path) {
        let Ok(source) = std::fs::read_to_string(path) else {
            return;
        };
        let (product, test) = split_source_lines(&source);
        self.product += product;
        self.test += test;
        self.tests += source.matches("#[test]").count();
    }

    fn add_test_file(&mut self, path: &std::path::Path) {
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        self.test += text.lines().count();
        self.tests += text.matches("#[test]").count();
    }
}

/// All `.rs` files under `dir` (recursive; missing dir = empty).
fn rust_files(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
    }
    files
}

/// Split a source file into (product, test) lines. An inline test module
/// starts at the `mod … {` following a `#[cfg(test)]` attribute and ends at
/// its balancing brace; brace counting is approximate (string/comment braces
/// skew it) — diagnostic numbers, not measurements.
fn split_source_lines(source: &str) -> (usize, usize) {
    let mut product = 0;
    let mut test = 0;
    let mut depth = 0i32;
    // brace depth at which the current cfg(test) module closes
    let mut test_close: Option<i32> = None;
    let mut pending_test = false;
    for line in source.lines() {
        if test_close.is_some() {
            test += 1;
        } else {
            product += 1;
        }
        let trimmed = line.trim_start();
        if trimmed.starts_with("#[cfg(test)]") {
            pending_test = true;
        }
        if pending_test
            && (trimmed.starts_with("mod ") || trimmed.starts_with("pub mod "))
            && let Some(open) = line.find('{')
        {
            depth += 1;
            test_close = Some(depth);
            pending_test = false;
            for ch in line[open + 1..].chars() {
                match ch {
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    _ => {}
                }
            }
            // a one-line module (`mod tests {}`) closes immediately
            if test_close.is_some_and(|close| depth < close) {
                test_close = None;
            }
            continue;
        }
        for ch in line.chars() {
            match ch {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
        }
        if test_close.is_some_and(|close| depth < close) {
            test_close = None;
        }
    }
    (product, test)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_counts_inline_test_module() {
        let src = "fn main() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn works() {}\n}\n";
        // the attribute and `mod` line count as product; the body is test
        assert_eq!(split_source_lines(src), (4, 3));
    }

    #[test]
    fn split_handles_nested_braces_in_tests() {
        let src = "#[cfg(test)]\nmod t {\n    #[test]\n    fn x() {\n        let m = vec![1];\n        assert_eq!(m.len(), 1);\n    }\n}\nfn after() {}\n";
        let (product, test) = split_source_lines(src);
        assert_eq!((product, test), (3, 6));
    }

    #[test]
    fn no_test_module_means_all_product() {
        assert_eq!(split_source_lines("fn a() {}\nfn b() {}\n"), (2, 0));
    }

    #[test]
    fn prose_mentions_do_not_open_test_modules() {
        let src = "//! mentions `#[cfg(test)]` and `mod x {` in prose\nfn real() {}\n";
        assert_eq!(split_source_lines(src), (2, 0));
    }

    #[test]
    fn one_line_test_module_closes_immediately() {
        // the `mod t {}` line counts as product like any trigger line, and a
        // one-line module has no body lines — so all four lines are product
        let src = "fn a() {}\n#[cfg(test)]\nmod t {}\nfn b() {}\n";
        assert_eq!(split_source_lines(src), (4, 0));
    }
}
