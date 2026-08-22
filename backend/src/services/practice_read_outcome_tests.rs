// The outcomes one read can have, and the row each of them writes.
//
// `read_answer` itself needs a provider, a settings snapshot and a registry, and
// is exercised on DEV. What is testable here — and what the answer row's honesty
// actually rests on — is the mapping: every arm produces a row whose columns say
// which arm it was, and no two arms produce the same row.
//
// The module had NO TESTS AT ALL before T1. These are the first.

use super::*;
use crate::services::practice_read_parse::compose_read_text;
use crate::services::practice_read_parse::Overrun;

fn parts() -> ReadParts {
    ReadParts {
        call: "You let the compound braid stand.".to_string(),
        why: "Two questions were tied into one.".to_string(),
        pointers: vec!["Take the second question only.".to_string()],
        keys: vec!["P2".to_string(), "R1".to_string()],
        ok: false,
    }
}

/// The stored don't-recall line writes a complete read and calls no model.
#[test]
fn the_stored_dont_recall_line_is_a_read_with_no_model_behind_it() {
    let outcome = ReadOutcome::stored("Fine. \"I don't recall\" is a complete answer.".to_string());
    let row = outcome.to_row();

    assert_eq!(
        row.read_text.as_deref(),
        Some("Fine. \"I don't recall\" is a complete answer.")
    );
    assert_eq!(
        row.read_ok,
        Some(true),
        "the rail goes green — it is complete"
    );
    assert_eq!(row.read_error, None);
    assert_eq!(row.read_abstain_reason, None);
    // No model, no tokens, no milliseconds — the row says the call never happened.
    assert_eq!(row.read_model, None);
    assert_eq!(row.read_input_tokens, None);
    assert_eq!(row.read_ms, None);
    // And it is STAMPED, so a later re-read can tell a stored line from a
    // judgement without re-reading the text.
    assert_eq!(row.read_version.as_deref(), Some(STORED_READ_VERSION));
    assert_eq!(row.read_attempts, None, "no model was asked");
    assert_eq!(row.read_overruns, None);
    assert_ne!(
        row.read_version.as_deref(),
        Some("practice_read_prompt_v3.md"),
        "naming a prompt file would claim a model produced a line we wrote"
    );
}

/// A load failure abstains, and the row carries BOTH halves of the reason.
///
/// This replaces the blind read. Before T1 a failed `list_points` logged an error
/// and carried on with an empty vector: the model judged her answer against a
/// question and a watch-for alone, and returned a sentence that looked exactly
/// like a good one — green rail, `read_error` NULL, nothing on the row recording
/// that the read was made blind.
#[test]
fn an_input_that_failed_to_load_abstains_and_says_so_twice() {
    let failure = PayloadFailure::Points {
        scenario_id: uuid::Uuid::nil(),
        source: anyhow::anyhow!("connection reset by peer"),
    };
    let outcome = ReadOutcome::from_payload_failure("I can't read this one.", &failure);
    let row = outcome.to_row();

    // Marie's half — she is told the read declined, not shown a blank.
    assert_eq!(row.read_text.as_deref(), Some("I can't read this one."));
    assert_eq!(
        row.read_abstain_reason.as_deref(),
        Some("her talking points could not be loaded")
    );
    // The operator's half, on the same row.
    let error = row.read_error.expect("an abstain records why");
    assert!(error.contains("her points could not be read"), "{error}");
    assert!(error.contains("connection reset by peer"), "{error}");

    // NOT a judgement: the rail must go neutral, never green or red.
    assert_eq!(row.read_ok, None);
    assert_eq!(row.read_call, None);
    assert_eq!(row.read_pointers, None);
}

/// An accepted read writes every part into its own column.
#[test]
fn an_accepted_read_writes_each_part_to_its_own_column() {
    let outcome = ReadOutcome {
        text: Some(compose_read_text(&parts())),
        ok: Some(false),
        parts: Some(parts()),
        version: Some("practice_read_prompt_v3.md".to_string()),
        model: Some("claude-opus-5".to_string()),
        input_tokens: Some(2100),
        output_tokens: Some(180),
        ms: Some(4200),
        ..Default::default()
    };
    let row = outcome.to_row();

    assert_eq!(
        row.read_call.as_deref(),
        Some("You let the compound braid stand.")
    );
    assert_eq!(
        row.read_why.as_deref(),
        Some("Two questions were tied into one.")
    );
    assert_eq!(
        row.read_pointers,
        Some(serde_json::json!(["Take the second question only."]))
    );
    assert_eq!(row.read_keys, Some(serde_json::json!(["P2", "R1"])));
    assert_eq!(
        row.read_version.as_deref(),
        Some("practice_read_prompt_v3.md")
    );

    // The composed line the untouched frontend renders is on the row too — the
    // reveal AND the question-review page both read this column.
    assert_eq!(
        row.read_text.as_deref(),
        Some("You let the compound braid stand. Take the second question only.")
    );
    // An accepted read clears the in-flight marker rather than replacing it.
    assert_eq!(row.read_error, None);
    // Nothing to keep: the parts ARE the model's own words, column by column.
    assert_eq!(row.read_raw_reply, None);
}

