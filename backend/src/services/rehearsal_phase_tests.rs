// Tests for `services::rehearsal_phase`.
//
// The rule these pin is FORUM WINS, and the reason it needs pinning rather than
// commenting is that the failure is invisible: a date-decided phase is a
// perfectly plausible chip, and the one document it gets wrong is the appeal —
// on the page a witness preps from, in front of opposing counsel.

use super::*;
use crate::domain::settings::Settings;

/// The settings snapshot with the seeded phase rows.
fn settings() -> Settings {
    Settings::for_test()
}

#[test]
fn the_four_phases_come_from_the_row_in_order() {
    assert_eq!(
        phase_labels(&settings()),
        vec!["Pre-probate", "Probate", "COA", "Complaint"]
    );
}

/// THE RULE. The appeal is COA even though its date sits inside probate.
///
/// This is the assertion the whole module exists for. 2012-01-12 falls between
/// the 2009-06 and 2014-01 boundaries, so the date rule alone would call it
/// Probate — and that chip, on the Court of Appeals ruling, is the known-wrong
/// answer the architect rejected date-only assignment over.
#[test]
fn the_appeal_is_coa_even_though_its_date_falls_inside_probate() {
    let s = settings();
    let by_date_alone = phase_of(None, Some("2012-01-12"), &s);
    assert_eq!(
        by_date_alone, "Probate",
        "the date rule alone puts January 2012 in probate — which is exactly why \
         the forum has to win"
    );

    assert_eq!(
        phase_of(
            Some("doc-court-of-appeals-rulling-01-12-2012"),
            Some("2012-01-12"),
            &s
        ),
        "COA",
        "the forum map names this document an appeal, and forum wins over date"
    );
}

/// A forum settles the phase even with NO date at all.
#[test]
fn a_named_forum_settles_the_phase_without_any_date() {
    assert_eq!(
        phase_of(
            Some("doc-court-of-appeals-rulling-01-12-2012"),
            None,
            &settings()
        ),
        "COA"
    );
}

/// A document the map does not name falls through to the date, not to a guess.
#[test]
fn an_unlisted_document_falls_through_to_the_date_rule() {
    let s = settings();
    assert_eq!(
        phase_of(Some("doc-some-letter"), Some("2009-11-05"), &s),
        "Probate"
    );
}

#[test]
fn the_boundaries_split_the_timeline_at_both_ends() {
    let s = settings();
    // Before the first boundary.
    assert_eq!(phase_of(None, Some("2005"), &s), "Pre-probate");
    // Exactly ON the first boundary is already probate — the boundary is the
    // start of the phase it opens, not the last day of the one before.
    assert_eq!(phase_of(None, Some("2009-06"), &s), "Probate");
    // Exactly ON the second boundary is the complaint era.
    assert_eq!(phase_of(None, Some("2014-01"), &s), "Complaint");
    assert_eq!(phase_of(None, Some("2015-10"), &s), "Complaint");
}

/// Year-only dates classify correctly, because the comparison is on prefixes.
///
/// This is not a nicety: measured on DEV, this record's evidence dates range from
/// `"2005"` to `"2015-10"`, so a rule that needed a full day would refuse to
/// place most of them. Parsing to a date would also have to invent a day, which
/// on a witness's page is putting a date in their mouth.
#[test]
fn a_year_only_date_still_lands_in_the_right_phase() {
    let s = settings();
    assert_eq!(phase_of(None, Some("2008"), &s), "Pre-probate");
    assert_eq!(phase_of(None, Some("2011"), &s), "Probate");
    assert_eq!(phase_of(None, Some("2015"), &s), "Complaint");
}

/// No date and no forum is a REAL state on 57% of this case's evidence.
#[test]
fn a_statement_with_neither_a_forum_nor_a_date_says_so() {
    let s = settings();
    assert_eq!(phase_of(None, None, &s), "No date yet");
    assert_eq!(
        phase_of(None, Some("   "), &s),
        "No date yet",
        "whitespace is not a date"
    );
}
