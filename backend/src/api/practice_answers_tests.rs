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

/// The READ module's source — `read_and_attach` and `read_for` live there.
///
/// ## Why a second file to read (2026-09-15)
///
/// The Answer-analysis switch took `practice_answers.rs` past the 300-line limit,
/// and the seam Rule 17 forced is the honest one: that module ROUTES, and
/// `practice_answer_read` is the one thing that happens at one of its addresses.
/// The claims below did not change when the code moved; only where they are read
/// from did, which is the standing cost of a scanner and is stated at the top of
/// this file.
fn read_module() -> String {
    without_comments(
        &std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("src/api/practice_answer_read.rs"),
        )
        .expect("the read module is on disk"),
    )
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
    // The attach moved into `read_and_attach` on 2026-09-15; the CLAIM is
    // unchanged — the re-read path must still reach it, and it must still
    // attach. Asserted in two halves because it now spans two modules, and
    // either half alone would be satisfied by a path that called nothing.
    assert!(
        after.contains("read_and_attach("),
        "the re-read path must still reach the read — otherwise pressing Answer \
         on unchanged text runs nothing and shows her the last critique: {after}"
    );
    assert!(
        read_module().contains("attach_read("),
        "the read is no longer attached to any row — a critique that is computed \
         and never written is one nobody sees twice"
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
    // The read moved one module along (Rule 17, 2026-09-15). Same claim, same
    // words, read from where the code now is.
    let body = read_module();

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

// =============================================================================
// The Answer-analysis switch: off means NO MODEL WAS ASKED
// =============================================================================
//
// CC_TASK_PRACTICE_POLISH_v1 item 3. The ruling is absolute — "off ⇒ the read is
// NEVER requested — no model call, nothing in flight" — and it cannot be kept by
// the browser alone: saving an answer and reading it are ONE request, so a
// browser that wants no read still has to make the call that would produce one.
// The wish therefore travels with the answer and the handler obeys it, and these
// pin that it does.
//
// Same limit as every scan above: the shape, not the behaviour. A database tier
// would assert the row instead.

/// The handler function ALONE — signature to its own closing brace.
///
/// ## Why not [`post_answer_body`]
///
/// That one runs to the next `pub async fn`, so it also swallows the private
/// helpers declared after the handler — `read_and_attach` and `read_for` among
/// them. That is what its own assertions want (they are about what those helpers
/// do). These assertions are about WHERE a call sits relative to a branch, and a
/// helper's own definition inside the slice would satisfy "the call is present"
/// while proving nothing about the branch.
fn post_answer_fn() -> String {
    let source = handler();
    let from = source
        .find("pub async fn post_practice_answer(")
        .expect("the answer handler is declared");
    let rest = &source[from..];
    // The first line that is a brace in column zero closes the function: this
    // file's own style puts every nested brace under at least four spaces.
    let to = rest.find("\n}").map(|i| i + 2).unwrap_or(rest.len());
    without_comments(&rest[..to])
}

/// The model is asked ONLY on the branch where a read was wanted.
///
/// The one assertion this whole feature rests on. A `read_and_attach` call that
/// drifted above the `if` — a refactor, a merge — would restore the model call
/// for every answer while every other test in this repository stayed green, and
/// the only visible symptom would be a bill.
#[test]
fn the_model_is_asked_only_when_a_read_was_requested() {
    let body = post_answer_fn();

    let branch = body
        .find("if body.want_read {")
        .expect("the switch must be a branch in the handler, not a filter elsewhere");
    let calls: Vec<usize> = body
        .match_indices("read_and_attach(")
        .map(|(i, _)| i)
        .collect();

    assert_eq!(
        calls.len(),
        1,
        "the read is reached from exactly one place, or the branch above it \
         guards only one of several doors: {body}"
    );
    assert!(
        calls[0] > branch,
        "the read is asked for BEFORE the switch is consulted — off would then \
         mean a model call whose answer is thrown away: {body}"
    );
    assert!(
        body.contains("ReadOutcome::not_requested()"),
        "the off arm must return the NAMED no-read outcome, so the response \
         carries no verdict and the row's reason has a name: {body}"
    );
}

/// Nothing is attached to an existing row when nobody asked for a read.
///
/// ## ⚑ The defect this exists to prevent
///
/// Pressing Answer on unchanged text re-uses the standing answer row. If the off
/// arm attached anything to it, Marie would lose a critique she already had —
/// silently, and as a side effect of a switch about FUTURE reads. The attach
/// lives inside `read_and_attach`, which the off arm never reaches; this asserts
/// the handler itself never attaches.
#[test]
fn the_off_arm_cannot_overwrite_a_read_that_already_stands() {
    let body = post_answer_fn();

    assert!(
        !body.contains("attach_read("),
        "the handler attaches a read directly again — the off arm must be \
         unable to reach any write to an existing row: {body}"
    );
}

/// The two "no read yet" markers are chosen by the switch, at INSERT time.
///
/// Standing Rule 1: a row nobody asked a question about must not wear the marker
/// that means "a model is being asked right now", which is also the shape of a
/// backend that died mid-read.
#[test]
fn a_new_row_records_which_kind_of_no_read_it_is() {
    let body = post_answer_fn();

    assert!(
        body.contains("READ_IN_FLIGHT"),
        "the in-flight marker is gone from the insert: {body}"
    );
    assert!(
        body.contains("READ_NOT_REQUESTED"),
        "a row written with the switch off must say nobody asked, not that a \
         model is being asked: {body}"
    );
}

/// An absent `want_read` means what it meant before the field existed.
///
/// `#[serde(default)]` on a `bool` yields FALSE, which would turn the read off
/// for every caller that predates the switch — a behaviour change nobody asked
/// for, delivered silently. The named default says `true`.
#[test]
fn a_request_that_does_not_mention_the_switch_still_gets_its_read() {
    let dto =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/dto/practice.rs"))
            .expect("the practice DTOs are on disk");
    let dto = without_comments(&dto);

    assert!(
        dto.contains("#[serde(default = \"want_read_by_default\")]"),
        "want_read must default through a NAMED function — bare #[serde(default)] \
         on a bool is false, which silently retires the read for old callers"
    );
    assert!(
        dto.contains("fn want_read_by_default() -> bool {\n    true\n}"),
        "the default must be TRUE: absent means what it meant yesterday"
    );

    // A request the browser sends with the switch ON decodes as a read wanted,
    // and one with it OFF decodes as no read — the serde contract itself, not a
    // scan of it.
    let asked: crate::dto::practice::AnswerRequest = serde_json::from_str(
        r#"{"session_id":"00000000-0000-0000-0000-000000000000",
             "question_id":"00000000-0000-0000-0000-000000000000",
             "answer_text":"I filed it.","dont_recall":false,"want_read":true}"#,
    )
    .expect("a request naming the switch decodes");
    assert!(asked.want_read);

    let declined: crate::dto::practice::AnswerRequest = serde_json::from_str(
        r#"{"session_id":"00000000-0000-0000-0000-000000000000",
             "question_id":"00000000-0000-0000-0000-000000000000",
             "answer_text":"I filed it.","dont_recall":false,"want_read":false}"#,
    )
    .expect("a request declining the read decodes");
    assert!(!declined.want_read);

    let silent: crate::dto::practice::AnswerRequest = serde_json::from_str(
        r#"{"session_id":"00000000-0000-0000-0000-000000000000",
             "question_id":"00000000-0000-0000-0000-000000000000",
             "answer_text":"I filed it.","dont_recall":false}"#,
    )
    .expect("a request that predates the switch decodes");
    assert!(
        silent.want_read,
        "a caller that never heard of the switch must still get its read"
    );
}
