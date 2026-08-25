// Tests for `domain::case_phase`.
//
// Two properties matter here and neither is about Rust: the slugs must be the
// timeline's slugs, and this module must never grow a label.

use super::*;

/// THE ONE THAT MATTERS. These four strings are `timeline.json`'s phase ids, and
/// a document tagged with anything else would never meet its phase.
///
/// Pinned as literals on purpose. A test that read the slugs back from
/// `CasePhase::slug()` would pass no matter what they were changed to; this one
/// fails if anybody edits them, which is the point — they are a contract with a
/// data file the compiler cannot see.
#[test]
fn the_slugs_are_the_timelines_slugs() {
    assert_eq!(CasePhase::Estate.slug(), "estate");
    assert_eq!(CasePhase::Probate.slug(), "probate");
    assert_eq!(CasePhase::Appeals.slug(), "appeals");
    assert_eq!(CasePhase::CivilLawsuit.slug(), "civil_lawsuit");
}

/// The vocabulary is the case's chronology, in order, with nothing extra.
#[test]
fn every_phase_is_listed_once_in_chronological_order() {
    let slugs: Vec<&str> = ALL_CASE_PHASES.iter().map(|p| p.slug()).collect();
    assert_eq!(slugs, vec!["estate", "probate", "appeals", "civil_lawsuit"]);
    assert_eq!(
        ALL_CASE_PHASES.len(),
        4,
        "a phase was added or removed without bumping CASE_PHASE_LOOKUP_V",
    );
}

#[test]
fn a_slug_round_trips_through_parse_and_back() {
    for phase in ALL_CASE_PHASES {
        assert_eq!(CasePhase::from_slug(phase.slug()), Some(*phase));
    }
}

/// A typo must not silently become the first phase.
#[test]
fn an_unknown_slug_is_refused_rather_than_defaulted() {
    for bad in [
        "appeal",
        "COA",
        "Estate",
        "coa",
        "complaint",
        "probate_court",
    ] {
        assert_eq!(CasePhase::from_slug(bad), None, "{bad:?} should not parse");
    }
}

/// The display labels must never be parseable as slugs — if they were, a
/// frontend that posted a label instead of a slug would silently succeed and
/// store the wrong vocabulary in the column.
#[test]
fn the_display_labels_are_not_accepted_as_slugs() {
    for label in ["PRE-PROBATE", "PROBATE", "COA", "COMPLAINT"] {
        assert_eq!(
            CasePhase::from_slug(label),
            None,
            "{label:?} is a display label from the phases table, not a stored slug",
        );
    }
}

// ── validate ────────────────────────────────────────────────────────────

#[test]
fn a_valid_slug_validates_to_its_phase() {
    assert_eq!(validate(Some("probate")), Ok(Some(CasePhase::Probate)));
    assert_eq!(
        validate(Some("civil_lawsuit")),
        Ok(Some(CasePhase::CivilLawsuit)),
    );
}

/// Absence is VALID — the field is never required. This is the difference from
/// `date_precision::validate`, which refuses a silent blank.
#[test]
fn absence_is_accepted_because_a_phase_is_never_required() {
    assert_eq!(validate(None), Ok(None));
}

/// A `<select>` with no selection posts `""`. Refusing it would make "clear this
/// field" impossible from the only UI that writes it.
#[test]
fn an_empty_or_blank_string_clears_the_phase_rather_than_failing() {
    assert_eq!(validate(Some("")), Ok(None));
    assert_eq!(validate(Some("   ")), Ok(None));
    assert_eq!(validate(Some("\t\n")), Ok(None));
}

/// Surrounding whitespace is trimmed rather than rejected — it is a transport
/// artefact, not an answer.
#[test]
fn surrounding_whitespace_does_not_change_the_answer() {
    assert_eq!(validate(Some("  appeals  ")), Ok(Some(CasePhase::Appeals)));
}

/// The refusal names what was sent AND what would have been accepted; an
/// operator should not have to read the source to fix their request.
#[test]
fn an_unknown_phase_is_refused_with_a_message_naming_the_valid_slugs() {
    let err = validate(Some("coa")).expect_err("'coa' is a label, not a slug");
    let CasePhaseError::Unknown { supplied, valid } = &err;
    assert_eq!(supplied, "coa");
    for slug in ["estate", "probate", "appeals", "civil_lawsuit"] {
        assert!(
            valid.contains(slug),
            "the message must name {slug}: {valid}"
        );
    }
    assert!(
        err.to_string().contains("coa"),
        "the message must quote what was actually sent: {err}",
    );
}

