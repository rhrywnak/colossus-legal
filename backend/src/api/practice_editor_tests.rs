// =============================================================================
// Which fields the deck editor may write, and which it must refuse
// =============================================================================
//
// The three private helpers at the top of [`super`] are the whole of the edit
// route's vocabulary: `column_for` decides what a client's string is allowed to
// become, `current` decides what the change row records as the OLD value, and
// `kind_for` decides what Marie's changed-list calls the edit. All three are
// exhaustive matches over the same set of field names, and they are written in
// three separate places — which is exactly the shape that drifts.
//
// ## Why these tests exist now
//
// `receipt` became editable on 2026-09-15 (CC_TASK_PRACTICE_POLISH_v1), and
// adding a field means adding an arm to each of the three. A field added to
// `column_for` alone would WRITE correctly and log a change row whose `before` was
// empty — Marie would be told the receipt changed and shown nothing it changed
// FROM, silently, on every receipt edit. Nothing in this module was tested before
// today, so nothing would have caught it.
//
// ## Why the module and not the HTTP surface
//
// These three functions are private, and running the handler needs a pool and a
// transaction — there is no database in `cargo test`. The rule they enforce is
// pure, so it is tested where it is pure. The route TABLE (that the path exists
// and takes POST) is `practice_tests`; the fences the add path applies are
// `practice_editor_add_tests`.

use super::*;
use uuid::Uuid;

/// A question record with every editable field filled, so `current` has
/// something to find for each of them.
///
/// ## Rust Learning: a fixture function rather than a `const`
///
/// `PracticeQuestionRecord` holds `String`s and a `Uuid`, neither of which can be
/// built in a `const` — `String` allocates, and allocation cannot happen at
/// compile time. The idiom is a function returning a fresh value per test, which
/// also means one test mutating the fixture cannot reach another.
fn question() -> PracticeQuestionRecord {
    PracticeQuestionRecord {
        id: Uuid::nil(),
        scenario_id: Uuid::nil(),
        side: "george".to_string(),
        text: "You never filed the claim, did you?".to_string(),
        tactic: Some(3),
        braid_rows: None,
        source_kind: "instance".to_string(),
        source_ref: None,
        source_line: None,
        receipt: Some("Built from: Supp. Brief p.4".to_string()),
        watch_for: Some("Do not accept the premise.".to_string()),
        pair_said: None,
        pair_admitted: None,
        stronger: Some("The form said there was nobody else.".to_string()),
        stronger_lean: None,
        flag_note: None,
        kind: "cross".to_string(),
        deck_key: Some("g1".to_string()),
        follows_key: None,
        hidden_at: None,
        draft_by: None,
        sort_order: 1,
        updated_at: chrono::Utc::now(),
    }
}

/// Every field the browser may send maps to a column.
///
/// The list is the frontend's `EditableField` union, spelled out. A field added
/// there and not here is a 400 the moment somebody uses it.
#[test]
fn every_editable_field_names_its_own_column() {
    assert_eq!(column_for("text"), Some("text"));
    assert_eq!(column_for("watch_for"), Some("watch_for"));
    assert_eq!(column_for("stronger"), Some("stronger"));
    assert_eq!(column_for("follows"), Some("follows_key"));
    assert_eq!(
        column_for("receipt"),
        Some("receipt"),
        "the Built-from line became editable on 2026-09-15"
    );
}

/// A field this build does not implement is refused, never guessed at.
///
/// `column_for` interpolates its result into SQL. The `_` arm is the injection
/// fence, and a test that only checked the accepted names would pass just as
/// happily if the fence returned the input.
#[test]
fn a_field_this_build_cannot_edit_is_refused() {
    assert_eq!(column_for("bogus"), None);
    assert_eq!(column_for(""), None);
    // The tactic is editable but is NOT a text column — it takes the numeric
    // path in `write_field` and must never reach a `SET <column> = $2`.
    assert_eq!(column_for("tactic"), None, "the tactic has its own writer");
    // Neither an injection nor a column that exists but is not authored content.
    assert_eq!(column_for("hidden_at"), None);
    assert_eq!(column_for("text, hidden_at = NULL --"), None);
}

