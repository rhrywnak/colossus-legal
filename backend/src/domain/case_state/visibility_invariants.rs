//! Disk/code invariant pinning the `case_state` layering law (Standing Rule 21:
//! when an invariant must hold across many files, assert it by scanning the
//! files).
//!
//! ## What the law is, and why a test guards something the compiler already does
//!
//! `partition.rs` declares three tier sets — the probative set, the topical-only
//! difference, and their union — without `pub`. Those three lists ARE the
//! definition of "connected" for this product, and privacy is what stops a
//! second definition from growing: a module that cannot name `PROBATIVE_EDGE_TYPES`
//! cannot quietly re-derive "probative" from it, so the only route to the answer
//! is [`super::partition::ConnectionTier::edge_types`].
//!
//! The compiler is therefore the primary enforcement, and this file adds nothing
//! to it on any build that currently passes. What it adds is a TRIPWIRE on the
//! change that would dismantle the law, because that change is one character
//! long. Someone needs the probative set in a new module, types `pub` in front
//! of the const, and everything compiles — the law is gone, silently, in a diff
//! line that looks like a nothing. This test turns that one character into a
//! failing build with a message explaining what was just undone.
//!
//! ## What it checks, and the one thing it cannot
//!
//! Two things: that the declarations in `partition.rs` carry no visibility
//! modifier at all, and that no `.rs` file outside `domain/case_state/` so much
//! as spells one of the three names — in code OR in prose. The prose half is
//! deliberate: a doc comment naming a tier set is how a caller starts thinking
//! it may reach for one, and it costs nothing to keep the family's vocabulary
//! inside the family.
//!
//! It does NOT check that a caller went through the accessor for the RIGHT tier
//! — asking `Topical` where the headline wants `Probative` is a correctness bug
//! the type system cannot see either, and it is the query-shape tests in
//! `case_health_repository_tests` that catch it by asserting the generated Cypher.
//!
//! ## Why the scope is tier sets and not edge names
//!
//! The law governs notions of CONNECTEDNESS, not the schema vocabulary. Roughly
//! eight repository modules legitimately interpolate `schema::CORROBORATES` and
//! friends into Cypher because they are walking the graph — proof review, causes
//! of action, element and allegation detail, human facts. Spelling an edge type
//! to traverse it is not defining a tier, and a test that forbade it would be
//! forbidding ordinary query construction. It is the SETS that constitute a
//! definition, so it is the sets this file polices.

use std::fs;
use std::path::{Path, PathBuf};

use super::partition::ConnectionTier;

/// The three declarations that constitute the partition's definition of
/// "connected" and must therefore never leave `partition.rs`.
///
/// Spelled as string literals rather than referenced as items for the obvious
/// reason: the items are private, and a test that could name them would be proof
/// the invariant had already failed. This file lives INSIDE `domain/case_state/`,
/// so [`SCANNED_FAMILY_DIR`] excludes it from the outside-scan by construction
/// and these literals cannot report themselves as violations.
const TIER_SET_NAMES: &[&str] = &[
    "PROBATIVE_EDGE_TYPES",
    "TOPICAL_ONLY_EDGE_TYPES",
    "TOPICAL_EDGE_TYPES",
];

/// The module family permitted to name a tier set, as a path fragment.
///
/// Written with a forward slash and compared against a normalised path so the
/// check does not silently pass on a platform with a different separator — a
/// scan that matches nothing reports a clean tree, which is the worst failure
/// mode a scan has.
const SCANNED_FAMILY_DIR: &str = "src/domain/case_state/";

/// Every `.rs` file under `backend/src/`, sorted for deterministic reporting.
fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).expect("backend/src is readable");
    for entry in entries {
        let path = entry.expect("directory entry is readable").path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// `(repo-relative path, contents)` for every Rust source in the crate.
fn all_sources() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_sources(&root.join("src"), &mut files);
    files.sort();

    files
        .into_iter()
        .map(|path| {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let text = fs::read_to_string(&path).expect("source file is UTF-8");
            (rel, text)
        })
        .collect()
}