// ── The contract with timeline.json (Standing Rule 21) ──────────────────
//
// The slugs above are a contract with a file the compiler cannot see. Roman's
// ruling put the display labels in that file and ONLY there, so the coupling is
// now load-bearing in both directions: if the ids drift, a document's phase
// stops resolving to a label and the Documents column renders blank.

/// Where the case's phases actually live, since Phase B retired the JSON.
///
/// `chronology_phases` is seeded by this migration and served by the backend;
/// the file this used to read is gone (ruling R15). Relative to the crate root,
/// because `CARGO_MANIFEST_DIR` is the only path a test can trust.
// STRUCTURAL: a repo-internal pointer to one immutable, version-controlled
// migration. Identical in every environment; nothing here varies by deployment.
const PHASES_MIGRATION: &str = "pipeline_migrations/20260825105447_chronology_tables.sql";

fn phases_migration() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(PHASES_MIGRATION)
}

/// The phase ids and labels the migration seeds, in the order it seeds them.
///
/// Parses the one `INSERT INTO chronology_phases … VALUES` block. Deliberately
/// crude, like every sibling migration reader in this repo: the rows have a
/// fixed shape, and a shape that stops matching returns nothing — which the
/// vacuity guard below turns into a failure rather than a silent pass.
fn seeded_phases() -> Vec<(String, String)> {
    let path = phases_migration();
    let body = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));

    // Comments first: this codebase documents its rules next to its rules, and a
    // header quoting a row would otherwise be parsed as one.
    let sql: String = body
        .lines()
        .map(|line| match line.find("--") {
            Some(at) => &line[..at],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n");

    let Some(at) = sql.find("INSERT INTO chronology_phases") else {
        return Vec::new();
    };
    let block = &sql[at..];
    let end = block.find(';').unwrap_or(block.len());

    let mut out = Vec::new();
    for row in block[..end].split("\n    (").skip(1) {
        let mut literals = row.split('\'').skip(1).step_by(2);
        let (Some(id), Some(label)) = (literals.next(), literals.next()) else {
            continue;
        };
        out.push((id.to_string(), label.to_string()));
    }
    out
}

/// THE CONTRACT. Every phase this enum defines exists in `chronology_phases`
/// under the same id, in the same order, and there are no others.
///
/// A test that only checked "every enum slug appears somewhere" would pass while
/// the table grew a fifth phase no document could ever be tagged with. Order is
/// asserted too, because both the UI's dropdown and the timeline render in it.
#[test]
fn the_enum_and_the_phases_table_define_the_same_four_phases() {
    let from_migration: Vec<String> = seeded_phases().into_iter().map(|(id, _)| id).collect();
    let from_code: Vec<String> = ALL_CASE_PHASES
        .iter()
        .map(|p| p.slug().to_string())
        .collect();

    assert_eq!(
        from_code, from_migration,
        "domain::case_phase and the chronology_phases seed disagree about the \
         case's phases. They are one vocabulary: fix whichever is wrong, and \
         remember the labels live in the table while the slugs live in both.",
    );
}

/// Every phase carries a non-empty label, because the frontend renders the label
/// and a blank one would silently produce an empty cell in the Documents table.
///
/// The label TEXT is deliberately not asserted here: Roman renames these at will
/// and a test pinning "PRE-PROBATE" would turn a data edit back into a code
/// change, which is what moving them out of code was for.
#[test]
fn every_seeded_phase_carries_a_usable_label() {
    for (id, label) in seeded_phases() {
        assert!(!id.trim().is_empty(), "a phase has a blank id");
        assert!(
            !label.trim().is_empty(),
            "phase {id:?} has no label — the Documents column would render blank",
        );
    }
}

/// Guard against the scan silently passing because it read nothing.
///
/// ⚑ This is the half that matters. The two tests above are both satisfied by an
/// empty list — `assert_eq!(vec![], vec![])` is a pass — so if the migration
/// moves or its INSERT changes shape, they would go green while checking
/// nothing at all. This says how many rows the reader must actually have found.
#[test]
fn the_scan_can_actually_see_the_rows_it_claims_to_check() {
    let path = phases_migration();
    assert!(
        path.exists(),
        "the chronology phases migration is not at {}",
        path.display()
    );
    assert_eq!(
        seeded_phases().len(),
        ALL_CASE_PHASES.len(),
        "the reader found {} phase rows in {} — the INSERT's shape has changed \
         and the two tests above are now passing vacuously",
        seeded_phases().len(),
        path.display(),
    );
}
