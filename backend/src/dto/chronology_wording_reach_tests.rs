// =============================================================================
// ⚑ EVERY KEY THE TIMELINE ASKS FOR HAS A FIELD ON THE WIRE
// =============================================================================
//
// The third edge of the three-party agreement, for the chronology surfaces.
// Its practice sibling was written after .407 shipped a backend that booted
// cleanly and a page that rendered BLANK: seven rows were seeded and declared in
// no Rust block, so they had no field, were never serialized, and never reached
// the browser — where the accessor throws by name. The rows were correct in the
// database the whole time.
//
//   · the DATABASE holds rows
//   · the BACKEND declares keys       ← boot refuses if a declared key has no row
//   · the FRONTEND requests strings   ← this test, and nothing else
//
// The boot check proves the store satisfies what the code DECLARES. It is
// structurally incapable of noticing a key the code never declares and the
// browser asks for anyway.
//
// ## Why the accessor is `cw` and not `w`
//
// `practice_wording_reach_tests` scans `frontend/src/pages` for `w("…")` and
// requires every hit to be a PRACTICE field. The timeline's pages live in that
// same directory, so a timeline page calling `w("page_title")` would fail that
// scan for a key that was never practice's to carry. `cw(` is invisible to it —
// the character before the `w` is alphanumeric, which that scanner explicitly
// rejects — and is scanned here instead.
//
// ## Its limit, stated plainly
//
// A key assembled from a variable cannot be seen by reading source. None exist
// on these surfaces today (the card's tag chips read a tag's own label, not a
// wording key), and if that pattern ever appears the answer is a generated
// client, not a cleverer scanner.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::domain::wording_chronology::ChronologyWording;
use crate::dto::chronology_wording::ChronologyWordingDto;

/// The frontend directories whose chronology surfaces read wording.
// STRUCTURAL: repo-internal source paths, not deployment configuration. They
// cannot vary between DEV and PROD, and a moved directory fails this test rather
// than mis-serving a request.
const SURFACE_DIRS: &[&str] = &[
    "../frontend/src/components/timeline",
    "../frontend/src/pages",
    "../frontend/src/components",
];

/// The two accessors these surfaces name a key through.
///
/// `cw(…)` throws by name when a key is missing — right at a page boundary.
/// `cwSafe(…)` degrades to nothing — right inside a list, where one absent
/// marker must not take the whole timeline down. Both are scanned, because both
/// are a request the wire has to satisfy.
///
/// Each is called as `accessor(<the wording object>, "key")`, so the scan below
/// reads past the first argument rather than expecting the literal to follow the
/// opening paren.
const ACCESSORS: &[&str] = &["cw(", "cwSafe("];

/// Source with its `//` comments removed.
///
/// ⚑ Required before any scan of this repository's source. This codebase
/// documents its rules next to its rules, so a scanner searching for a name
/// finds the DOCUMENTATION first. The rule and its instances are stated once in
/// `domain::wording_tests`.
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

/// Every `.tsx`/`.ts` file under the surfaces, tests excluded.
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
            if is_source && !name.contains(".test.") {
                out.push(path);
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Every wording key the chronology surfaces request, with the file asking.
///
/// ## ⚑ MATCHED EXACTLY, closing quote and all
///
/// The practice sibling's first version took THE NEXT STRING ANYWHERE after an
/// opener and reported `type="button"` as a missing key from four files. A false
/// failure is worse than no test: the first person to meet it widens it until it
/// passes, and then it guards nothing. So a match is a complete literal or it is
/// not a match.
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

        for accessor in ACCESSORS {
            for (at, _) in source.match_indices(accessor) {
                // The opener must not be the tail of a longer identifier —
                // `myCw(` is not this accessor.
                let preceded_ok = source[..at]
                    .chars()
                    .next_back()
                    .is_none_or(|c| !(c.is_alphanumeric() || c == '_' || c == '.'));
                if !preceded_ok {
                    continue;
                }

                // Read to the end of THIS call and take the first string
                // literal in it. A `)` before any `"` means the call named no
                // key and is not a wording request at all.
                let rest = &source[at + accessor.len()..];
                let quote = rest.find('"');
                let close = rest.find(')');
                let (Some(quote), Some(close)) = (quote, close) else {
                    continue;
                };
                if close < quote {
                    continue;
                }
                let Some(end) = rest[quote + 1..].find('"') else {
                    continue;
                };
                let key = &rest[quote + 1..quote + 1 + end];
                let shaped = !key.is_empty()
                    && key
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
                if shaped {
                    out.push((key.to_string(), name.clone()));
                }
            }
        }
    }
    out
}

/// The field names the wire object actually carries.
fn wire_fields() -> BTreeSet<String> {
    let dto = ChronologyWordingDto::from(&ChronologyWording::for_test());
    let value = serde_json::to_value(&dto).expect("the mirror serializes");
    value
        .as_object()
        .expect("an object body")
        .keys()
        .cloned()
        .collect()
}