/// An empty `why` is stored as NULL, not as an empty string.
///
/// "The model wrote no why" and "the model wrote an empty string" are the same
/// fact and must not become two rows. A column holding `''` also reads as a
/// present-but-blank part on every screen that later renders it.
#[test]
fn an_omitted_part_is_null_rather_than_blank() {
    let outcome = ReadOutcome {
        parts: Some(ReadParts {
            why: String::new(),
            pointers: vec![],
            ..parts()
        }),
        ..Default::default()
    };
    let row = outcome.to_row();

    assert_eq!(row.read_why, None, "an empty why is NULL");
    // An empty POINTER LIST is still an array: "this read had no pointers" is a
    // fact about the read, while a NULL there would mean "this row has no read".
    assert_eq!(row.read_pointers, Some(serde_json::json!([])));
}

/// A row with no read at all has no parts columns.
#[test]
fn a_row_with_no_read_has_no_parts() {
    let row = ReadOutcome::default().to_row();
    assert_eq!(row.read_call, None);
    assert_eq!(row.read_why, None);
    assert_eq!(row.read_pointers, None, "NULL, not an empty array");
    assert_eq!(row.read_keys, None);
    assert_eq!(row.read_version, None);
}

/// No two outcomes produce the same row — Standing Rule 1, as arithmetic.
///
/// The rule says every operationally distinct state has a different observable.
/// This walks the four an answer row can hold after a read attempt and asserts
/// they are pairwise different rows, which is the rule stated in a way a build can
/// check. Before T1 two of these — an accepted read and a blind one — were
/// byte-identical.
#[test]
fn every_read_outcome_writes_a_distinguishable_row() {
    let accepted = ReadOutcome {
        text: Some("You let it stand.".to_string()),
        ok: Some(false),
        parts: Some(parts()),
        ..Default::default()
    };
    let model_abstained = ReadOutcome {
        text: Some("I can't read this one. That looks like a test entry.".to_string()),
        abstain_reason: Some("That looks like a test entry.".to_string()),
        error: Some("the model abstained: That looks like a test entry.".to_string()),
        ..Default::default()
    };
    let load_failed = ReadOutcome::from_payload_failure(
        "I can't read this one.",
        &PayloadFailure::TacticUnnamed { card: 5 },
    );
    let stored = ReadOutcome::stored("Fine. \"I don't recall\" is a complete answer.".to_string());

    let rows: Vec<String> = [&accepted, &model_abstained, &load_failed, &stored]
        .iter()
        .map(|outcome| format!("{:?}", outcome.to_row()))
        .collect();

    for (i, left) in rows.iter().enumerate() {
        for (j, right) in rows.iter().enumerate() {
            if i != j {
                assert_ne!(
                    left, right,
                    "two operationally distinct outcomes wrote the same row"
                );
            }
        }
    }
}

/// An overrun kept "as returned" says so ON THE ROW, not only in a log.
///
/// This is the half of the inverted ceiling rule that survives the log window.
/// Marie gets her coaching either way — that is the point of storing the part as
/// returned — but an operator asking "is a ceiling set wrong?" or "did the model
/// change?" next week needs the answer in the database, and the ceiling that was
/// IN FORCE at the time, because the four ceilings are settings rows somebody
/// moves.
#[test]
fn an_overrun_stored_as_returned_is_recorded_on_the_row() {
    let outcome = ReadOutcome {
        text: Some("You let it stand.".to_string()),
        ok: Some(false),
        parts: Some(parts()),
        attempts: Some(2),
        overruns: vec![Overrun {
            part: "why".to_string(),
            words: 61,
            limit: 55,
        }],
        ..Default::default()
    };
    let row = outcome.to_row();

    assert_eq!(row.read_attempts, Some(2), "this answer cost two calls");
    assert_eq!(
        row.read_overruns,
        Some(serde_json::json!([{ "part": "why", "words": 61, "limit": 55 }])),
        "the limit IN FORCE is stored beside the count, because the ceiling moves"
    );
    // And the read itself is intact — never truncated, never discarded.
    assert_eq!(
        row.read_call.as_deref(),
        Some("You let the compound braid stand.")
    );
}

/// A clean read stores no overruns at all — NULL, not an empty array.
///
/// ANTI-VACUITY for the test above, and a query-shape decision: the ordinary case
/// must not put a row into every "show me the overruns" query an operator writes.
#[test]
fn a_clean_read_stores_no_overruns() {
    let outcome = ReadOutcome {
        parts: Some(parts()),
        attempts: Some(1),
        ..Default::default()
    };
    let row = outcome.to_row();

    assert_eq!(row.read_overruns, None, "NULL, not []");
    assert_eq!(row.read_attempts, Some(1));
}

/// A one-call read and a two-call read are distinguishable rows.
///
/// The token counts on this row are the SUM across attempts, so without the
/// attempt count 4,200 input tokens is one expensive call or two ordinary ones
/// and nothing else on the row can say which. It also closes the gap the payload
/// audit named: `read_ms` has always spanned the whole retry loop with no counter
/// beside it.
#[test]
fn the_attempt_count_makes_the_token_total_readable() {
    let one = ReadOutcome {
        parts: Some(parts()),
        attempts: Some(1),
        input_tokens: Some(4200),
        ..Default::default()
    };
    let two = ReadOutcome {
        parts: Some(parts()),
        attempts: Some(2),
        input_tokens: Some(4200),
        ..Default::default()
    };
    assert_eq!(
        one.to_row().read_input_tokens,
        two.to_row().read_input_tokens
    );
    assert_ne!(
        format!("{:?}", one.to_row()),
        format!("{:?}", two.to_row()),
        "same spend, different number of calls — the rows must differ"
    );
}
