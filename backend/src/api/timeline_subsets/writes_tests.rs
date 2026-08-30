//! ⚑ THE WRITE GUARD, PROVED BY SCANNING THE HANDLERS.
//!
//! The rule is that subset reads stay open and subset writes are never
//! anonymous. In axum that is a one-character difference at each handler —
//! `user: AuthUser` versus `user: Option<AuthUser>` — and THE WRONG ONE
//! COMPILES, serves, and answers 200 to a request with no session. There is no
//! type that would have caught it and no runtime symptom to notice: the write
//! simply lands, stamped with whatever the handler decided to do about a `None`.
//!
//! So the rule is enforced by reading the source, the same way the chronology's
//! own write guard is (`api::timeline_write::events_tests`). Every `pub async
//! fn` in this directory must take the non-optional extractor, except the three
//! READS, which are named here as the exceptions they are.
//!
//! The handlers themselves need an `AppState` (two pools, a graph, a registry),
//! which this project has no test tier for. This is what is reachable, and it is
//! the half that would fail silently.

/// The modules whose handlers are scanned.
// STRUCTURAL: repo-internal source paths, not deployment configuration. A moved
// file fails this test rather than quietly leaving a handler unscanned.
const HANDLER_FILES: &[&str] = &[
    "src/api/timeline_subsets/reads.rs",
    "src/api/timeline_subsets/writes.rs",
    "src/api/timeline_subsets/scenario_links.rs",
];

/// The handlers in those files that are READS, and why.
///
/// Looking at a story is not privileged, exactly as looking at the chronology is
/// not (chronology Phase A). Named here so the exception is three lines in a
/// test rather than a judgement somebody makes again per handler.
const OPEN_READ_HANDLERS: &[&str] = &["get_subsets", "get_subset", "get_scenario_subsets"];

/// One file's source with its `//` comments removed.
///
/// ⚑ Required before any scan of this repository's source: this codebase
/// documents its rules next to its rules, so a scanner hunting for
/// `Option<AuthUser>` finds the module header explaining why it is forbidden.
/// The rule is stated once in `domain::wording_tests`.
fn source_without_comments(relative: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} is not readable: {e}", path.display()));
    raw.lines()
        .map(|line| match line.find("//") {
            Some(at) => &line[..at],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `(handler name, the extractor line that follows it)` for every public handler
/// in a scanned file.
fn handlers_in(relative: &str) -> Vec<(String, String)> {
    let source = source_without_comments(relative);
    let mut out = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("pub async fn ") else {
            continue;
        };
        let name = rest.split('(').next().unwrap_or("").trim().to_string();
        // The extractor is the first argument, on the next line — the house
        // formatting for a multi-line signature, which `cargo fmt --check`
        // keeps true.
        let next = lines
            .get(index + 1)
            .copied()
            .unwrap_or("")
            .trim()
            .to_string();
        out.push((name, next));
    }
    out
}

#[test]
fn every_subset_write_handler_demands_an_authenticated_user() {
    // ⚑ THE MUTATION PROOF THE REPORT CITES. Change one handler's `user:
    // AuthUser` to `user: Option<AuthUser>` and this test names it. Without it,
    // an unauthenticated write would be a 200 that nothing in the build noticed.
    let mut anonymous = Vec::new();
    let mut checked = 0usize;

    for file in HANDLER_FILES {
        for (name, first_argument) in handlers_in(file) {
            if OPEN_READ_HANDLERS.contains(&name.as_str()) {
                continue;
            }
            checked += 1;
            if first_argument.contains("Option<AuthUser>") {
                anonymous.push(format!("{file}::{name}"));
            } else if !first_argument.contains("user: AuthUser") {
                anonymous.push(format!(
                    "{file}::{name} takes `{first_argument}` as its first argument, \
                     which is neither extractor this scan understands"
                ));
            }
        }
    }

    assert!(
        anonymous.is_empty(),
        "these subset write handlers would serve an anonymous request:\n  {}",
        anonymous.join("\n  ")
    );
    // ⚑ ANTI-VACUITY. The assertion above is satisfied by finding nothing, so a
    // moved file or a renamed extractor would leave it green while checking no
    // handler at all.
    assert_eq!(
        checked, 7,
        "the scan checked {checked} write handlers, not the seven this feature has \
         — a file moved, or a handler was added without a decision about its guard"
    );
}

#[test]
fn every_open_read_is_named_and_actually_open() {
    // The other direction. A read that quietly became `AuthUser` would 401 an
    // anonymous visitor looking at a page the chronology itself lets them see —
    // and it would look like a permissions bug rather than a regression here.
    let mut wrong = Vec::new();
    let mut found = 0usize;

    for file in HANDLER_FILES {
        for (name, first_argument) in handlers_in(file) {
            if !OPEN_READ_HANDLERS.contains(&name.as_str()) {
                continue;
            }
            found += 1;
            if !first_argument.contains("Option<AuthUser>") {
                wrong.push(format!("{file}::{name} takes `{first_argument}`"));
            }
        }
    }

    assert!(
        wrong.is_empty(),
        "these are named as open reads and are not:\n  {}",
        wrong.join("\n  ")
    );
    assert_eq!(
        found,
        OPEN_READ_HANDLERS.len(),
        "the scan found {found} of the {} named open reads — one was renamed or \
         removed, and its exemption is now excusing nothing",
        OPEN_READ_HANDLERS.len()
    );
}

#[test]
fn no_handler_opens_its_own_transaction() {
    // The one-write-path rule at the handler layer. A handler that called
    // `pool.begin()` could commit without the seal, and the SQL-statement scan
    // in `services::chronology_subset_write::tests` would not see it — the
    // statements would still be in the right file.
    let mut offenders = Vec::new();
    for file in HANDLER_FILES {
        let source = source_without_comments(file);
        for forbidden in [".begin()", "tx.commit()"] {
            if source.contains(forbidden) {
                offenders.push(format!("{file} contains `{forbidden}`"));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a subset handler owns a transaction; every mutation must be ONE call into \
         services::chronology_subset_write:\n  {}",
        offenders.join("\n  ")
    );
}
