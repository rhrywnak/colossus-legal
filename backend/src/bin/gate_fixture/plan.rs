//! The repeatable `CODE=VALUE` flags, parsed once into one lookup per scenario.
//!
//! ## Why the counts and the file names are flags and not constants
//!
//! Standing Rule 2. `292`, `44`, `10`, `s11_gate_fixture_v1.json` and the seven
//! `$50,000` evidence ids are all facts about ONE case on ONE day; compiling any
//! of them in would make this tool useless to the next case and — worse — would
//! let a later edit quietly change what the fixture claims. Everything
//! case-specific arrives on the command line, and the README the tool writes
//! records exactly what was passed.
//!
//! ## Rust Learning: `split_once` rather than `split('=')`
//!
//! `split_once('=')` cuts at the FIRST `=` and returns the rest untouched, so a
//! value containing an `=` survives. `split('=')` would shatter it into pieces
//! and the caller would have to guess how to put it back.

use std::collections::HashMap;
use std::process::ExitCode;

use colossus_legal_backend::oneshot::exit::EXIT_BAD_INPUT;
use colossus_legal_backend::services::gate_fixture::ExpectedCounts;
use tracing::error;

use crate::Args;

/// Everything the command line said about each scenario, indexed by code.
pub(crate) struct Plan {
    runs: HashMap<String, String>,
    expects: HashMap<String, ExpectedCounts>,
    files: HashMap<String, String>,
    outside: HashMap<String, Vec<String>>,
}

impl Plan {
    pub(crate) fn from_args(args: &Args) -> Result<Self, ExitCode> {
        let mut expects = HashMap::new();
        for (code, spec) in split_pairs(&args.expects)? {
            expects.insert(code, parse_counts(&spec)?);
        }
        let mut outside: HashMap<String, Vec<String>> = HashMap::new();
        for (code, id) in split_pairs(&args.outside)? {
            outside.entry(code).or_default().push(id);
        }
        Ok(Self {
            runs: split_pairs(&args.runs)?.into_iter().collect(),
            expects,
            files: split_pairs(&args.files)?.into_iter().collect(),
            outside,
        })
    }

    pub(crate) fn run_date(&self, code: &str) -> Result<&str, ExitCode> {
        required(self.runs.get(code).map(String::as_str), code, "--run")
    }

    pub(crate) fn expected_counts(&self, code: &str) -> Result<ExpectedCounts, ExitCode> {
        match self.expects.get(code) {
            Some(c) => Ok(*c),
            None => Err(missing(code, "--expect")),
        }
    }

    pub(crate) fn file(&self, code: &str) -> Result<&str, ExitCode> {
        required(self.files.get(code).map(String::as_str), code, "--file")
    }

    pub(crate) fn outside_ids(&self, code: &str) -> &[String] {
        // An empty list is a legitimate answer (a scenario with no known hidden
        // card), so this one is NOT required — unlike the three above, whose
        // absence would silently change what the fixture claims.
        self.outside.get(code).map(Vec::as_slice).unwrap_or(&[])
    }
}

/// Split `CODE=VALUE` on the FIRST `=`, so a value may contain one.
fn split_pairs(raw: &[String]) -> Result<Vec<(String, String)>, ExitCode> {
    raw.iter()
        .map(|item| {
            item.split_once('=')
                .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
                .ok_or_else(|| {
                    error!(argument = %item, "expected CODE=VALUE, e.g. S-11=2026-08-29");
                    ExitCode::from(EXIT_BAD_INPUT)
                })
        })
        .collect()
}

/// `candidates/relevant/included/outside` → four numbers.
///
/// ## Rust Learning: collecting into `Result`, not `Option`
///
/// `Vec<Result<T, E>>` collects into `Result<Vec<T>, E>`, short-circuiting on the
/// FIRST error and carrying it out. The earlier version of this used
/// `.parse().ok()` and collected into an `Option`, which works — a `None` drives
/// the error arm below — but it throws the parse error away, so the operator was
/// told the whole spec was malformed without being told which field. Collecting
/// the `Result` keeps the reason, and there is no `.ok()` discarding anything.
fn parse_counts(spec: &str) -> Result<ExpectedCounts, ExitCode> {
    let parts: Vec<&str> = spec.split('/').collect();
    let numbers: Vec<usize> = parts
        .iter()
        .map(|p| p.trim().parse::<usize>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| {
            error!(
                spec = %spec, error = %source,
                "a count is not a number — expected candidates/relevant/included/outside, e.g. 292/44/10/7"
            );
            ExitCode::from(EXIT_BAD_INPUT)
        })?;

    match numbers.as_slice() {
        [candidates, opus_relevant, included, outside_pool] => Ok(ExpectedCounts {
            candidates: *candidates,
            opus_relevant: *opus_relevant,
            included: *included,
            outside_pool: *outside_pool,
        }),
        other => {
            error!(
                spec = %spec, fields = other.len(),
                "expected exactly four counts — candidates/relevant/included/outside, e.g. 292/44/10/7"
            );
            Err(ExitCode::from(EXIT_BAD_INPUT))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Four good numbers parse, in the documented order.
    #[test]
    fn four_counts_parse_in_order() {
        let counts = parse_counts("292/44/10/7").expect("a well-formed spec");
        assert_eq!(counts.candidates, 292);
        assert_eq!(counts.opus_relevant, 44);
        assert_eq!(counts.included, 10);
        assert_eq!(counts.outside_pool, 7);
    }

    /// Too few fields is rejected rather than silently padded.
    ///
    /// The slice pattern below `parse_counts` is the load-bearing part: if a
    /// fifth count is ever added, the pattern must change with it, and this is
    /// what fails in a millisecond if it does not.
    #[test]
    fn three_counts_are_rejected() {
        assert!(parse_counts("292/44/10").is_err());
    }

    /// Five fields is rejected too — the arm is exact, not "at least four".
    #[test]
    fn five_counts_are_rejected() {
        assert!(parse_counts("292/44/10/7/1").is_err());
    }

    /// A non-numeric field is rejected. Before the Result collect, the operator
    /// was told the spec was wrong but not that "ten" was the reason.
    #[test]
    fn a_non_numeric_count_is_rejected() {
        assert!(parse_counts("292/44/ten/7").is_err());
    }

    /// `CODE=VALUE` splits on the FIRST `=`, so a value may contain one.
    #[test]
    fn a_pair_splits_on_the_first_equals_only() {
        let pairs = split_pairs(&["S-11=doc:evidence:abc=def".to_string()]).expect("well formed");
        assert_eq!(
            pairs,
            vec![("S-11".to_string(), "doc:evidence:abc=def".to_string())]
        );
    }

    /// An argument with no `=` at all is rejected, not silently skipped. A
    /// dropped `--outside` id would make the fixture short by one card and the
    /// count line would be the only clue.
    #[test]
    fn an_argument_without_an_equals_is_rejected() {
        assert!(split_pairs(&["S-11-no-equals".to_string()]).is_err());
    }
}

fn required<'a>(value: Option<&'a str>, code: &str, flag: &str) -> Result<&'a str, ExitCode> {
    value.ok_or_else(|| missing(code, flag))
}

fn missing(code: &str, flag: &str) -> ExitCode {
    error!(scenario = %code, flag = %flag, "no value given for this scenario");
    ExitCode::from(EXIT_BAD_INPUT)
}

// ── Building one fixture ──────────────────────────────────────────────────────
