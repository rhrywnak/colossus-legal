// What the answer-write half guarantees, pinned against its own source.
//
// ## Why this is a sibling file and not a `mod tests` inside the module
//
// `sql_invariants` and `practice_sql_shape` both read source TEXT. A test helper
// that spells a SQL keyword is indistinguishable to them from a statement, and
// putting these tests inline made three real guards fail on a phantom INSERT with
// no column list. Out here the strings are strings.

/// This module's own source, read from disk.
///
/// ## Why the SQL is read from the FILE and not hoisted to a `const`
///
/// A `const` was the obvious way to make these statements testable, and it
/// was wrong: the repo-wide arity scanner (`sql_invariants`) reads CALL SITES
/// and counts the placeholders in the literal it finds there. Behind a const
/// name it finds none, so both of these statements would have been silently
/// exempted from the guard that catches a bind landing in the wrong column —
/// trading a runtime-only defect for a passing test. Reading the file keeps
/// both guards live, and it is the technique three other tests in this tree
/// already use for exactly this reason.
fn source() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/repositories/pipeline_repository/practice_answers.rs");
    std::fs::read_to_string(path).expect("this module's source is on disk")
}

/// The text of one statement, from its opening keyword to the closing quote.
fn statement(starting: &str) -> String {
    let src = source();
    let at = src
        .find(starting)
        .unwrap_or_else(|| panic!("no statement starting {starting:?}"));
    let rest = &src[at..];
    let end = rest.find("\",\n    )").unwrap_or(rest.len());
    rest[..end].to_string()
}

/// Every column that records something a HUMAN did.
///
/// Not a stylistic list: these are the columns the hard constraint protects —
/// "no existing answer, note, flag or change-log row destroyed or rewritten"
/// — while Marie and Roman practise on this deck all week.
const HERS: &[&str] = &[
    "answer_text",
    "dont_recall",
    "self_check",
    "points_to",
    "question_text",
    "mark",
];

/// **A read can never rewrite her answer.**
///
/// The structural half of "the answer survives a failed read". The ordering
/// half — that the row is committed BEFORE the model is called — lives in the
/// handler and needs a database to observe; this needs neither, and it is the
/// guarantee that actually protects her words. A read that goes wrong in any
/// way whatsoever cannot touch a column in `HERS`, because the statement does
/// not name one.
///
/// The failure this prevents is not hypothetical: `attach_read` takes an
/// `AnswerRead` built from a model reply, and one `answer_text = $N` added to
/// that SET clause during a later feature would let a vendor's output
/// overwrite a witness's testimony, with nothing in review to catch it.
#[test]
fn a_read_can_never_rewrite_her_answer() {
    let update = statement("UPDATE practice_answers SET");
    for column in HERS {
        assert!(
            !update.contains(column),
            "attach_read names `{column}`, which records something Marie did. \
             A read must never be able to write it.\n{update}"
        );
    }
    // ANTI-VACUITY: prove the statement was actually found and read.
    assert!(update.contains("read_text = $2"), "{update}");
    // The LAST placeholder, so this anti-vacuity check tracks the real end of
    // the statement rather than a point somewhere in its middle.
    assert!(update.contains("read_overruns = $17"), "{update}");
}

/// The row is OPENED with no read on it.
///
/// The two-write shape is only worth its extra statement if the first write
/// genuinely carries no verdict. A `read_text` back in this INSERT would mean
/// the model had been called before the answer was safe — the shape T1 exists
/// to leave behind — and it would compile and pass every other test.
#[test]
fn the_answer_row_is_opened_before_any_read_exists() {
    let insert = statement("INSERT INTO practice_answers");
    for column in [
        "read_text",
        "read_ok",
        "read_call",
        "read_why",
        "read_pointers",
        "read_keys",
        "read_version",
        "read_model",
        "read_raw_reply",
    ] {
        assert!(
            !insert.contains(column),
            "insert_answer names `{column}` — the row must open with no read\n{insert}"
        );
    }
    // `read_error` IS named, and that is the point: it carries the in-flight
    // marker, so "no read yet" is never a silent blank.
    assert!(insert.contains("read_error"), "{insert}");
    assert!(insert.contains("answer_text"), "{insert}");
}