/// The tier sets are declared with no visibility modifier whatsoever.
///
/// Checks the whole declaration line rather than searching for `pub const`, so
/// `pub(crate)`, `pub(super)` and `pub(in ...)` are caught by the same assertion.
/// Any of those would open the definition to at least one other module, which is
/// exactly what the law forbids — the permitted audience is `partition.rs` and
/// nothing else.
#[test]
fn edge_class_constants_are_private_to_the_partition() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/domain/case_state/partition.rs");
    let text = fs::read_to_string(&path).expect("partition.rs is readable UTF-8");

    for name in TIER_SET_NAMES {
        // The declaration is the line that both introduces a `const` and spells
        // this name; a doc comment mentioning the name has no `const` on it, and
        // a use site inside `edge_types()` has no `const` either.
        let declaration = text
            .lines()
            .find(|line| line.contains("const ") && line.contains(&format!("{name}:")));

        let Some(line) = declaration else {
            panic!(
                "{name} is no longer declared in partition.rs — if the tier set was \
                 renamed, rename it in TIER_SET_NAMES too, or this scan silently \
                 stops policing it"
            );
        };

        assert!(
            line.trim_start().starts_with("const "),
            "{name} must be declared `const`, with no visibility modifier — found: \
             `{}`. Any `pub`, including `pub(crate)` or `pub(super)`, hands the \
             definition of \"connected\" to another module, and a second definition \
             is the drift this family exists to prevent. Callers take \
             ConnectionTier::edge_types() instead.",
            line.trim()
        );
    }
}

/// No module outside `domain/case_state/` names a tier set — in code or prose.
#[test]
fn no_module_outside_case_state_names_an_edge_class_constant() {
    let offenders: Vec<String> = all_sources()
        .into_iter()
        .filter(|(rel, _)| !rel.contains(SCANNED_FAMILY_DIR))
        .flat_map(|(rel, text)| {
            let mut hits = Vec::new();
            for (index, line) in text.lines().enumerate() {
                for name in TIER_SET_NAMES {
                    if line.contains(name) {
                        hits.push(format!("{}:{} — names {}", rel, index + 1, name));
                    }
                }
            }
            hits
        })
        .collect();

    assert!(
        offenders.is_empty(),
        "the tier sets are private to domain/case_state/partition.rs and define \
         what \"connected\" means; a module that names one is either reading the \
         definition directly (use ConnectionTier::edge_types()) or is about to \
         grow a second copy of it:\n  {}",
        offenders.join("\n  ")
    );
}

/// The scan reads a real corpus, and reads the file the law is about.
///
/// Without this, a broken walker (an unreadable directory swallowed, an
/// extension typo) would return an empty file list and the assertion above would
/// pass vacuously — a green test guarding nothing, which is worse than no test
/// because it reports safety that was never checked.
#[test]
fn the_scan_reads_a_representative_corpus() {
    let sources = all_sources();
    assert!(
        sources.len() > 100,
        "expected the backend's Rust sources; found {} — the walker is probably \
         no longer descending",
        sources.len()
    );

    // The callers this task re-pointed are the ones most likely to regress, so
    // prove they are actually in the scanned set rather than merely absent.
    for expected in [
        "src/repositories/case_health_repository.rs",
        "src/repositories/case_health_builder.rs",
        "src/api/case_health.rs",
    ] {
        assert!(
            sources.iter().any(|(rel, _)| rel == expected),
            "{expected} must be in the scanned corpus; it is a partition caller"
        );
    }

    // And prove the family exclusion is doing work rather than matching nothing:
    // partition.rs itself names all three sets and must be excluded.
    assert!(
        sources
            .iter()
            .any(|(rel, text)| rel.contains(SCANNED_FAMILY_DIR)
                && TIER_SET_NAMES.iter().all(|n| text.contains(n))),
        "no file under {SCANNED_FAMILY_DIR} names the tier sets — the exclusion \
         path is wrong, so the scan is excluding nothing and would not catch a \
         leak either"
    );
}

/// The accessor returns the ratified sets, so re-routed callers are unchanged.
///
/// This is the behavioural half of the move: every caller that used to read a
/// const now calls `edge_types()`, and this asserts the two are the same thing.
/// The tier CONTENTS are pinned inside `partition.rs` (`probative_is_exactly_the_\
/// three_truth_bearing_edges`); what is pinned here is that the public route
/// reaches them intact.
#[test]
fn the_accessor_is_the_only_route_to_the_tier_sets() {
    let probative = ConnectionTier::Probative.edge_types();
    let topical = ConnectionTier::Topical.edge_types();

    assert_eq!(
        probative,
        &["CORROBORATES", "REBUTS", "CHARACTERIZES"],
        "the headline tier reached through the accessor must be the ratified set"
    );
    assert_eq!(
        topical,
        &["CORROBORATES", "REBUTS", "CHARACTERIZES", "ABOUT"]
    );
    assert!(
        !probative.contains(&"ABOUT"),
        "ABOUT must never reach the headline tier through the public accessor"
    );
}
