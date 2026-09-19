// =============================================================================
// THE BUNDLING IS COMPLETE, AND NO KEY IS CLAIMED TWICE
// =============================================================================
//
// `settings_map` decides which block of the Settings page every stored parameter
// appears in. Two things can go wrong with it, and neither one breaks a build:
//
//   1. A key is claimed by two blocks. The `HashMap` insert silently keeps the
//      last, and the row appears under a heading that is not where anyone looks
//      for it.
//   2. A NEW block is declared somewhere in `domain/` and nobody adds it here.
//      Its keys then resolve to "Undeclared — read by nothing" — a live, working
//      block of wording labelled dead on the one page that can change it.
//
// Both are caught below, and the second is caught by scanning the DISK rather
// than by a second list in this file. A second list is the defect it would be
// trying to prevent.

use super::*;

/// Every `*_KEYS` declaration this rule covers must be bundled by `settings_map`.
// STRUCTURAL: repo-internal source paths, the same shape as `ROOTS` in
// `services::no_stale_model_literal_tests`.
const SCANNED_DIRS: &[&str] = &["src/domain", "src/services"];

/// One key is claimed by exactly one block.
///
/// ## Why the assertion is `sum == distinct` and not a literal number
///
/// The house rule after the v2.1.14 merge is that an exact count catches what a
/// `>=` lets through — two branches each taking a running total from 49 to 50,
/// and git merging them to 50. That lesson holds, and this IS the exact form of
/// it: if any key were claimed twice, the blocks' lengths would add up to more
/// than the number of distinct keys, and the equality would fail by exactly the
/// number of collisions. Writing `853` instead would assert the same thing plus
/// a number that churns every time a line of wording is added — and a literal
/// that has to be edited weekly is a literal people edit without reading.
#[test]
fn every_declared_key_is_claimed_by_exactly_one_block() {
    let declared: usize = AREAS
        .iter()
        .flat_map(|area| area.blocks.iter())
        .map(|block| block.keys.len())
        .sum();
    let distinct = declared_keys().count();

    assert_eq!(
        declared,
        distinct,
        "{} key(s) are claimed by more than one block — the page would show each \
         of them under only the last block that claimed it, and the earlier \
         block's heading would be silently short",
        declared - distinct
    );

    // Anti-vacuity. NOT a substitute for the equality above — `0 == 0` would
    // satisfy it happily, and a bundling that had lost every key would pass a
    // test that only compared two sums to each other.
    assert!(
        distinct > 800,
        "only {distinct} keys are bundled; the store holds 860-odd, so whole \
         blocks have gone missing from AREAS"
    );
}

/// No two blocks, and no two areas, share an id.
///
/// The page routes by these ids. Two blocks called the same thing would make one
/// of them unreachable from the rail with nothing in the build to say so.
#[test]
fn every_area_and_block_id_is_unique() {
    let mut area_ids: Vec<&str> = AREAS.iter().map(|a| a.id).collect();
    area_ids.push(UNDECLARED_AREA_ID);
    let mut sorted = area_ids.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), area_ids.len(), "two areas share an id");

    let mut block_ids: Vec<&str> = AREAS
        .iter()
        .flat_map(|a| a.blocks.iter())
        .map(|b| b.id)
        .collect();
    block_ids.push(UNDECLARED_BLOCK_ID);
    let mut sorted = block_ids.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), block_ids.len(), "two blocks share an id");
}

/// Nothing is bundled under an empty label, and no block points at an empty list.
///
/// A block with no keys is a heading a human can open to find nothing, which
/// reads as a bug in the page rather than as what it is — a bundling mistake.
#[test]
fn no_block_is_empty_and_every_label_says_something() {
    for area in AREAS {
        assert!(
            !area.label.trim().is_empty(),
            "area {} has no label",
            area.id
        );
        assert!(!area.blocks.is_empty(), "area {} has no blocks", area.id);
        for block in area.blocks {
            assert!(
                !block.label.trim().is_empty(),
                "block {} has no label",
                block.id
            );
            assert!(
                !block.keys.is_empty(),
                "block {} points at an empty key list",
                block.id
            );
        }
    }
}

