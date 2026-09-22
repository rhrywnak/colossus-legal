// =============================================================================
// ⚑ EVERY KEY THE FRONTEND ASKS FOR HAS A FIELD ON THE WIRE
// =============================================================================
//
// Written after .407 shipped a backend that booted cleanly and a practice page
// that rendered BLANK.
//
// ## What went wrong, and why nothing caught it
//
// Seven wording rows were seeded by a migration and DECLARED IN NO RUST BLOCK.
// The wire object is built field-by-field from those blocks, so a key no block
// declares has no field, is not serialized, and never reaches the browser —
// where `wordingOf` throws by name. The rows were correct in the database the
// whole time.
//
// THREE PARTIES, AND EVERY GUARD CHECKED ONLY TWO:
//
//   · the DATABASE holds rows
//   · the BACKEND declares keys       ← boot refuses if a declared key has no row
//   · the FRONTEND requests strings   ← nothing checked this at all
//
// The boot check proves the store satisfies what the code DECLARES. It is
// structurally incapable of noticing a key the code never declares and the
// browser asks for anyway. "All 235 declared keys resolve" was true, proved
// against the live database, and irrelevant to this failure.
//
// This test closes the third edge. Frontend↔backend here, backend↔database at
// boot: the chain is then whole, transitively.
//
// ## Why a source scan, and its limit
//
// There is no DOM tier in this project and no generated client, so the only
// place the two vocabularies meet is on disk. This reads the practice surfaces
// for `w("…")` / `wordingOf(…, "…")` and requires each name to be a field of
// `PracticeWordingDto`. It cannot see a key built at runtime from a variable.
//
// ## ⚑ AND FOUR SUCH KEYS DO EXIST — I claimed none did, and was wrong
//
// `PracticeReveal.tsx` calls `w(key)` over a `CHECKS` array, so
// `check_only_asked`, `check_accepted_premise`, `check_explained_unasked` and
// `check_guessed` are invisible here. All four ARE fields on the mirror today,
// so nothing is uncovered — but the blind spot is real, it predates this test,
// and the component itself is the RETIRED sitting screen. Corrected after the
// architecture gate caught the claim; measured, not assumed.
//
// A key assembled from a variable cannot be checked by reading source. If that
// pattern spreads beyond the retired screen, this test stops being sufficient
// and the answer is a generated client, not a cleverer scanner.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The frontend directories whose practice surfaces read wording.
// STRUCTURAL: repo-internal source paths, not deployment configuration. They
// cannot vary between DEV and PROD, and a moved directory fails this test rather
// than mis-serving a request.
const SURFACE_DIRS: &[&str] = &[
    "../frontend/src/components/practice",
    // CC_TASK_CHAT_ENGINE_v1: the discussion panel's own folder. The scan does not
    // recurse, so a subfolder is invisible to it until it is named here.
    "../frontend/src/components/practice/chat",
    "../frontend/src/pages",
];

/// Source with its `//` comments removed.
///
/// ⚑ Required before any scan of this repository's source. This codebase
/// documents its rules next to its rules, so a scanner searching for a name
/// finds the DOCUMENTATION first — an apostrophe inside a comment banner cost an
/// hour tonight. The rule, and the five instances that produced it, are stated
/// once in `domain::wording_tests` above `seeded_value_in`.
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

/// Files under a scanned directory whose words are NOT the practice vocabulary.
///
/// ## Why a skip list rather than one union of every mirror
///
/// `src/pages` holds one page that is not a practice surface: "For you", whose
/// twenty-five strings are their own block and their own mirror
/// (CC_TASK_FOR_YOU_v1). Unioning its fields into the set this scanner checks
/// against would make a PRACTICE page asking for `group_today` pass — a guard
/// that checks a part of the truth and reports on the whole, which is the exact
/// failure the header above describes arriving from the other direction.
///
/// So that page is skipped here and scanned by `for_you_wording_reach_tests`,
/// which checks it against its own mirror. A file added to this list without a
/// scanner of its own is a surface nothing guards — do not lengthen it lightly.
// STRUCTURAL: repo-internal source file names, exactly as SURFACE_DIRS above.
const NOT_A_PRACTICE_SURFACE: &[&str] = &["ForYouPage.tsx"];

/// Every `.tsx`/`.ts` file under the practice surfaces, tests excluded.
fn surface_files() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut out = Vec::new();
    for dir in SURFACE_DIRS {
        let Ok(entries) = std::fs::read_dir(root.join(dir)) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let is_source = name.ends_with(".tsx") || name.ends_with(".ts");
            if is_source && !name.contains(".test.") && !NOT_A_PRACTICE_SURFACE.contains(&name) {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// Every wording key the practice surfaces request, with the file asking.
///
/// ## ⚑ MATCHED EXACTLY, closing paren and all
///
/// The first version searched for the opener and then took THE NEXT STRING
/// ANYWHERE after it, which reported `type="button"` as a missing wording key
/// from four files at once. A false failure is the one thing worse than no test:
/// the first person to meet it widens it until it passes, and then it guards
/// nothing. So a match is the whole call — `w("some_key")` — or it is not a
/// match.
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
            // The call must be `w(` or `wordingOf(…, ` — read backwards from the
            // paren so the opener cannot be part of a longer identifier.
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

        // `wordingOf(wording, "key")` — the same call spelled out.
        let mut from = 0;
        while let Some(at) = source[from..].find("wordingOf(") {
            let start = from + at + "wordingOf(".len();
            if let Some(comma) = source[start..].find(',') {
                let after = start + comma + 1;
                let trimmed = source[after..].trim_start();
                if let Some(rest) = trimmed.strip_prefix('"') {
                    if let Some(end) = rest.find('"') {
                        let key = &rest[..end];
                        if rest[end + 1..].starts_with(')')
                            && !key.is_empty()
                            && key
                                .chars()
                                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                        {
                            out.push((key.to_string(), name.clone()));
                        }
                    }
                }
            }
            from = start;
        }
    }
    out
}

