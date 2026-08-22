// What the model is told, pinned — including what it is NOT told.
//
// ## Why the negative assertions are whole-body equality and not `!contains`
//
// The audit found the previous guard here
// (`the_user_message_carries_the_answer_the_points_and_the_card_and_no_more`) was
// eight `assert!(message.contains(…))` calls under a name promising "and no
// more". It would have passed unchanged if somebody added a case summary, the
// sworn pair, the whole scenario definition — or the STRONGER ANSWER, which sits
// on the very record the gatherer holds.
//
// A `!contains` list is only better by the length of the list: it catches the
// leaks somebody thought of. Comparing the WHOLE BODY catches every leak,
// including the ones nobody thought of, because a field added to the message has
// nowhere to hide in a string equality.

use super::*;

/// An S-5 cross question, with everything present. Values are the shapes of the
/// real ones — **[measured 2026-08-20 from scenario aecbaf77 on DEV]** — so a
/// reader can compare a failure here against the live payload.
fn cross_payload() -> ReadPayload {
    ReadPayload {
        question: "There were multiple contacts among you and your sisters, correct?".to_string(),
        side: "the defense".to_string(),
        kind: "cross".to_string(),
        tactic: Tactic::Named("compound".to_string()),
        answer: "Nothing stopped them from dividing it.".to_string(),
        points: vec![
            Keyed::new(
                "P1",
                Some("I asked in writing to divide Dad's things.".to_string()),
            ),
            Keyed::new("P2", Some("They admitted they got my letter.".to_string())),
            Keyed::new("P3", Some("I never refused anything.".to_string())),
        ],
        receipts: vec![
            Keyed::new("R1", Some("your certified letter, 16 Nov 2009".to_string())),
            Keyed::new("R2", Some("CFS Interrogatory Response, p. 10".to_string())),
            Keyed::new("R3", Some("CFS Interrogatory Response, p. 14".to_string())),
        ],
        said: Some("…multiple contacts… — Phillips, discovery response, p. 5".to_string()),
        admitted: Some("…there were no such conversations. — CFS, sworn answer, p. 5".to_string()),
        points_to: PointsTo::NeverOpened,
        watch_for: Some("WATCH FOR — two things braided.".to_string()),
        always: "Tell the truth · Answer only what's asked".to_string(),
    }
}

/// A Chuck DIRECT question: no tactic, no sworn pair. **[measured: this is 20 of
/// the 30 live deck rows — every question Chuck asks.]**
fn direct_payload() -> ReadPayload {
    ReadPayload {
        question: "Marie, what did you do when the estate had to be divided?".to_string(),
        side: "Chuck".to_string(),
        kind: "direct".to_string(),
        tactic: Tactic::NoneByDesign,
        said: None,
        admitted: None,
        ..cross_payload()
    }
}

/// THE WHOLE MESSAGE, for a cross question with everything present.
///
/// If this fails because a field was ADDED, that is the test doing its job: read
/// the diff and decide whether the model should have been given that. It is the
/// one place in this build where a leak from the graph or the corpus into an LLM
/// input would show up without a query appearing anywhere.
#[test]
fn the_payload_is_exactly_this_and_nothing_else() {
    let expected = "\
THE QUESTION (the defense): There were multiple contacts among you and your sisters, correct?
THE KIND: cross
THE TACTIC: compound

HER ANSWER, verbatim:
Nothing stopped them from dividing it.

HER THREE POINTS:
P1. I asked in writing to divide Dad's things.
P2. They admitted they got my letter.
P3. I never refused anything.

THE RECEIPTS BEHIND HER POINTS:
R1. your certified letter, 16 Nov 2009
R2. CFS Interrogatory Response, p. 10
R3. CFS Interrogatory Response, p. 14

WHAT THEY SAID:
S1. …multiple contacts… — Phillips, discovery response, p. 5

WHAT THEY ADMITTED UNDER OATH:
S2. …there were no such conversations. — CFS, sworn answer, p. 5

WHAT SHE SAID SHE WOULD POINT TO: (she did not open the exhibit list)

THE WATCH-FOR: WATCH FOR — two things braided.

THE ALWAYS CARD: Tell the truth · Answer only what's asked

THE KEYS YOU MAY CITE: P1 P2 P3 R1 R2 R3 S1 S2
";
    assert_eq!(build_user_message(&cross_payload()), expected);
}