/// A stored key no block claims reaches the page anyway, labelled dead.
///
/// This is the ruled behaviour of 2026-09-19: the thirteen `practice_notes_*`
/// rows and `practice_row_review_link` are stored, declared by nothing and read
/// by nothing. They are SHOWN, in their own group at the bottom, rather than
/// dropped — a row a human cannot see is a row nobody ever retires.
#[test]
fn a_key_no_block_claims_lands_in_the_undeclared_group() {
    let placed = locate("practice_notes_save_label");
    assert_eq!(placed.area_id, UNDECLARED_AREA_ID);
    assert_eq!(placed.block_id, UNDECLARED_BLOCK_ID);
}

/// A key a block DOES claim lands there, not in the undeclared group.
///
/// The other half of the pair above: without it, a `locate` that returned the
/// undeclared placement for everything would pass that test perfectly.
#[test]
fn a_claimed_key_lands_in_its_own_block() {
    let placed = locate("talking_points_cap");
    assert_eq!(placed.area_id, "core");
    assert_eq!(placed.block_id, "core");

    let wording = locate("practice_review_title");
    assert_eq!(wording.area_id, "practice");
    assert_eq!(wording.block_id, "practice_review");
}

/// Every `*_KEYS` list on disk is bundled by `settings_map`.
///
/// ## Rule 21: the invariant spans files, so the test reads files
///
/// The failure this catches is an omission, and an omission is invisible to a
/// test that walks what IS in `AREAS`. So the test walks the source instead,
/// finds every settings key list the repo declares, and requires each one to be
/// named in `settings_map.rs`.
///
/// The pattern is deliberately strict — `_KEYS: &[&str] = &[` — so that the
/// fixed-size property arrays in `embedding_repository` (`[&str; 11]`, nothing
/// to do with the settings store) and `practice_read_parse::FIELD_KEYS` (a
/// single `&str`) are not swept in.
#[test]
fn every_key_list_on_disk_is_bundled_here() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let map_source = std::fs::read_to_string(root.join("src/services/settings_map.rs"))
        .expect("settings_map.rs must be readable — this test scans it");

    let mut found = Vec::new();
    for dir in SCANNED_DIRS {
        collect_key_lists(&root.join(dir), &mut found);
    }

    // Anti-vacuity: a scanner that matched nothing would otherwise report
    // "every list is bundled" about the empty set.
    assert!(
        found.len() >= 25,
        "the scanner found only {} key lists; twenty-five are declared, so it \
         has stopped matching some of what it is supposed to police. If a block \
         was deliberately retired, drop this floor in the same commit — that is \
         the moment the bundling above most needs re-checking.",
        found.len()
    );

    let missing: Vec<&String> = found
        .iter()
        .filter(|name| !mentions(&map_source, name))
        .collect();
    assert!(
        missing.is_empty(),
        "these key lists are declared but no block in settings_map bundles them, \
         so every one of their settings shows on the page as dead: {missing:?}"
    );
}

/// Does `source` name `ident` as a WHOLE identifier?
///
/// ## Why a plain `contains` is wrong here
///
/// `WORDING_KEYS` is a substring of `PRACTICE_WORDING_KEYS`, `MATRIX_WORDING_KEYS`
/// and nine others. A `contains` check would therefore report the shared-wording
/// block as bundled for as long as ANY of its longer namesakes was — which is to
/// say, always. The one list most likely to be forgotten would have been the one
/// list this test could never fail on.
fn mentions(source: &str, ident: &str) -> bool {
    let is_ident = |c: char| c.is_alphanumeric() || c == '_';
    source.match_indices(ident).any(|(at, _)| {
        let before_ok = at == 0 || !source[..at].chars().next_back().is_some_and(is_ident);
        let after = at + ident.len();
        let after_ok = !source[after..].chars().next().is_some_and(is_ident);
        before_ok && after_ok
    })
}

/// Collect `pub const NAME_KEYS: &[&str] = &[` declarations under `dir`.
fn collect_key_lists(dir: &std::path::Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            collect_key_lists(&path, out);
            continue;
        }
        // Test modules declare fixtures that look like key lists and are not.
        if !name.ends_with(".rs") || name.ends_with("_tests.rs") {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        for line in source.lines() {
            let line = line.trim();
            if !line.contains("_KEYS: &[&str] = &[") {
                continue;
            }
            if let Some(name) = line
                .split_whitespace()
                .find(|word| word.ends_with("_KEYS:"))
                .map(|word| word.trim_end_matches(':').to_string())
            {
                out.push(name);
            }
        }
    }
}
