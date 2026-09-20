// =============================================================================
// THE RULES THE READER APPLIES SILENTLY, APPLIED LOUDLY
// =============================================================================
//
// Every refusal below exists because `parse_verbatim_list` would otherwise
// accept the input and quietly change it. The test that matters most is the
// last one: it reads the encoded rows back through the REAL reader and proves
// the columns still line up, which is the invariant the whole feature turns on.

use super::*;

use crate::domain::settings::parse_verbatim_list;
use crate::services::settings_groups::group_by_id;

fn bench() -> &'static CoupledGroup {
    group_by_id("reviewer_bench").expect("the reviewer bench is declared")
}

fn entry(login: &str, name: &str) -> Vec<String> {
    vec![login.to_string(), name.to_string()]
}

#[test]
fn two_reviewers_encode_to_two_aligned_rows() {
    let rows = encode_entries(
        bench(),
        &[entry("cpenzien", "Chuck"), entry("roman", "Roman")],
    )
    .expect("a well-formed bench is accepted");

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].key, "practice_reviewer_usernames");
    assert_eq!(rows[0].value, "cpenzien,roman");
    assert_eq!(rows[1].key, "practice_reviewer_display_names");
    assert_eq!(rows[1].value, "Chuck,Roman");
}

#[test]
fn the_shipped_single_reviewer_still_encodes() {
    let rows = encode_entries(bench(), &[entry("cpenzien", "Chuck")]).expect("one is legal");
    assert_eq!(rows[0].value, "cpenzien");
    assert_eq!(rows[1].value, "Chuck");
}

#[test]
fn surrounding_whitespace_is_trimmed_not_stored() {
    let rows = encode_entries(bench(), &[entry("  roman  ", "  Roman  ")]).expect("trimmed");
    assert_eq!(rows[0].value, "roman");
}

/// An empty bench is refused. (M)
///
/// REQUIRED by the 2026-09-20 ruling, and newly reachable: until the pair write
/// existed, `("none", "none")` could not be submitted because each row had to be
/// saved alone and each save was refused. A bench with nobody on it passes the
/// length check happily — 0 == 0 — and leaves a review queue no one can clear.
#[test]
fn an_empty_bench_is_refused_because_nobody_could_then_clear_the_queue() {
    let refused = encode_entries(bench(), &[]).expect_err("zero entries must be refused");

    assert!(matches!(refused, PairError::Empty { .. }));
    let sentence = refused.to_string();
    assert!(
        sentence.contains("at least one reviewer"),
        "the refusal must name what to add: {sentence}"
    );
    assert!(
        sentence.contains("review queue can never be cleared"),
        "and why it matters: {sentence}"
    );
}

/// A blank cell is refused rather than silently dropped. (M)
#[test]
fn a_blank_cell_is_refused_naming_its_column_and_row() {
    let refused = encode_entries(bench(), &[entry("cpenzien", "Chuck"), entry("roman", "  ")])
        .expect_err("a blank display name must be refused");

    match refused {
        PairError::BlankCell {
            column, position, ..
        } => {
            assert_eq!(column, "Name shown on screen");
            assert_eq!(
                position, 2,
                "positions are 1-based — the operator counts rows"
            );
        }
        other => panic!("expected a blank-cell refusal, got {other:?}"),
    }
}

/// A repeated login is refused. (M)
#[test]
fn a_repeated_login_is_refused_by_name() {
    let refused = encode_entries(bench(), &[entry("roman", "Roman"), entry("roman", "R2")])
        .expect_err("a duplicate login must be refused");

    match refused {
        PairError::Duplicate { column, value, .. } => {
            assert_eq!(column, "Sign-in name");
            assert_eq!(value, "roman");
        }
        other => panic!("expected a duplicate refusal, got {other:?}"),
    }
}

/// ⚑ A repeated DISPLAY NAME is refused too — the surprising half. (M)
///
/// The obvious rule is "logins must be unique". The reader de-duplicates EVERY
/// list, so two reviewers sharing a display name collapse that column to one
/// entry while the logins column keeps both — and the store then refuses to boot
/// over a length mismatch nobody typed.
#[test]
fn a_repeated_display_name_is_refused_because_the_reader_dedupes_every_column() {
    let refused = encode_entries(
        bench(),
        &[entry("cpenzien", "Chuck"), entry("croberts", "Chuck")],
    )
    .expect_err("two reviewers cannot share a printed name");

    match refused {
        PairError::Duplicate { column, value, .. } => {
            assert_eq!(column, "Name shown on screen");
            assert_eq!(value, "Chuck");
        }
        other => panic!("expected a duplicate refusal, got {other:?}"),
    }
}

/// A comma inside a value is refused. (M)
#[test]
fn a_comma_in_a_value_is_refused_because_the_store_cannot_escape_one() {
    let refused = encode_entries(bench(), &[entry("gsmith", "Smith, Jr.")])
        .expect_err("a comma would be read back as two entries");

    assert!(matches!(refused, PairError::SeparatorInValue { .. }));
    assert!(refused.to_string().contains("read back as two"));
}

/// The literal `none` is refused as a value. (M)
#[test]
fn the_none_token_is_refused_as_a_value() {
    let refused = encode_entries(bench(), &[entry("none", "Nobody")])
        .expect_err("`none` alone reads back as an EMPTY list");

    assert!(matches!(refused, PairError::NoneTokenAsValue { .. }));
}

/// A row of the wrong width is refused rather than indexed past the end.
#[test]
fn an_entry_with_the_wrong_number_of_cells_is_refused() {
    let refused = encode_entries(bench(), &[vec!["roman".to_string()]])
        .expect_err("one cell in a two-column group is a shape error");

    match refused {
        PairError::WrongArity {
            found, expected, ..
        } => {
            assert_eq!(found, 1);
            assert_eq!(expected, 2);
        }
        other => panic!("expected an arity refusal, got {other:?}"),
    }
}

/// ⚑ What is encoded here reads back ALIGNED through the real reader. (M)
///
/// This is the invariant the whole feature turns on, and it is asserted against
/// `parse_verbatim_list` itself rather than against a second copy of its rules.
/// Every other test in this file is a way of NOT reaching this one's failure.
#[test]
fn whatever_this_module_encodes_reads_back_with_the_columns_in_step() {
    let entries = [
        entry("cpenzien", "Chuck"),
        entry("roman", "Roman"),
        entry("docmarie", "Marie"),
    ];
    let rows = encode_entries(bench(), &entries).expect("well-formed");

    let read_back: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            parse_verbatim_list(row.key, &row.value).expect("the reader accepts what we wrote")
        })
        .collect();

    // Every column reads back with exactly as many entries as were submitted —
    // which is the length check in `settings_practice::reviewer_bench`, proved
    // here before the store ever sees the value.
    for column in &read_back {
        assert_eq!(
            column.len(),
            entries.len(),
            "a column changed length between encoding and reading: {read_back:?}"
        );
    }
    assert_eq!(read_back[0], vec!["cpenzien", "roman", "docmarie"]);
    assert_eq!(read_back[1], vec!["Chuck", "Roman", "Marie"]);
}
