//! The answer path's contract, at the only layer this environment can assert it.
//!
//! ## ⚑ WHY A SCANNER AND NOT A BEHAVIOURAL TEST — the question to ask first
//!
//! **Because the behavioural test cannot run here.** That is the whole answer,
//! and it was MEASURED rather than assumed: bypass the version rule in the
//! handler (`let answer_id = if false {`) so that every press writes a version,
//! and `services::practice_answer_version`'s six tests still pass. They test the
//! pure DECISION, which the mutation never touches. Only this file notices.
//!
//! A real behavioural test — "identical text creates no second version" — has to
//! run `post_practice_answer`, which needs a pool, and there is no database in
//! the unit-test environment.
//!
//! **The day that stops being true, DELETE THIS FILE.** A scanner is coupled to
//! the SHAPE of the code, not to what it does: refactor the `if` into a `match`
//! or an early return and the behaviour is identical while these fail. A test
//! that will one day demand the wrong repair is worth keeping only while it is
//! the only test there is.
//!
//! ## ⚑ ONE ITEM, TWO EFFECTS — for whoever fixes the test tier
//!
//! The event that deletes this file is the same event that closes a hole: this
//! repository has no test tier that can reach a database, so
//! `post_practice_answer` — the path EVERY answer Marie writes travels down —
//! is reachable only by inspection. Whoever builds that tier is not doing two
//! jobs. **This scanner is yours to remove**, and its removal is how you will
//! know the tier is real.
//!
//! ## Why these are SOURCE SCANS
//!
//! The behaviour they pin lives in a handler that needs a pool, and there is no
//! database in the unit-test environment. The repository's own convention for
//! pool-bound code is Rule 21 — read the source and the migrations off disk and
//! assert what they say — and that is what these do.
//!
//! The DECISION itself is not scanned: it is a pure function with real tests in
//! `services::practice_answer_version`. These assert that the handler is WIRED
//! to it, which is the half a unit test of the pure function cannot see.
//!
//! ## ⚑ Why this file exists at all
//!
//! `fence_not_already_answered` was deleted on 2026-08-23 because it returned
//! 409 on the loop `CC_TASK_PRACTICE_ONE_PAGE` §4 makes the whole design. It had
//! no test of its own — the sibling fence tests cover only the two PURE fences —
//! so its removal took nothing with it, and the rule that replaced it was
//! watched by nothing at all. That is one step worse than this week's three
//! green-but-blind fixtures: not a test that stopped watching, but a behaviour
//! nothing ever watched.

use std::path::Path;

/// The answer handler's source.
fn handler() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/api/practice_answers.rs"),
    )
    .expect("the answer handler is on disk")
}

/// One source file with its `//` comments removed.
///
/// ## ⚑ Why this exists, and the defect it is answering
///
/// These tests assert that certain names are ABSENT from the handler. The
/// handler also carries a long comment explaining why one of them was deleted —
/// so the first version of this file failed on its own documentation.
///
/// That is the third time today one shape has bitten: a migration documents
/// `SET value         = '` in its header and a parser searching for that string
/// finds the comment first; a wording fixture reads a seed migration whose
/// comments quote the very format it parses; and this. **Prose about a rule
/// matches a parser looking for the rule.** Strip the prose before you scan.
///
/// The rule this obeys, and WHY it is a rule rather than an accident, is stated
/// once in `domain::wording_tests` — above `seeded_value_in`, beside the two
/// parsers that learned it first. Do not copy it here; the fourth scanner will
/// be in a file nobody thought to copy it into.
///
/// Deliberately crude — it does not know about `//` inside a string literal.
/// There is none in this handler, and a real tokenizer here would be a parser
/// nobody asked for guarding four assertions.
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

/// The body of `post_practice_answer`, from its signature to the next `pub fn`,
/// with comments stripped — see [`without_comments`].
fn post_answer_body() -> String {
    let source = handler();
    let from = source
        .find("pub async fn post_practice_answer(")
        .expect("the answer handler is declared");
    let rest = &source[from..];
    let to = rest[1..]
        .find("\npub async fn ")
        .map(|i| i + 1)
        .unwrap_or(rest.len());
    without_comments(&rest[..to])
}

/// A SECOND answer to the same question is not refused.
///
/// Roman's ruling of 2026-08-23: she edits the box and presses Answer again, and
/// that loop is the design. The sitting is reused across an afternoon now, so a
/// per-sitting refusal would fire on her second edit of the day.
#[test]
fn a_second_answer_to_one_question_is_not_refused() {
    let body = post_answer_body();

    assert!(
        !body.contains("fence_not_already_answered"),
        "the per-sitting refusal is back on the answer path — it 409s the loop \
         §4 calls the whole design"
    );
    assert!(
        !body.contains("AppError::Conflict"),
        "the answer path returns a Conflict again. A second answer to one \
         question is a VERSION, not a collision: {body}"
    );
}