/// The files the served object's `String` fields are declared across.
///
/// ## ⚑ TWO FILES SINCE 2026-09-19, AND THE SECOND ONE MATTERS
///
/// The mirror was one struct in one file when this scanner was written, and it
/// read that file. CC_TASK_REVIEW_PAGE_v1 moved ten fields into a
/// `#[serde(flatten)]`'d sibling to keep both files under Rule 17 — and a
/// scanner still reading one file would have reported those ten keys as
/// missing the moment the review page asked for one. Which is the same shape of
/// failure this whole test exists to catch, arriving from the other direction:
/// a guard that checks a PART of the truth and reports on the WHOLE.
///
/// So the list is explicit, and adding a flattened sibling without adding it
/// here fails loudly rather than quietly — see the sentinel below.
// STRUCTURAL: repo-internal source paths, exactly as SURFACE_DIRS above.
const MIRROR_FILES: &[&str] = &[
    "src/dto/practice_wording.rs",
    "src/dto/practice_wording_review.rs",
    // REVIEW_COUNTS_HONEST: the review bar's eight strings, split out under
    // Rule 17 the same way the review page's ten were. A sibling added here and
    // not to this list would not be a false pass — its fields would simply be
    // missing from the scan, and every key the browser asks for out of it would
    // fail as "declared nowhere". Loud, not silent, which is why the list is a
    // list and not a directory walk.
    "src/dto/practice_wording_deck_review.rs",
    // CHAT_ENGINE_v1: the discussion panel's forty-two strings, flattened the same way.
    "src/dto/practice_wording_chat.rs",
];

/// Every `String` field the served wording object carries, across all three structs.
///
/// `#[serde(flatten)]` makes them serialize as ONE object, so from the
/// browser's side there is no seam here at all — which is exactly why the scan
/// must not have one either.
fn mirror_fields() -> BTreeSet<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    MIRROR_FILES
        .iter()
        .flat_map(|file| {
            let source = std::fs::read_to_string(root.join(file))
                .unwrap_or_else(|cause| panic!("{file} is not on disk: {cause}"));
            without_comments(&source)
                .lines()
                .filter_map(|line| {
                    let line = line.trim();
                    let rest = line.strip_prefix("pub ")?;
                    let (name, tail) = rest.split_once(':')?;
                    tail.trim().starts_with("String").then(|| name.to_string())
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn every_key_the_practice_surfaces_request_has_a_field_on_the_wire() {
    let fields = mirror_fields();
    let requested = requested_keys();

    // ⚑ ANTI-VACUITY BY SENTINEL, NOT BY COUNT.
    //
    // A scan reading nothing reports nothing missing, forever — and a THRESHOLD
    // cannot tell that apart from a scan reading half. If `components/practice`
    // were renamed, `pages` alone could still clear a `> 50` bar and every key in
    // the lost directory would escape in silence. That is the same failure that
    // bit twice earlier today: a count satisfied by the wrong things.
    //
    // So: one named key per surface directory, each requested ONLY from that
    // directory. Their absence proves a directory was lost, which a number
    // cannot.
    for (sentinel, from) in [
        ("practice_mode_label", "components/practice"),
        ("print_answers_page_title", "pages"),
        ("chat_switcher_footer", "components/practice/chat"),
    ] {
        assert!(
            requested.iter().any(|(key, _)| key == sentinel),
            "`{sentinel}` was not found — the scan has stopped reading {from}, \
             and every key in it is now escaping silently"
        );
    }
    // One sentinel PER MIRROR FILE, for the reason the surface sentinels above
    // exist: a scan that lost one of the two structs would still find hundreds
    // of fields and report nothing missing.
    for (sentinel, from) in [
        ("row_delete_label", "practice_wording.rs"),
        ("review_title", "practice_wording_review.rs"),
        ("chat_open_hint", "practice_wording_chat.rs"),
    ] {
        assert!(
            fields.contains(sentinel),
            "the mirror scan found no `{sentinel}` — it has stopped reading \
             {from}, and every missing key from it would now look present"
        );
    }

    let mut missing: Vec<String> = requested
        .iter()
        .filter(|(key, _)| !fields.contains(key))
        .map(|(key, file)| format!("{key}  ← {file}"))
        .collect();
    missing.sort();
    missing.dedup();

    assert!(
        missing.is_empty(),
        "these keys are requested by the browser and carried by NO field on the \
         wire, so `wordingOf` throws and the page renders blank — which is what \
         .407 shipped:\n  {}",
        missing.join("\n  ")
    );
}
