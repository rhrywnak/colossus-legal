//! ⚑ THE WRITE GUARD, PROVED BY SCANNING THE HANDLERS.
//!
//! Phase C's rule is that reads stay open and writes are never anonymous. In
//! axum that is a one-character difference at each handler — `user: AuthUser`
//! versus `user: Option<AuthUser>` — and the wrong one COMPILES, serves, and
//! answers 200 to a request with no session. There is no type that would have
//! caught it and no runtime symptom to notice: the write simply lands, stamped
//! with whatever the handler decided to do about a `None`.
//!
//! So the rule is enforced by reading the source. Every `pub async fn` in the
//! two write-handler modules must take the non-optional extractor, except the
//! document PICKER, which is a read and is named here as the one exception.
//!
//! The handlers themselves need an `AppState` (two pools, a graph, a registry),
//! which this project has no test tier for. This is what is reachable, and it is
//! the half that would fail silently.

/// The modules whose handlers must be guarded.
// STRUCTURAL: repo-internal source paths, not deployment configuration. A moved
// file fails this test rather than quietly leaving a handler unscanned.
const WRITE_HANDLER_FILES: &[&str] = &[
    "src/api/timeline_write/events.rs",
    "src/api/timeline_write/links.rs",
];

/// The one handler in those files that is a READ, and why.
///
/// The document picker answers "which documents match what I typed" — seeing
/// which documents exist is not privileged, exactly as the timeline itself is
/// not. The WRITE that uses a choice is guarded; this is the search in front of
/// it. Named here so the exception is one line in a test rather than a judgement
/// somebody makes again per handler.
const OPEN_READ_HANDLERS: &[&str] = &["get_document_choices"];

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

/// `(handler name, the extractor line that follows it)` for every public
/// handler in a scanned file.
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
fn every_chronology_write_handler_demands_an_authenticated_user() {
    // ⚑ THE MUTATION PROOF THE REPORT CITES. Change one handler's `user:
    // AuthUser` to `user: Option<AuthUser>` and this test names it. Without it,
    // an unauthenticated write would be a 200 that nothing in the build noticed.
    let mut anonymous = Vec::new();
    let mut checked = 0usize;

    for file in WRITE_HANDLER_FILES {
        for (name, first_argument) in handlers_in(file) {
            if OPEN_READ_HANDLERS.contains(&name.as_str()) {
                continue;
            }
            checked += 1;
            if !first_argument.starts_with("user: AuthUser") {
                anonymous.push(format!("{file}::{name} takes `{first_argument}`"));
            }
        }
    }

    assert!(
        anonymous.is_empty(),
        "a chronology write handler does not demand an authenticated user — an \
         anonymous request would be stamped and written:\n  {}",
        anonymous.join("\n  ")
    );
    // Anti-vacuity: the assertion above passes trivially if the scan found no
    // handlers at all, which is what a moved file or a re-formatted signature
    // would look like.
    assert!(
        checked >= 8,
        "the scan found only {checked} guarded handlers — Phase C ships eight \
         mutating endpoints, so the scanner has stopped seeing them"
    );
}

#[test]
fn the_named_open_read_really_is_a_read() {
    // The exception list is only safe if the thing on it cannot write. This
    // proves the picker's handler contains no write, rather than trusting its
    // name.
    let source = source_without_comments("src/api/timeline_write/links.rs");
    let at = source
        .find("pub async fn get_document_choices")
        .expect("the picker handler is in that file");
    let body = &source[at..];
    let end = body.find("\n}\n").unwrap_or(body.len());
    let body = &body[..end];

    for forbidden in [
        "seal_and_commit",
        "insert_",
        "delete_",
        "update_",
        "open_write",
    ] {
        assert!(
            !body.contains(forbidden),
            "the picker is on the open-read exception list but its body contains \
             `{forbidden}` — it is not a read any more"
        );
    }
}

#[test]
fn every_write_handler_opens_and_seals_through_the_guard() {
    // The other two halves of the guard: the acting user is stamped through
    // `open_write`, and the transaction is closed by `seal_and_commit`, which is
    // the only committer. A handler that did its own thing would still compile.
    for file in WRITE_HANDLER_FILES {
        let source = source_without_comments(file);
        assert!(
            source.contains("open_write(&user)"),
            "{file} never stamps the acting user"
        );
        assert!(
            source.contains("seal_and_commit("),
            "{file} never seals a write, so it writes no history"
        );
        assert!(
            !source.contains(".commit()"),
            "{file} commits a transaction itself, which is a write with no history row"
        );
    }
}

#[test]
fn the_scanner_can_see_the_handlers_it_claims_to_check() {
    // ⚑ THE VACUITY GUARD, on the scanner itself. Both assertions above are
    // satisfiable by finding nothing.
    let names: Vec<String> = WRITE_HANDLER_FILES
        .iter()
        .flat_map(|file| handlers_in(file))
        .map(|(name, _)| name)
        .collect();
    for sentinel in [
        "post_event",
        "put_event",
        "delete_event",
        "post_undelete",
        "post_link",
        "delete_event_link",
        "post_note",
        "delete_note",
        "get_document_choices",
    ] {
        assert!(
            names.contains(&sentinel.to_string()),
            "the scan did not find {sentinel} — it read {names:?}"
        );
    }
}

#[test]
fn stripping_comments_hides_a_documented_extractor_from_the_scan() {
    // This module's own header says `Option<AuthUser>` twice, in prose. A
    // scanner that read comments would report the header as a defect.
    let source = source_without_comments("src/api/timeline_write/events.rs");
    assert!(
        !source.contains("Option<AuthUser>"),
        "the stripper is not removing comments; the scan is reading documentation"
    );
}
