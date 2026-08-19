// Tests for `practice::deck_valid` — the refusals CC_TASK_PRACTICE_V1 added.
//
// A separate file from `deck_file_tests` rather than more of it: that file was
// at 316 lines before this task, and the refusals below are about a different
// thing anyway — not "is this row well-formed" but "do these rows agree with
// each other". Every one of them is a way a deck could be internally
// inconsistent and still parse, which is the only kind of wrongness serde
// cannot see.

use crate::practice::deck_file::{
    DeckError, DeckFile, DeckKind, DeckQuestion, DeckSide, DeckSourceKind,
};

fn question(key: &str, side: DeckSide, kind: Option<DeckKind>) -> DeckQuestion {
    DeckQuestion {
        key: Some(key.to_string()),
        side,
        kind,
        follows: None,
        source_line: None,
        draft_by: None,
        source_kind: DeckSourceKind::Manual,
        source_index: None,
        tactic: None,
        braid_rows: None,
        text: format!("question {key}"),
        receipt: None,
        pair_said: None,
        pair_admitted: None,
        watch_for: None,
        stronger: None,
        stronger_lean: None,
    }
}

fn deck(questions: Vec<DeckQuestion>) -> DeckFile {
    DeckFile {
        scenario_code: "S-5".to_string(),
        points: vec![],
        questions,
    }
}

/// Absent `kind` means what every deck written before the column meant.
#[test]
fn an_absent_kind_resolves_by_side() {
    assert_eq!(
        question("g1", DeckSide::George, None).resolved_kind(),
        DeckKind::Cross
    );
    assert_eq!(
        question("c1", DeckSide::Chuck, None).resolved_kind(),
        DeckKind::Direct
    );
}

/// A redirect that does not say what it follows is refused.
///
/// A redirect exists ONLY because of the George question it repairs — the mixed
/// queue deals it immediately behind that question, and without `follows` there
/// is nowhere to put it. Writing one without a target is not a smaller redirect;
/// it is a question with no reason to exist.
#[test]
fn a_redirect_without_follows_is_refused_by_position() {
    let mut r = question("r1", DeckSide::Chuck, Some(DeckKind::Redirect));
    r.follows = None;
    let error = deck(vec![r]).validate().expect_err("must refuse");
    assert!(
        matches!(error, DeckError::RedirectWithoutFollows { position: 1 }),
        "{error}"
    );
}

/// Only a redirect may carry `follows`.
///
/// The column has the same CHECK, but a CHECK violation is a mid-transaction
/// database error naming a constraint and no line in the file.
#[test]
fn follows_on_anything_but_a_redirect_is_refused() {
    let mut c = question("c1", DeckSide::Chuck, Some(DeckKind::Direct));
    c.follows = Some("g1".to_string());
    let error = deck(vec![question("g1", DeckSide::George, None), c])
        .validate()
        .expect_err("must refuse");
    assert!(
        matches!(
            error,
            DeckError::FollowsOnNonRedirect {
                position: 2,
                kind: "direct",
                ..
            }
        ),
        "{error}"
    );
}

/// A `follows` naming a key the file does not carry is refused.
///
/// This is the check a foreign key would have bought. `follows_key` deliberately
/// is not one — the file speaks in keys and the seed writes a George row and its
/// redirect in the same transaction — so the check is made here, at the moment a
/// human can still fix the file.
#[test]
fn a_follows_naming_no_question_in_the_deck_is_refused() {
    let mut r = question("r1", DeckSide::Chuck, Some(DeckKind::Redirect));
    r.follows = Some("g9".to_string());
    let error = deck(vec![question("g1", DeckSide::George, None), r])
        .validate()
        .expect_err("must refuse");
    assert!(
        matches!(error, DeckError::FollowsUnknownKey { position: 2, ref follows } if follows == "g9"),
        "{error}"
    );
}

/// A redirect may not follow another redirect, or a direct.
///
/// Stricter than an FK could ever be — an FK sees a row, not a kind — and it is
/// the case that would actually happen: a copied block whose `follows` was left
/// pointing at the redirect above it, which would deal two of Chuck's questions
/// in a row with no George trap between them.
#[test]
fn a_redirect_following_something_that_is_not_a_cross_is_refused() {
    let mut r1 = question("r1", DeckSide::Chuck, Some(DeckKind::Redirect));
    r1.follows = Some("g1".to_string());
    let mut r2 = question("r2", DeckSide::Chuck, Some(DeckKind::Redirect));
    r2.follows = Some("r1".to_string());

    let error = deck(vec![question("g1", DeckSide::George, None), r1, r2])
        .validate()
        .expect_err("must refuse");
    assert!(
        matches!(
            error,
            DeckError::FollowsNotCross {
                position: 3,
                kind: "redirect",
                ..
            }
        ),
        "{error}"
    );
}

/// Two questions cannot share a key.
///
/// The key is the IDENTITY `--update` matches on. A duplicate would mean one of
/// them silently overwriting the other on every run — and the database's unique
/// index would catch it only as a constraint error, mid-transaction, naming an
/// index rather than the two questions.
#[test]
fn two_questions_sharing_a_key_are_refused_by_key() {
    let error = deck(vec![
        question("g1", DeckSide::George, None),
        question("g1", DeckSide::Chuck, None),
    ])
    .validate()
    .expect_err("must refuse");
    assert!(
        matches!(error, DeckError::DuplicateKey { ref key } if key == "g1"),
        "{error}"
    );
}

/// A blank optional field is refused rather than treated as absent.
///
/// The columns behind these carry `btrim(...) <> ''` checks, so a blank one
/// fails as a constraint error naming no line in the file. And the intent is
/// different anyway: `key: ""` is somebody who meant to write a key.
#[test]
fn a_blank_optional_field_is_refused_and_never_read_as_absent() {
    let mut blank_key = question("g1", DeckSide::George, None);
    blank_key.key = Some("   ".to_string());
    assert!(matches!(
        deck(vec![blank_key]).validate(),
        Err(DeckError::BlankKey { position: 1 })
    ));

    let mut blank_source = question("g1", DeckSide::George, None);
    blank_source.source_line = Some(String::new());
    assert!(matches!(
        deck(vec![blank_source]).validate(),
        Err(DeckError::BlankSourceLine { position: 1 })
    ));

    let mut blank_draft = question("g1", DeckSide::George, None);
    blank_draft.draft_by = Some(" ".to_string());
    assert!(matches!(
        deck(vec![blank_draft]).validate(),
        Err(DeckError::BlankDraftBy { position: 1 })
    ));
}

/// A well-formed cross + redirect pair passes.
///
/// ANTI-VACUITY: every test above asserts a refusal, and a `validate` that
/// refused EVERYTHING would pass all of them. This is the one that says the
/// shape the task actually asks for is accepted.
#[test]
fn a_cross_and_its_redirect_are_accepted() {
    let mut r = question("r1", DeckSide::Chuck, Some(DeckKind::Redirect));
    r.follows = Some("g1".to_string());
    assert_eq!(
        deck(vec![question("g1", DeckSide::George, None), r]).validate(),
        Ok(())
    );
}