/// The model answer is NOT in the payload.
///
/// `stronger` and `stronger_lean` are columns on the very record the gatherer
/// reads — `PracticeQuestionRecord` carries both — and the reveal renders them in
/// its collapsed drawer. A model judging her attempt must not be able to see the
/// answer she was attempting. Design §8 retires them from the payload by name.
///
/// This is a second assertion over the same string the test above pins, and it
/// earns its place by NAMING the thing: a whole-body equality that fails tells
/// you something changed, and this tells you what must never come back.
#[test]
fn the_stronger_answer_never_reaches_the_model() {
    let mut payload = cross_payload();
    // The real S-5 g4 values, verbatim.
    let stronger = "Nothing stopped them from dividing it — I asked in writing, \
                    and they've admitted they got my letter.";
    let lean = "leans on points 1 and 2";
    payload.answer = "my own words".to_string();

    let message = build_user_message(&payload);
    assert!(
        !message.contains(stronger),
        "the stored stronger answer leaked into the payload"
    );
    assert!(
        !message.contains(lean),
        "stronger_lean leaked into the payload"
    );
    // And the structural reason it cannot: there is no field for it.
    assert!(!message.contains("STRONGER"));
}

/// A direct question — no tactic, no pair — is a COMPLETE payload, not a broken one.
///
/// This is the row of Roman's §4 table that would have cost the most: read as a
/// load failure, every question Chuck asks would abstain on every answer,
/// permanently. Two thirds of the live deck.
#[test]
fn a_direct_question_with_no_tactic_and_no_pair_is_still_a_full_payload() {
    let message = build_user_message(&direct_payload());

    assert!(
        message.contains("THE TACTIC: none — this question carries no tactic"),
        "the absence must be SAID, not omitted: {message}"
    );
    assert!(
        message.contains("S1. (no sworn pair is recorded for this question)"),
        "a missing pair is a named absence: {message}"
    );
    assert!(
        message.contains("S2. (no sworn pair is recorded for this question)"),
        "both halves are named: {message}"
    );
    // Her points and receipts are still there — the answer is fully judgeable.
    assert!(message.contains("P1. I asked in writing"));
    assert!(message.contains("R1. your certified letter, 16 Nov 2009"));
}

/// A REDIRECT question is the same shape. One test per kind, per the task.
///
/// Not redundant with the direct case: `kind` is sent as itself precisely because
/// `side` cannot tell Chuck's two apart, and the three kinds are judged by
/// different rules. A payload that silently collapsed redirect into direct would
/// pass the test above.
#[test]
fn a_redirect_question_with_no_tactic_and_no_pair_is_still_a_full_payload() {
    let payload = ReadPayload {
        kind: "redirect".to_string(),
        ..direct_payload()
    };
    let message = build_user_message(&payload);

    assert!(message.contains("THE KIND: redirect"));
    assert!(message.contains("THE TACTIC: none — this question carries no tactic"));
    assert!(message.contains("S1. (no sworn pair is recorded for this question)"));
}

/// Half a sworn pair sends the half that exists (Roman, A3).
///
/// The seed writes both or neither and no constraint enforces it. When one is
/// missing anyway, the answer is still judgeable against the half that is there —
/// so the payload carries it under its own key rather than discarding both or
/// abstaining. **[measured: 0 such rows on DEV today.]**
#[test]
fn half_a_sworn_pair_sends_the_half_that_exists() {
    let payload = ReadPayload {
        admitted: None,
        ..cross_payload()
    };
    let message = build_user_message(&payload);

    assert!(
        message.contains("S1. …multiple contacts…"),
        "the half that exists is sent: {message}"
    );
    assert!(
        message.contains("S2. (no sworn pair is recorded for this question)"),
        "the half that does not is named absent: {message}"
    );
    // And S2 is not citable, because there is nothing behind it to cite.
    assert!(payload.citable_keys().contains("S1"));
    assert!(!payload.citable_keys().contains("S2"));
}

