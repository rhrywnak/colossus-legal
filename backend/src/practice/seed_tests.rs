// Tests for `practice::seed`'s pure decisions.
//
// The database halves (`read_sources`, `write_deck`) need a live pipeline pool
// and are exercised by the operator's dry run, which prints the same counts this
// module renders. What IS unit-testable is the part that decides — the binding of
// a question to a source, the already-seeded fork, and the proof text — and those
// are the three that can be silently wrong.

use super::*;
use crate::practice::deck_file::{DeckQuestion, DeckSide};

fn sources() -> ScenarioSources {
    ScenarioSources {
        scenario_id: Uuid::nil(),
        instances: vec![
            "doc-hearing:evidence:aaa".to_string(),
            "doc-hearing:evidence:bbb".to_string(),
        ],
        points: vec![Uuid::from_u128(1), Uuid::from_u128(2), Uuid::from_u128(3)],
    }
}

fn question(kind: DeckSourceKind, index: Option<usize>) -> DeckQuestion {
    DeckQuestion {
        side: DeckSide::George,
        source_kind: kind,
        source_index: index,
        tactic: None,
        braid_rows: None,
        text: "a question".to_string(),
        receipt: None,
        pair_said: None,
        pair_admitted: None,
        watch_for: None,
        stronger: None,
        stronger_lean: None,
    }
}

/// A position in the file becomes the id of the thing it names.
///
/// This is the join the whole "nothing is invented" claim rests on: position 2 in
/// the deck must resolve to the SECOND ruled instance, not the first and not the
/// last. An off-by-one here would attach every cross question to the wrong
/// evidence, and every screen would still render.
#[test]
fn a_one_based_position_resolves_to_that_source() {
    let s = sources();

    assert_eq!(
        resolve_ref(&question(DeckSourceKind::Instance, Some(1)), 1, &s, "S-5").unwrap(),
        Some("doc-hearing:evidence:aaa".to_string())
    );
    assert_eq!(
        resolve_ref(&question(DeckSourceKind::Instance, Some(2)), 2, &s, "S-5").unwrap(),
        Some("doc-hearing:evidence:bbb".to_string())
    );
    assert_eq!(
        resolve_ref(&question(DeckSourceKind::Point, Some(3)), 3, &s, "S-5").unwrap(),
        Some(Uuid::from_u128(3).to_string())
    );
}

/// A manual question binds to nothing, and that is not a failure.
#[test]
fn a_manual_question_resolves_to_no_ref() {
    assert_eq!(
        resolve_ref(
            &question(DeckSourceKind::Manual, None),
            1,
            &sources(),
            "S-5"
        )
        .unwrap(),
        None
    );
}

/// A position the scenario cannot honour REFUSES, and the message carries the
/// number an operator needs.
///
/// The alternative — writing the row with a NULL ref while its receipt still
/// claims "Built from: the hearing, p. 34" — is precisely the stale-pointer
/// shape: a screen asserting a source that is not there.
#[test]
fn a_position_past_the_end_refuses_and_names_what_is_available() {
    let error = resolve_ref(
        &question(DeckSourceKind::Instance, Some(3)),
        4,
        &sources(),
        "S-5",
    )
    .expect_err("the scenario has two instances, not three");

    match error {
        SeedError::SourceOutOfRange {
            position,
            kind,
            index,
            ref code,
            available,
        } => {
            assert_eq!((position, kind, index, available), (4, "instance", 3, 2));
            assert_eq!(code, "S-5");
        }
        other => panic!("wrong refusal: {other}"),
    }

    let message = resolve_ref(
        &question(DeckSourceKind::Point, Some(9)),
        1,
        &sources(),
        "S-5",
    )
    .expect_err("the scenario has three points")
    .to_string();
    assert!(message.contains("point 9"), "{message}");
    assert!(message.contains("only 3"), "{message}");
    assert!(
        message.contains("nothing was written"),
        "the operator must be told the store is untouched: {message}"
    );
}

/// A second run over an unchanged deck is a NO-OP that says so.
///
/// This is what makes the tool safe to repeat, which the one-shot family
/// requires. It must not read as success-with-writes, because an operator who
/// believes a second run re-seeded would go looking for edits that never landed.
#[test]
fn a_repeat_run_over_the_same_deck_is_a_no_op_and_not_a_write() {
    let s = sources();
    let deck = DeckFile {
        scenario_code: "S-5".to_string(),
        questions: vec![
            question(DeckSourceKind::Manual, None),
            question(DeckSourceKind::Manual, None),
        ],
    };

    let report = finish_already_seeded("S-5", &s, &deck, 2).expect("the counts agree");
    assert!(report.already_seeded);
    assert!(!report.written, "a no-op has written nothing");
    assert_eq!(report.questions_before, report.questions_after);
    assert!(render_report(&report).contains("already carries this deck"));
}