#[test]
fn every_key_the_timeline_asks_for_is_a_field_on_the_wire() {
    let fields = wire_fields();
    let mut missing = Vec::new();
    for (key, file) in requested_keys() {
        if !fields.contains(&key) {
            missing.push(format!("{file} asks for \"{key}\""));
        }
    }
    assert!(
        missing.is_empty(),
        "the timeline requests wording the wire object does not carry — every one \
         of these renders as a thrown error or an empty control:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn the_scan_can_actually_see_the_surfaces_it_claims_to_check() {
    // ⚑ THE VACUITY GUARD. Every assertion above is satisfied by finding
    // nothing, so a moved directory or a renamed accessor would leave this file
    // green while checking no source at all. These two keys are requested by
    // name on the two pages, and a scan that cannot see them is a scan that has
    // stopped working.
    let asked: BTreeSet<String> = requested_keys().into_iter().map(|(key, _)| key).collect();

    for sentinel in [
        "page_title",
        "no_document_label",
        "no_history_label",
        // Phase C's surfaces, one sentinel each: the form, the undo line, the
        // picker and the history mapper. A scan that stopped seeing any of the
        // four new files would be caught by whichever of these it lost.
        "form_title_placeholder",
        "undo_label",
        "picker_capped_template",
        "history_updated_label",
    ] {
        assert!(
            asked.contains(sentinel),
            "the scan did not find {sentinel:?} — it read {} keys from {} files, \
             which means the surfaces moved or the accessor was renamed",
            asked.len(),
            surface_files().len(),
        );
    }
}

/// Keys declared ahead of the screen that will read them.
///
/// EMPTY, and that is the point. Phase B left the other direction reported
/// rather than enforced because the write surfaces were a phase away and half
/// the block had no asker yet. Phase C shipped them, every declared key is now
/// requested by name, and the door closes behind it: a row seeded, mirrored and
/// paid for that no screen speaks is a row that drifts unnoticed forever, which
/// is the .407 defect read backwards.
///
/// Declaring a key one commit before its screen is still allowed — it costs one
/// line here, and that line is a promise with a name on it rather than a
/// silence.
// STRUCTURAL: a list of wording keys, which are join keys between code and rows
// in the same category as a column name. Not deployment configuration.
const DECLARED_AHEAD_OF_THEIR_SCREEN: &[&str] = &[
    // Timeline subsets, T1.2. Sixteen rows seeded and mirrored one commit
    // ahead of the screens that speak them: the Subsets section and its
    // picker are task 2, the scenario attach row and the floating window are
    // task 3, and each is its own branch. Every line here retires as its
    // screen lands, and this list is empty again when task 3 merges — a
    // line left behind after that would be a row nothing says.
    "subsets_section_title",
    "subsets_section_subtitle",
    "subsets_add_button",
    "subsets_carried_by_prefix",
    "subsets_gap_count_template",
    "subsets_removed_event_line",
    "subsets_size_line_template",
    "subsets_picker_hint",
    "subsets_picker_gap_hint",
    "scenario_view_timeline_button",
    "scenario_timeline_row_label",
    "scenario_attach_link",
    "subsets_window_open_timeline",
    "subsets_window_edit",
    "subsets_window_footer_template",
    "subsets_empty_state",
    // Task 2's seven, seeded by 20260830153346 in the commit that declares
    // them and spoken by the very next one — the Subsets section and the
    // picker. They are here for one commit, not one task: the migration is
    // its own commit so the wording lands before the screen that reads it,
    // which is the order that keeps a deploy from ever serving a screen whose
    // words are missing.
    "subsets_event_count_template",
    "subsets_form_add_title",
    "subsets_picked_count_template",
    "subsets_pill_gaps_template",
    "subsets_form_name_label",
    "subsets_form_description_label",
    "subsets_note_placeholder",
];

#[test]
fn no_declared_word_is_left_with_no_asker() {
    let asked: BTreeSet<String> = requested_keys().into_iter().map(|(key, _)| key).collect();
    let unasked: Vec<String> = wire_fields()
        .difference(&asked)
        .filter(|key| !DECLARED_AHEAD_OF_THEIR_SCREEN.contains(&key.as_str()))
        .cloned()
        .collect();
    assert!(
        unasked.is_empty(),
        "these chronology words are declared, seeded and mirrored, and no screen \
         asks for them — either wire them up or retire the rows, or name them in \
         DECLARED_AHEAD_OF_THEIR_SCREEN with the screen that is coming:\n  {}",
        unasked.join("\n  ")
    );
}

#[test]
fn the_ahead_of_their_screen_list_holds_only_real_keys() {
    // A typo in that list would silently excuse nothing, leaving a genuinely
    // unasked key to slip through under a name that does not exist.
    let fields = wire_fields();
    for key in DECLARED_AHEAD_OF_THEIR_SCREEN {
        assert!(
            fields.contains(*key),
            "{key} is excused from the reach check but is not a field on the wire"
        );
    }
}
