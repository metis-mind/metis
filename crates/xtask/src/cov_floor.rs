//! Kernel-crate line-coverage floors (testing-strategy D3 / §3.C). Floor
//! values live in `.config/cov-floor.toml` — gate surface, since lowering a
//! floor weakens the gate — pinned from measured coverage at the owning
//! station's closeout and ratcheted only upward. With no floors pinned the
//! check prints a note and passes: the mechanism is what the infrastructure
//! station ships; kernel stations pin the numbers.
//!
//! Std-only like the rest of xtask: the floor file is a flat
//! `crate = percent` table parsed by hand, and the measured number is lifted
//! out of `cargo llvm-cov -p <crate> --json --summary-only` by anchored
//! string surgery on the one field path we need.

use std::process::{Command, ExitCode};

const FLOOR_FILE: &str = ".config/cov-floor.toml";

/// Comparison epsilon: llvm-cov percentages are f64 ratios while the floor
/// file spells them in decimal — a pinned floor must not fail on float noise
/// in the trailing digits. 0.01 keeps 0.1-granularity floors meaningful
/// (a real 0.1-step drop still fails).
const EPSILON: f64 = 0.01;

/// Entry point: `cov-floor` (no arguments).
pub fn run(args: &[String]) -> ExitCode {
    if !args.is_empty() {
        eprintln!("usage: cov-floor  (no arguments; floors live in {FLOOR_FILE})");
        return ExitCode::from(2);
    }
    let text = match std::fs::read_to_string(FLOOR_FILE) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("error: cannot read {FLOOR_FILE}: {err}");
            return ExitCode::from(2);
        }
    };
    let floors = match parse_floors(&text) {
        Ok(floors) => floors,
        Err(err) => {
            eprintln!("error: {FLOOR_FILE}: {err}");
            return ExitCode::from(2);
        }
    };
    if floors.is_empty() {
        println!(
            "cov-floor: no floors pinned ({FLOOR_FILE} is empty) — kernel stations pin at closeout"
        );
        return ExitCode::SUCCESS;
    }
    let mut failures = Vec::new();
    for (krate, floor) in &floors {
        let measured = match measure(krate) {
            Ok(percent) => percent,
            Err(code) => return code,
        };
        if measured + EPSILON >= *floor {
            println!("cov-floor: {krate}: {measured:.2}% >= floor {floor:.2}%");
        } else {
            failures.push(format!("{krate}: {measured:.2}% < floor {floor:.2}%"));
        }
    }
    if failures.is_empty() {
        return ExitCode::SUCCESS;
    }
    eprintln!("cov-floor: coverage dropped below the pinned floor:");
    for failure in &failures {
        eprintln!("  - {failure}");
    }
    eprintln!("the floor only ratchets upward (testing-strategy §3.C): restore coverage");
    eprintln!("with tests; lowering a floor is a gate weakening, not a trailer-level move");
    ExitCode::FAILURE
}

/// The pinned floors as (crate name, percent) pairs, in file order.
/// Accepts only `name = percent` lines plus blanks and `#` comments
/// (whole-line or inline) — the
/// file is gate surface, so the parser is strict about what it admits.
fn parse_floors(text: &str) -> Result<Vec<(String, f64)>, String> {
    let mut floors: Vec<(String, f64)> = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.split_once('#').map_or(raw, |(body, _)| body).trim();
        if line.is_empty() {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            return Err(format!(
                "line {}: expected `crate-name = percent`",
                index + 1
            ));
        };
        let name = name.trim();
        let valid = !name.is_empty()
            && name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_');
        if !valid {
            return Err(format!("line {}: bad crate name {name:?}", index + 1));
        }
        if floors.iter().any(|(seen, _)| seen == name) {
            return Err(format!("line {}: duplicate floor for {name}", index + 1));
        }
        let percent: f64 = value
            .trim()
            .parse()
            .map_err(|_| format!("line {}: bad percent {value:?}", index + 1))?;
        if !(0.0..=100.0).contains(&percent) {
            return Err(format!(
                "line {}: percent {percent} out of range 0..=100",
                index + 1
            ));
        }
        floors.push((name.to_string(), percent));
    }
    Ok(floors)
}

/// Measured line coverage of one crate via llvm-cov's JSON summary.
fn measure(krate: &str) -> Result<f64, ExitCode> {
    let output = match Command::new("cargo")
        .args(["llvm-cov", "-p", krate, "--json", "--summary-only"])
        .output()
    {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            eprintln!(
                "error: cargo llvm-cov -p {krate} failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            return Err(ExitCode::from(2));
        }
        Err(err) => {
            eprintln!("error: cannot run cargo llvm-cov: {err}");
            return Err(ExitCode::from(2));
        }
    };
    let json = String::from_utf8_lossy(&output.stdout);
    lines_percent(&json).ok_or_else(|| {
        eprintln!("error: cannot find totals.lines.percent in llvm-cov output for {krate}");
        ExitCode::from(2)
    })
}

/// Lift `totals.lines.percent` out of the summary JSON without a JSON
/// parser: anchor on `totals` first so the per-file `lines` summaries in
/// `data[].files[]` are never matched.
fn lines_percent(json: &str) -> Option<f64> {
    let totals = json.find("\"totals\"")?;
    let lines = totals + json[totals..].find("\"lines\"")?;
    let anchor = lines + json[lines..].find("\"percent\":")?;
    let start = anchor + "\"percent\":".len();
    let end = start + json[start..].find([',', '}'])?;
    json[start..end].trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floors_parse_strictly() {
        assert_eq!(
            parse_floors(
                "# comment\n\nmetis-value = 72.5  # pinned at closeout\nmetis_fiber_core = 80\n"
            )
            .unwrap(),
            vec![
                ("metis-value".to_string(), 72.5),
                ("metis_fiber_core".to_string(), 80.0)
            ]
        );
        assert!(parse_floors("").unwrap().is_empty());
        assert!(parse_floors("metis-value 72.5").is_err());
        assert!(parse_floors("Bad Name = 1").is_err());
        assert!(parse_floors("metis-value = soon").is_err());
        assert!(parse_floors("metis-value = 101").is_err());
        assert!(parse_floors("metis-value = 70\nmetis-value = 72").is_err());
    }

    #[test]
    fn lines_percent_anchors_on_totals() {
        let json = r#"{"data":[{"files":[{"filename":"src/lib.rs","summary":{"lines":{"count":10,"covered":9,"percent":90.0}}}],"totals":{"functions":{"count":1,"covered":1,"percent":100.0},"lines":{"count":40,"covered":29,"percent":72.5}}}]}"#;
        assert_eq!(lines_percent(json), Some(72.5));
        assert_eq!(lines_percent("{\"data\":[]}"), None);
        assert_eq!(lines_percent("not json"), None);
    }

    #[test]
    fn epsilon_absorbs_float_noise_only() {
        // black_box keeps clippy from seeing these as constant assertions
        let measured = std::hint::black_box(29.0) / 40.0 * 100.0;
        assert!(measured + EPSILON >= 72.5);
        // noise below the pinned decimal is absorbed
        let noisy = std::hint::black_box(72.499_999);
        assert!(noisy + EPSILON >= 72.5);
        // a real 0.1-step drop still fails
        let dropped = std::hint::black_box(72.4);
        assert!(dropped + EPSILON < 72.5);
    }
}