/// A deck that DIFFERS from the stored one refuses rather than overwriting.
///
/// The consequence of overwriting is not abstract: `practice_answers` cites a
/// question id with ON DELETE RESTRICT, so Chuck's sheet for a session Marie has
/// already run would lose the question she was asked. The refusal names both
/// counts so the operator can see which way it drifted.
#[test]
fn a_changed_deck_refuses_because_an_answer_may_already_cite_a_question() {
    let deck = DeckFile {
        scenario_code: "S-5".to_string(),
        questions: vec![question(DeckSourceKind::Manual, None)],
    };

    let message = finish_already_seeded("S-5", &sources(), &deck, 10)
        .expect_err("10 stored against 1 incoming")
        .to_string();

    assert!(message.contains("10 questions"), "{message}");
    assert!(message.contains("1 questions"), "{message}");
    assert!(message.contains("nothing was written"), "{message}");
}

/// The proof text distinguishes the three outcomes.
///
/// A dry run and a write must not read alike — the family's whole discipline is
/// that the operator can tell from the output whether anything happened.
#[test]
fn the_proof_text_tells_a_dry_run_from_a_write() {
    let base = SeedReport {
        scenario_code: "S-5".to_string(),
        scenario_id: Uuid::nil(),
        questions_before: 0,
        questions_after: 0,
        questions_planned: 10,
        instances_available: 5,
        points_available: 3,
        written: false,
        already_seeded: false,
    };

    assert!(render_report(&base).contains("DRY RUN — nothing was written"));

    let written = SeedReport {
        questions_after: 10,
        written: true,
        ..base.clone()
    };
    let text = render_report(&written);
    assert!(text.contains("WROTE the deck"));
    assert!(
        text.contains("questions before      0") && text.contains("questions after       10"),
        "the count proof is the point of the report: {text}"
    );
}

// ── `load_deck`'s two I/O paths ─────────────────────────────────────────────
//
// Pure file I/O — no `PgPool` — so it does not fall under the DEV-verified
// convention, and the shape is the one `pipeline::config`'s loader tests already
// cover (`load_missing_profile_returns_error`, `load_invalid_yaml_returns_error`).
//
// What makes these worth writing rather than assuming: the operator running this
// binary at 7am on a Tuesday has a filename in one hand and a terminal in the
// other, and the ONLY thing that tells them which of the two mistakes they made
// is the sentence these tests pin. "No such file" without the path sends them
// hunting through three directories.

/// A scratch directory that cleans itself up. Written here rather than pulling in
/// a temp-dir crate for two tests — the same choice `settings_template_file`'s
/// tests made, for the same reason.
struct Scratch(std::path::PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("colossus-practice-deck-{name}"));
        let _ = std::fs::remove_dir_all(&path); // best-effort: a leftover from a previous run
        std::fs::create_dir_all(&path).expect("test scratch dir is creatable");
        Scratch(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0); // best-effort: test cleanup
    }
}

/// A deck file nobody deployed refuses BY PATH.
#[test]
fn a_deck_file_that_is_not_there_refuses_and_names_where_it_looked() {
    let missing = std::path::Path::new("/this/does/not/exist/S-9.yaml");
    let error = load_deck(missing).expect_err("no such file");

    assert!(
        matches!(error, SeedError::Unreadable { .. }),
        "wrong refusal: {error}"
    );
    let message = error.to_string();
    assert!(
        message.contains("/this/does/not/exist/S-9.yaml"),
        "the refusal must name the path it looked at: {message}"
    );
}

/// A deck file that is not YAML refuses as a PARSE failure, not as a missing one.
///
/// The distinction is the whole point: "could not be read" sends an operator to
/// the filesystem, "is not valid YAML" sends them to their editor. Collapsing the
/// two would send them to the wrong place half the time.
#[test]
fn a_deck_file_that_is_not_yaml_refuses_as_a_parse_failure_and_names_the_file() {
    let scratch = Scratch::new("bad-yaml");
    let path = scratch.0.join("S-9.yaml");
    std::fs::write(
        &path,
        "scenario_code: S-9\nquestions: [ this: is: not: yaml",
    )
    .expect("test file is writable");

    let error = load_deck(&path).expect_err("that is not a deck");

    assert!(
        matches!(error, SeedError::Unparseable { .. }),
        "a malformed file must not be reported as an unreadable one: {error}"
    );
    let message = error.to_string();
    assert!(message.contains("S-9.yaml"), "{message}");
    assert!(message.contains("not valid YAML"), "{message}");
}

/// A file that parses but is not a legal deck refuses as INVALID.
///
/// The third arm, and the one that proves `load_deck` actually calls `validate`.
/// Without this the loader could hand a blank-question deck straight to the
/// database, and every refusal `deck_file_tests` pins would be unreachable in
/// production.
#[test]
fn a_deck_that_parses_but_is_illegal_refuses_before_any_connection_is_opened() {
    let scratch = Scratch::new("invalid");
    let path = scratch.0.join("S-9.yaml");
    std::fs::write(
        &path,
        "scenario_code: S-9\nquestions:\n  - side: george\n    source_kind: manual\n    text: '   '\n",
    )
    .expect("test file is writable");

    let error = load_deck(&path).expect_err("question 1 is blank");

    assert!(
        matches!(error, SeedError::Invalid { .. }),
        "wrong refusal: {error}"
    );
    assert!(error.to_string().contains("question 1"), "{error}");
}
