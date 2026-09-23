// =============================================================================
// ⚑ EVERY KEY THE "FOR YOU" PAGE ASKS FOR HAS A FIELD ON THE WIRE
// =============================================================================
//
// The sibling of `practice_wording_reach_tests`, for this page's own
// vocabulary, and written for the failure that file records: .407 shipped a
// backend that booted cleanly and a practice page that rendered BLANK, because
// seven rows existed in the database, were declared in no Rust block, and were
// therefore never serialized — while the browser asked for them by name.
//
// THREE PARTIES, AND THE BOOT CHECK ONLY EVER SEES TWO:
//
//   · the DATABASE holds rows
//   · the BACKEND declares keys       ← boot refuses if a declared key has no row
//   · the FRONTEND requests strings   ← only a source scan can see this edge
//
// ## Why a second scanner instead of widening the first
//
// The practice scanner unions the fields of every practice mirror and checks
// every key any practice surface asks for against that union. Adding this
// page's mirror to that list would make a practice page asking for
// `group_today` pass — a guard that checks a PART of the truth and reports on
// the whole, which is the shape of failure that file itself warns about. Two
// scanners, two vocabularies, each precise.
//
// ## Its limit, stated
//
// It matches the literal call shape `w("key")` only. A key computed inside the
// parens is invisible to it, so this page writes every wording key as a
// literal — see the page's own header.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The frontend files that speak this page's words.
// STRUCTURAL: repo-internal source paths, not deployment configuration. A moved
// file fails this test rather than mis-serving a request.
const SURFACES: &[&str] = &[
    "../frontend/src/pages/ForYouPage.tsx",
    "../frontend/src/components/forYou",
];

/// The mirror whose `String` fields are what the browser actually receives.
const MIRROR: &str = "src/dto/for_you_wording.rs";

/// Source with its `//` comments removed.
///
/// ⚑ Required before any scan of this repository's source: this codebase
/// documents its rules next to its rules, so a scanner searching for a call
/// finds the DOCUMENTATION first.
fn without_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| match line.find("//") {
            Some(at) => &line[..at],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every `.ts`/`.tsx` file this page's words can be spoken from.
fn surface_files() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut out = Vec::new();
    for entry in SURFACES {
        let path = root.join(entry);
        if path.is_file() {
            out.push(path);
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&path) else {
            continue;
        };
        for found in entries.flatten() {
            let found = found.path();
            let name = found.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if (name.ends_with(".tsx") || name.ends_with(".ts")) && !name.contains(".test.") {
                out.push(found);
            }
        }
    }
    out.sort();
    out
}

/// Every wording key these surfaces request, with the file asking.
///
/// ⚑ MATCHED EXACTLY, closing paren and all — the lesson the practice scanner
/// records: a looser match reported `type="button"` as a missing wording key,
/// and a false accusation is worse than no test, because the first person to
/// meet one widens the guard until it passes.
fn requested_keys() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for path in surface_files() {
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        let source = without_comments(&raw);
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();
        for (at, _) in source.match_indices("(\"") {
            let before = &source[..at];
            let opener_ok = before.ends_with('w')
                && !before[..before.len() - 1]
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.');
            let key_start = at + 2;
            let Some(rel) = source[key_start..].find('"') else {
                continue;
            };
            let key = &source[key_start..key_start + rel];
            let closes = source[key_start + rel + 1..].starts_with(')');
            let shaped = !key.is_empty()
                && key
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
            if opener_ok && closes && shaped {
                out.push((key.to_string(), name.clone()));
            }
        }
    }
    out
}

/// Every `String` field the served object declares.
fn served_fields() -> BTreeSet<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(MIRROR);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|cause| panic!("{MIRROR} is on disk: {cause}"));
    without_comments(&source)
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let rest = line.strip_prefix("pub ")?;
            let name = rest.strip_suffix(": String,")?;
            Some(name.to_string())
        })
        .collect()
}

/// THE ASSERTION: nothing the page asks for is missing from the wire.
#[test]
fn every_key_the_page_asks_for_is_served() {
    let served = served_fields();
    for (key, file) in requested_keys() {
        assert!(
            served.contains(&key),
            "{file} asks for the wording key `{key}`, which is not a field of \
             ForYouWordingDto — it would not be serialized, and the page would \
             throw by name in the browser. Declare it in domain::wording_for_you, \
             seed it in a migration, and mirror it here.\nServed: {served:?}"
        );
    }
}

/// ANTI-VACUITY: the scan can actually see the page it claims to read.
///
/// Every assertion above is satisfied by finding nothing, so a renamed
/// directory or a page that stopped calling `w(...)` would leave a green test
/// guarding an empty list. These two floors are what make the green mean
/// something.
#[test]
fn the_scan_can_see_the_page_it_reads() {
    let files = surface_files();
    assert!(
        files.len() >= 2,
        "only {} For You surface file(s) found — the directory list has gone \
         stale: {files:?}",
        files.len()
    );
    let keys = requested_keys();
    assert!(
        keys.len() >= 5,
        "only {} wording call(s) were parsed out of the page — the scan has gone \
         blind, or the page stopped reading its words from the store",
        keys.len()
    );
    assert!(
        served_fields().len() >= 20,
        "the mirror parse found too few fields — its shape changed"
    );
}