/// A point with no receipt is printed AND is not citable.
///
/// Two rules in one, and they pull in opposite directions on purpose: the absence
/// is SAID (the honest-gap law — never a blank line under a point), and the key is
/// withheld from the citable set, because a model that cites `R2` when R2 holds
/// nothing is claiming a document exists. That is the invention this whole task
/// was written to stop.
#[test]
fn a_point_with_no_receipt_is_named_absent_and_is_not_citable() {
    let mut payload = cross_payload();
    payload.receipts[1] = Keyed::new("R2", None);

    let message = build_user_message(&payload);
    assert!(
        message.contains("R2. (none recorded)"),
        "the gap is named: {message}"
    );
    assert!(
        message.contains("THE KEYS YOU MAY CITE: P1 P2 P3 R1 R3 S1 S2"),
        "R2 must not be offered as citable: {message}"
    );
}

/// No points authored is a named absence and NOT an abstain (Roman, A2).
#[test]
fn a_scenario_with_no_points_still_produces_a_payload() {
    let payload = ReadPayload {
        points: Vec::new(),
        receipts: Vec::new(),
        ..cross_payload()
    };
    let message = build_user_message(&payload);

    assert!(message.contains("HER THREE POINTS:\n(none recorded)"));
    assert!(message.contains("THE RECEIPTS BEHIND HER POINTS:\n(none recorded)"));
    // The pair is still citable — the read is not blind, it is thinner.
    assert_eq!(
        payload.citable_keys().into_iter().collect::<Vec<_>>(),
        vec!["S1".to_string(), "S2".to_string()]
    );
}

/// "Never opened" and "opened and picked nothing" are two different sentences.
///
/// The column keeps them apart and so must the payload. Collapsing them tells the
/// model she considered the exhibits and reached for none, on an answer where she
/// never saw the list — a claim about her judgement, invented out of a NULL.
#[test]
fn the_two_points_to_absences_are_different_sentences() {
    let never = build_user_message(&ReadPayload {
        points_to: PointsTo::NeverOpened,
        ..cross_payload()
    });
    let nothing = build_user_message(&ReadPayload {
        points_to: PointsTo::OpenedAndPickedNothing,
        ..cross_payload()
    });
    let picked = build_user_message(&ReadPayload {
        points_to: PointsTo::Picked(vec![
            "your certified letter, 16 Nov 2009".to_string(),
            "CFS Interrogatory Response, p. 10".to_string(),
        ]),
        ..cross_payload()
    });

    assert!(never.contains("WHAT SHE SAID SHE WOULD POINT TO: (she did not open the exhibit list)"));
    assert!(nothing.contains(
        "WHAT SHE SAID SHE WOULD POINT TO: (she opened the exhibit list and picked nothing)"
    ));
    assert!(picked.contains(
        "WHAT SHE SAID SHE WOULD POINT TO: your certified letter, 16 Nov 2009 · CFS Interrogatory Response, p. 10"
    ));
    assert_ne!(never, nothing, "two facts must not render as one string");
}

/// A missing watch-for is a named absence — the column withdraws the box, and a
/// withdrawn box is not a failure.
#[test]
fn a_question_with_no_watch_for_is_still_read() {
    let message = build_user_message(&ReadPayload {
        watch_for: None,
        ..cross_payload()
    });
    assert!(message.contains("THE WATCH-FOR: (no watch-for was written for this question)"));
}

/// The citable set is exactly what has something behind it.
#[test]
fn only_keys_with_something_behind_them_are_citable() {
    let payload = ReadPayload {
        points: vec![
            Keyed::new("P1", Some("a point".to_string())),
            Keyed::new("P2", None),
        ],
        receipts: vec![Keyed::new("R1", None)],
        said: Some("they said".to_string()),
        admitted: None,
        ..cross_payload()
    };
    assert_eq!(
        payload.citable_keys().into_iter().collect::<Vec<_>>(),
        vec!["P1".to_string(), "S1".to_string()]
    );
}

/// The key line is stable across runs.
///
/// It is the same set twice — the line the model is shown and the set the parser
/// validates against — so an unstable order would make two identical payloads
/// produce two different prompts, and any diff of them unreadable.
#[test]
fn the_citable_key_line_is_sorted_and_stable() {
    let payload = cross_payload();
    let once = build_user_message(&payload);
    let twice = build_user_message(&payload);
    assert_eq!(once, twice);
    assert!(once.contains("THE KEYS YOU MAY CITE: P1 P2 P3 R1 R2 R3 S1 S2"));
}