/// Whether it writes a version is decided by the RULE, not re-derived here.
///
/// The handler must call `is_reread`. A second copy of "are these the same
/// words" inline would agree with the pure function today and drift the first
/// time either was touched — and the drift would be silent, because both
/// answers are plausible.
#[test]
fn the_handler_asks_the_rule_rather_than_deciding_for_itself() {
    let body = post_answer_body();

    assert!(
        body.contains("is_reread("),
        "the handler must consult `practice_answer_version::is_reread`"
    );
    assert!(
        !body.contains("== &body.answer_text") && !body.contains("== body.answer_text"),
        "the handler compares the text itself — that is a second copy of the \
         rule, and it will drift: {body}"
    );
}

/// A new version is written ONLY when the rule says the text changed.
///
/// The structural half of "same text twice creates no second version": the
/// insert must sit inside the branch the rule guards. An `insert_answer` outside
/// it would stack an identical row on every press, which is exactly what the
/// ruling exists to stop.
#[test]
fn the_insert_is_reached_only_when_the_text_changed() {
    let body = post_answer_body();

    let decision = body.find("is_reread(").expect("the rule is consulted");
    let insert = body.find("insert_answer(").expect("a version is inserted");
    assert!(
        insert > decision,
        "insert_answer runs before the rule is consulted, so every press writes \
         a version regardless of what she typed"
    );

    // And it is inside an `else` — the branch taken when the text CHANGED.
    let between = &body[decision..insert];
    assert!(
        between.contains("} else {"),
        "the insert is not in the changed-text branch: {between}"
    );

    // ⚑ AND THE GUARD ITSELF MUST READ THE RULE'S ANSWER.
    //
    // Mutation-checked, twice, because the first two attempts at this assertion
    // BOTH passed under `if false`. Shape was not enough — a branch can be
    // correctly shaped and decided by something else. Nor was counting the
    // binding's occurrences: it appears three times, so losing one still
    // cleared a `>= 2` threshold. What has to be true is narrower and exact:
    // the name the rule fills appears in the CONDITION of the `if` that chooses
    // between re-reading and inserting.
    let name = body[..decision]
        .rsplit("let ")
        .next()
        .and_then(|tail| tail.split_whitespace().next())
        .expect("the rule's answer is bound to a name");

    let guard_at = body
        .find("let answer_id = if")
        .expect("the answer id is chosen by a branch");
    let guard = &body[guard_at
        ..body[guard_at..]
            .find('{')
            .map(|i| guard_at + i)
            .expect("the branch opens")];

    assert!(
        guard.contains(name),
        "the branch that chooses between re-reading and inserting does not \
         mention `{name}` — it is decided by something other than what she \
         typed. Guard: {guard}"
    );
}

/// The re-read reuses the standing row, so its critique is REPLACED.
///
/// `attach_read` is `UPDATE practice_answers SET … WHERE id = $1`, so handing it
/// the existing answer's id overwrites that row's read rather than adding one.
/// Two critiques of one answer would be exactly the noise the version rule
/// exists to prevent — Roman, 2026-08-23.
#[test]
fn a_re_read_reuses_the_standing_answer_row() {
    let body = post_answer_body();
    let decision = body.find("is_reread(").expect("the rule is consulted");
    let after = &body[decision..];

    assert!(
        after.contains("*existing"),
        "the re-read branch must reuse the standing answer's id, or it writes a \
         second row and the ruling is undone: {after}"
    );
    assert!(
        after.contains("attach_read"),
        "the read must still be attached on the re-read path — otherwise \
         pressing Answer on unchanged text runs a read nobody ever sees"
    );
}

/// BOTH arms of the version decision announce themselves.
///
/// Rule 1: two operationally distinct states must produce two observables. With
/// only the re-read logged, an operator would infer "a version was written" from
/// the ABSENCE of a line — indistinguishable from the request never arriving.
///
/// ## ⚑ Asserted by CONTENT, not by count
///
/// A count of `tracing::info!` after the decision is satisfied by lines that are
/// not these two — the same failure as the `>= 2` binding count elsewhere in
/// this file's history. Each arm is identified by the WORDS it logs.
#[test]
fn both_arms_of_the_version_decision_are_logged() {
    let body = post_answer_body();

    assert!(
        body.contains("re-reading, not versioning"),
        "the re-read arm must say so"
    );
    assert!(
        body.contains("writing a new version"),
        "the new-version arm must say so — otherwise an operator reads its \
         success as silence"
    );
}

/// The footnote list is built from the ONE authority, not assembled here.
///
/// ## What this replaces, and why the old test had to go
///
/// It used to assert that `payload.said` and `payload.admitted` appeared in a
/// hand-built list. That list is gone: `citable_sources()` is now the single
/// function the prompt's key line, the reply parser and this footnote list all
/// read, so the divergence that shipped — a citation with nothing under it — is
/// impossible by construction rather than by care.
///
/// What is left to guard is that nobody rebuilds it by hand.
#[test]
fn the_footnote_list_comes_from_the_single_authority() {
    let body = post_answer_body();

    assert!(
        body.contains("citable_sources()"),
        "the footnote list must come from `citable_sources`, the same function \
         the model's key line is built from"
    );
    assert!(
        !body.contains("payload.said") && !body.contains("payload.admitted"),
        "the sworn pair is being folded in BY HAND again — that is the second \
         list, and the second list is what disagreed: {body}"
    );
    assert!(
        !body.contains(".chain(payload.receipts.iter())"),
        "points and receipts are being assembled here again rather than taken \
         from the authority: {body}"
    );
}