/// The change row's `before` is read for every editable field.
///
/// ## Domain note: an empty `before` is a lie Marie cannot check
///
/// `practice_deck_changes` is what the changed-list is built from, and a change
/// that says a field moved without saying what it moved FROM is a badge she
/// cannot act on. Each arm here is one field that would otherwise log `None`.
#[test]
fn the_old_value_is_read_for_every_editable_field() {
    let q = question();
    assert_eq!(current(&q, "text").as_deref(), Some(q.text.as_str()));
    assert_eq!(
        current(&q, "watch_for").as_deref(),
        Some("Do not accept the premise.")
    );
    assert_eq!(
        current(&q, "stronger").as_deref(),
        Some("The form said there was nobody else.")
    );
    assert_eq!(
        current(&q, "receipt").as_deref(),
        Some("Built from: Supp. Brief p.4"),
        "a receipt edit must record what it replaced"
    );
    // The tactic's `before` is the CARD NUMBER as text — the change row stores
    // what was stored, not the name a screen resolved it to.
    assert_eq!(current(&q, "tactic").as_deref(), Some("3"));
    assert_eq!(current(&q, "bogus"), None);
}

/// A field that is empty now records `None`, not an empty string.
///
/// "This had no receipt" and "this had a receipt of nothing" are different facts,
/// and the change row's column keeps them different.
#[test]
fn a_field_that_is_empty_now_has_no_before() {
    let mut q = question();
    q.receipt = None;
    q.watch_for = None;
    assert_eq!(current(&q, "receipt"), None);
    assert_eq!(current(&q, "watch_for"), None);
}

// =============================================================================
// The tactic's range, refused HERE rather than by the column (2026-09-15, Q4)
// =============================================================================
//
// `write_field` is pool-bound — it takes a transaction — so this is a SOURCE
// SCAN, the same tier and for the same reason as `practice_answers_tests`: the
// behaviour lives in a function that needs a database, and the repository's
// convention for that is to read the source and assert what it says.
//
// ## What this catches, and what it cannot
//
// It catches the guard being deleted, or drifting BELOW the write (where it
// would refuse nothing). It cannot prove the resulting status code is 400 — only
// a request through the router could, and there is no such tier here.

/// This module's source, with its comments stripped.
///
/// ⚑ Comments first, always: this file documents the rule three inches above the
/// code that keeps it, so a scan for the rule's own words finds the prose.
fn write_field_body() -> String {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/api/practice_editor.rs"),
    )
    .expect("the editor route is on disk");
    let source: String = source
        .lines()
        .map(|line| match line.find("//") {
            Some(at) => &line[..at],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n");
    let from = source
        .find("async fn write_field(")
        .expect("the field writer is declared");
    let rest = &source[from..];
    let to = rest.find("\n}").map(|i| i + 2).unwrap_or(rest.len());
    rest[..to].to_string()
}

/// A card outside 1–7 is refused BEFORE the write, with a 400.
///
/// ## Domain note: what this replaced
///
/// `8` parses as an `i16`, so until today it reached `set_tactic`, hit the
/// column's `CHECK (tactic BETWEEN 1 AND 7)`, and came back through `repo_error`
/// as a **500** — a message that says "this system broke" about a value the
/// error one line above already calls invalid. The add path
/// (`practice_editor_add_fences::fence_tactic`) had always answered 400. Same
/// rule, two doors, two answers.
#[test]
fn an_out_of_range_card_is_refused_before_the_column_sees_it() {
    let body = write_field_body();

    let guard = body
        .find("1..=TACTIC_CARD_MAX")
        .expect("the edit path must range-check the card, not leave it to the column");
    let write = body
        .find("set_tactic(")
        .expect("ANTI-VACUITY: the writer is still called from here");

    assert!(
        guard < write,
        "the range check sits BELOW the write — it refuses nothing: {body}"
    );
    assert!(
        body.contains("BadRequest"),
        "an out-of-range card must be a 400 and not a repository error: {body}"
    );
}

/// The refusal says the same thing on both doors.
///
/// The add path and the edit path fence one rule. Two messages for one rule is
/// how a person learns that "the form" and "the row" have different rules, which
/// they do not.
#[test]
fn both_doors_refuse_a_bad_card_in_the_same_words() {
    let body = write_field_body();
    let fences = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/api/practice_editor_add_fences.rs"),
    )
    .expect("the add fences are on disk");

    let sentence = "tactic must be a card number from 1 to {TACTIC_CARD_MAX}";
    assert!(
        body.contains(sentence),
        "the edit path's words changed: {body}"
    );
    assert!(
        fences.contains(sentence),
        "the add path's words changed — the two doors now disagree"
    );
}

/// Only a re-wording is a re-wording.
///
/// The kind is what tells Marie whether she must RE-READ the question. A receipt
/// correction is `edited` with the field named, like every other field that is
/// not the question's own words.
#[test]
fn only_the_question_text_is_recorded_as_a_rewording() {
    assert_eq!(kind_for("text"), "reworded");
    assert_eq!(kind_for("receipt"), "edited");
    assert_eq!(kind_for("watch_for"), "edited");
    assert_eq!(kind_for("tactic"), "edited");
}
