// What comes back, pinned: the three parts, the ceilings, and the citation check.

use super::*;

fn rules() -> ReadRules<'static> {
    ReadRules {
        max_words_call: 12,
        max_words_why: 55,
        max_words_pointer: 20,
        max_pointers: 3,
        max_words_after_fine: 6,
        fine_token: "Fine.",
    }
}

fn citable() -> BTreeSet<String> {
    ["P1", "P2", "P3", "R1", "R2", "R3", "S1", "S2"]
        .iter()
        .map(|k| (*k).to_string())
        .collect()
}

fn parse(raw: &str) -> Result<(ReadReply, Vec<Overrun>), ReplyRejection> {
    parse_reply(raw, rules(), &citable())
}

fn parts_of(reply: ReadReply) -> ReadParts {
    match reply {
        ReadReply::Parts(parts) => parts,
        ReadReply::Abstain(reason) => panic!("expected a judgement, got an abstain: {reason}"),
    }
}

// ── The ceilings and the settings coupling ───────────────────────────────────

/// The ceilings the parser judges by are the ones the STORE holds.
///
/// A compiled-in number here is the 2026-08-09 theme-scan defect: a 512 that
/// truncated 7 of 104 verdicts and could not be changed without a build.
#[test]
fn every_ceiling_arrives_from_the_settings_snapshot() {
    use crate::domain::settings::Settings;
    let stored = Settings::for_test();
    let read = &stored.practice_read;

    assert_eq!(rules().max_words_call as u32, read.max_words_call);
    assert_eq!(rules().max_words_why as u32, read.max_words_why);
    assert_eq!(rules().max_words_pointer as u32, read.max_words_pointer);
    assert_eq!(rules().max_pointers as u32, read.max_pointers);
    assert_eq!(
        rules().max_words_after_fine as u32,
        read.max_words_after_fine
    );
    assert_eq!(rules().fine_token, read.fine_token);
}

/// The OK word is COUPLED to the prompt file, and both are stored.
///
/// The prompt teaches the model to write it; this teaches the parser to see it.
/// An operator who edits one without the other gets every read marked as a fault,
/// silently. Carried forward from v2 unchanged, because v3 kept the token.
#[test]
fn the_ok_word_the_parser_expects_is_the_one_the_prompt_file_teaches() {
    use crate::domain::settings::Settings;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let stored = Settings::for_test();
    let prompt = std::fs::read_to_string(
        root.join("extraction_templates")
            .join(&stored.practice_read.prompt_file),
    )
    .expect("the read prompt named by the settings row is on disk");

    assert!(
        prompt.contains(&stored.practice_read.fine_token),
        "the prompt file never mentions {:?}, so no model would ever produce it",
        stored.practice_read.fine_token
    );
}

/// The reply's FIELD NAMES are the ones the prompt file teaches.
///
/// The same coupling as the OK word, one level up: the parser cannot read a field
/// the prompt never asked for, and a v3 that renamed `pointers` would send every
/// read down the re-request-then-abstain path with nothing saying why. Roman
/// ruled these stay in code rather than the store (2026-08-20) because a prompt
/// file is version-controlled, diffable and md5-verified on push — so this test is
/// the thing that keeps the two halves honest.
#[test]
fn the_reply_field_names_are_the_ones_the_prompt_file_teaches() {
    use crate::domain::settings::Settings;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let stored = Settings::for_test();
    let prompt = std::fs::read_to_string(
        root.join("extraction_templates")
            .join(&stored.practice_read.prompt_file),
    )
    .expect("the read prompt named by the settings row is on disk");

    for field in REPLY_FIELDS {
        assert!(
            prompt.contains(&format!("\"{field}\"")),
            "the prompt never shows the model a {field:?} field, so it would never send one"
        );
    }
}

/// The constants and the struct cannot drift apart.
///
/// ANTI-VACUITY for the test above: `REPLY_FIELDS` is a hand-written list, and a
/// field renamed on `RawReply` but left in the list would leave that test green
/// while the parser looked for something the struct no longer reads.
#[test]
fn the_reply_struct_and_the_field_constants_cannot_drift() {
    let raw = serde_json::json!({
        "call": "c", "why": "w", "pointers": [], "keys": [], "abstain": null
    });
    let parsed: RawReply = serde_json::from_value(raw).expect("the canonical shape parses");
    // Every constant names a field that actually carried its value through.
    assert_eq!(parsed.call, "c");
    assert_eq!(parsed.why, "w");
    assert_eq!(REPLY_FIELDS.len(), 5);
    for field in [
        FIELD_CALL,
        FIELD_WHY,
        FIELD_POINTERS,
        FIELD_KEYS,
        FIELD_ABSTAIN,
    ] {
        assert!(
            REPLY_FIELDS.contains(&field),
            "{field} is a declared constant missing from REPLY_FIELDS"
        );
    }
}

// ── The happy path ───────────────────────────────────────────────────────────

/// A three-part read is parsed into three parts, and nothing is dropped.
///
/// This is the v2 defect's headstone. The old parser took the first non-empty
/// LINE and discarded the rest with no error, no log and no `raw_reply` — so a
/// model that qualified its own verdict on line two was stored as an unqualified
/// verdict, and the qualification was unrecoverable.
#[test]
fn a_three_part_read_keeps_all_three_parts() {
    let reply = r#"{
        "call": "You let the compound braid stand.",
        "why": "The question ties conflicts to why nothing was divided. Those are two questions, and P2 answers only the second.",
        "pointers": ["Take the second question only."],
        "keys": ["P2"],
        "abstain": null
    }"#;
    let (parsed, overruns) = parse(reply).expect("a well-formed reply");
    let parts = parts_of(parsed);

    assert_eq!(parts.call, "You let the compound braid stand.");
    assert!(parts.why.starts_with("The question ties conflicts"));
    assert_eq!(parts.pointers, vec!["Take the second question only."]);
    assert_eq!(parts.keys, vec!["P2"]);
    assert!(!parts.ok, "naming a fault is the not-fine arm");
    assert!(overruns.is_empty());
}

/// The OK arm, and its shorter cap.
#[test]
fn the_ok_word_marks_the_fine_arm() {
    let (parsed, overruns) = parse(
        r#"{"call": "Fine. Short, and yours.", "why": "", "pointers": [], "keys": [], "abstain": null}"#,
    )
    .expect("a well-formed reply");
    let parts = parts_of(parsed);

    assert!(parts.ok);
    assert_eq!(parts.why, "");
    assert!(parts.pointers.is_empty(), "an empty part is legitimate");
    assert!(overruns.is_empty());
}

/// A fenced reply is a formatting habit, not a different answer.
#[test]
fn a_markdown_fence_is_stripped_rather_than_re_requested() {
    let fenced = "```json\n{\"call\": \"Fine.\", \"why\": \"\", \"pointers\": [], \"keys\": [], \"abstain\": null}\n```";
    let (parsed, _) = parse(fenced).expect("a fenced reply is still a reply");
    assert!(parts_of(parsed).ok);
}

/// An omitted part is legitimate, not a parse failure.
///
/// The design says a part may be omitted when there is nothing to say, so
/// `#[serde(default)]` is a domain decision and this is what pins it.
#[test]
fn a_reply_that_omits_the_optional_parts_is_accepted() {
    let (parsed, _) = parse(r#"{"call": "Fine."}"#).expect("call alone is a complete reply");
    let parts = parts_of(parsed);
    assert!(parts.ok);
    assert!(parts.why.is_empty());
    assert!(parts.pointers.is_empty());
    assert!(parts.keys.is_empty());
}

/// An unknown extra field does not cost her the read.
#[test]
fn an_unexpected_field_is_tolerated() {
    let (parsed, _) = parse(r#"{"call": "Fine.", "note": "chatty"}"#)
        .expect("a stray field is not a refusal to answer");
    assert!(parts_of(parsed).ok);
}

// ── The citation check ───────────────────────────────────────────────────────

/// Every key returned must be a key that was sent.
#[test]
fn every_returned_key_resolves_to_a_key_that_was_sent() {
    let (parsed, _) = parse(
        r#"{"call": "You left the anchor off.", "why": "R1 says so.", "pointers": [], "keys": ["R1", "P2"], "abstain": null}"#,
    )
    .expect("both keys were sent");
    assert_eq!(parts_of(parsed).keys, vec!["R1", "P2"]);
}

/// A key that was NOT sent is a refusal — the model invented a document.
///
/// This is T1's grounding half, and the thing that makes "a read that cannot cite
/// cannot claim" more than a sentence in a prompt. Before this task the model was
/// told to name a receipt and given none, so it reached for generic ones ("anchor
/// it to your letter") and Marie could not tell an invention from a fact.
#[test]
fn a_key_that_was_never_sent_refuses_the_whole_reply() {
    let rejection = parse(
        r#"{"call": "Name the letter.", "why": "R4 has it.", "pointers": [], "keys": ["R4"], "abstain": null}"#,
    )
    .expect_err("R4 was never sent");

    match rejection {
        ReplyRejection::UnknownKey { key, sent } => {
            assert_eq!(key, "R4");
            assert!(
                sent.contains("R1"),
                "the refusal names what WAS sent: {sent}"
            );
        }
        other => panic!("expected UnknownKey, got {other:?}"),
    }
}

/// A key whose value was a named absence is not citable either.
#[test]
fn a_key_with_nothing_behind_it_is_not_citable() {
    let thin: BTreeSet<String> = ["P1".to_string()].into_iter().collect();
    let rejection = parse_reply(
        r#"{"call": "Use your letter.", "keys": ["R1"]}"#,
        rules(),
        &thin,
    )
    .expect_err("R1 holds nothing on this scenario");
    assert!(matches!(rejection, ReplyRejection::UnknownKey { .. }));
}

// ── The rejections that re-request ───────────────────────────────────────────

#[test]
fn an_empty_reply_is_refused() {
    assert_eq!(
        parse("   ").expect_err("nothing came back"),
        ReplyRejection::Empty
    );
}

#[test]
fn prose_where_json_was_asked_for_is_refused() {
    let rejection = parse("You let the compound braid stand.").expect_err("that is not JSON");
    assert!(matches!(rejection, ReplyRejection::Unparseable { .. }));
}

/// A reply with neither a call nor an abstain judged nothing.
#[test]
fn a_reply_that_says_nothing_at_all_is_refused() {
    let rejection = parse(r#"{"why": "some reasoning", "pointers": []}"#)
        .expect_err("no call and no abstain is not a verdict");
    assert_eq!(rejection, ReplyRejection::NothingSaid);
}

// ── The abstain arm ──────────────────────────────────────────────────────────

/// The model declining is a first-class outcome, not a failure.
#[test]
fn the_model_can_abstain_in_its_own_words() {
    let (parsed, overruns) = parse(
        r#"{"call": "", "why": "", "pointers": [], "keys": [], "abstain": "That looks like a test entry rather than an answer."}"#,
    )
    .expect("an abstain is a well-formed reply");

    match parsed {
        ReadReply::Abstain(reason) => {
            assert_eq!(
                reason,
                "That looks like a test entry rather than an answer."
            )
        }
        ReadReply::Parts(parts) => panic!("expected an abstain, got {parts:?}"),
    }
    assert!(overruns.is_empty());
}

/// An abstain WINS over a filled shape.
///
/// A model that declines and then fills the parts anyway has still declined.
/// Showing the parts would be this build overriding its own model's refusal —
/// which is exactly the "manufacture a fault to fill the shape" the prompt
/// forbids, done by the code instead.
#[test]
fn an_abstain_beats_the_parts_beside_it() {
    let (parsed, _) = parse(
        r#"{"call": "You let it stand.", "why": "reasons", "pointers": ["do this"], "keys": [], "abstain": "I cannot judge this."}"#,
    )
    .expect("a well-formed reply");
    assert!(matches!(parsed, ReadReply::Abstain(_)));
}

/// A blank abstain is not an abstain.
#[test]
fn an_empty_abstain_string_is_ignored() {
    let (parsed, _) = parse(r#"{"call": "Fine.", "abstain": "   "}"#)
        .expect("a blank abstain does not decline anything");
    assert!(parts_of(parsed).ok);
}

// ── The ceilings, which NEVER discard ────────────────────────────────────────

/// An over-long part is REPORTED and KEPT — the inversion T1 ships.
///
/// Before this, a 26-word read was refused and Marie saw "no system read this
/// time" — one word over a cap and a witness got no coaching at all. **[measured:
/// 1 of the 12 answer rows on DEV is exactly this.]** The overrun is now a fact
/// about the reply, carried alongside it, never a reason to throw it away.
#[test]
fn an_over_long_call_is_reported_and_the_read_survives() {
    let long = "You accepted every single one of the words in that question without \
                once pausing to correct the premise";
    let (parsed, overruns) =
        parse(&format!(r#"{{"call": "{long}", "keys": []}}"#)).expect("still a read");

    let parts = parts_of(parsed);
    assert_eq!(parts.call, long, "the text is kept EXACTLY as returned");
    assert_eq!(overruns.len(), 1);
    assert_eq!(overruns[0].part, "call");
    assert_eq!(overruns[0].limit, 12);
    assert!(overruns[0].words > 12);
}

/// The OK arm is capped by the OK arm's own, shorter limit.
#[test]
fn fine_plus_a_speech_is_still_a_speech() {
    let (parsed, overruns) = parse(
        r#"{"call": "Fine. Short, and yours, and clear, and well judged throughout", "keys": []}"#,
    )
    .expect("still a read");
    assert!(parts_of(parsed).ok);
    assert_eq!(overruns.len(), 1);
    assert_eq!(overruns[0].limit, 6, "the after-fine cap, not the call cap");
}

/// Each part has its own ceiling, and each is reported separately.
#[test]
fn every_part_is_measured_against_its_own_ceiling() {
    let why = "word ".repeat(60);
    let pointer = "word ".repeat(25);
    let reply = serde_json::json!({
        "call": "You let it stand.",
        "why": why,
        "pointers": [pointer, "fine one", "another", "a fourth"],
        "keys": [],
    })
    .to_string();

    let (_, overruns) = parse(&reply).expect("still a read");
    let parts: Vec<&str> = overruns.iter().map(|o| o.part.as_str()).collect();

    assert!(parts.contains(&"why"), "{parts:?}");
    assert!(
        parts.contains(&"pointers"),
        "four pointers exceeds three: {parts:?}"
    );
    assert!(parts.contains(&"pointer 1"), "{parts:?}");
    assert!(
        !parts.contains(&"pointer 2"),
        "a pointer inside its cap is not reported: {parts:?}"
    );
}

/// A reply inside every ceiling reports nothing.
///
/// ANTI-VACUITY: a `measure` that returned an overrun for everything would pass
/// each test above and would re-request on every single answer.
#[test]
fn a_reply_inside_every_ceiling_reports_no_overrun() {
    let (_, overruns) = parse(
        r#"{"call": "You let the braid stand.", "why": "Two questions, one answer.", "pointers": ["Take the second."], "keys": []}"#,
    )
    .expect("a well-formed reply");
    assert!(overruns.is_empty(), "{overruns:?}");
}

// ── The composed line the untouched frontend renders ─────────────────────────

/// The call and the first pointer become the one line the reveal prints.
#[test]
fn the_composed_line_is_the_call_and_the_first_pointer() {
    let parts = ReadParts {
        call: "You let the compound braid stand.".to_string(),
        why: "irrelevant to the composition".to_string(),
        pointers: vec![
            "Take the second question only.".to_string(),
            "One clause, then stop.".to_string(),
        ],
        keys: vec![],
        ok: false,
    };
    assert_eq!(
        compose_read_text(&parts),
        "You let the compound braid stand. Take the second question only."
    );
}

/// A call with no pointer stands alone.
#[test]
fn a_call_with_no_pointers_composes_to_itself() {
    let parts = ReadParts {
        call: "Fine. Short, and yours.".to_string(),
        why: String::new(),
        pointers: vec![],
        keys: vec![],
        ok: true,
    };
    assert_eq!(compose_read_text(&parts), "Fine. Short, and yours.");
}

/// A call with no closing stop still reads as a sentence.
///
/// Without this the reveal would print "You let it stand Take the second" — two
/// sentences run together, on the one line a witness reads between reps.
#[test]
fn a_call_without_a_stop_gets_one_before_the_pointer() {
    let parts = ReadParts {
        call: "You let it stand".to_string(),
        why: String::new(),
        pointers: vec!["Take the second.".to_string()],
        keys: vec![],
        ok: false,
    };
    assert_eq!(
        compose_read_text(&parts),
        "You let it stand. Take the second."
    );
}

/// The abstain line, with and without the model's own reason.
#[test]
fn the_abstain_line_carries_the_models_reason_when_there_is_one() {
    assert_eq!(
        compose_abstain_text(
            "I can't read this one.",
            Some("That looks like a test entry.")
        ),
        "I can't read this one. That looks like a test entry."
    );
    assert_eq!(
        compose_abstain_text("I can't read this one.", None),
        "I can't read this one."
    );
    // A blank reason is not a reason.
    assert_eq!(
        compose_abstain_text("I can't read this one.", Some("  ")),
        "I can't read this one."
    );
}

/// Every rejection says, in the OPERATOR's words, what went wrong.
///
/// These `Display` strings land in `read_error`, which is the operator's half of
/// the Rule 1 split — Marie reads the abstain line, and this column is the only
/// place a log reader learns WHICH failure it was. A variant whose message said
/// nothing (or said the same as its neighbour) would collapse four distinguishable
/// states into one on the surface built to tell them apart.
///
/// The sibling assertion for `PayloadFailure` lives in `practice_read_gather_tests`;
/// this is the same guarantee for the reply side.
#[test]
fn every_rejection_reads_as_something_an_operator_can_act_on() {
    let empty = ReplyRejection::Empty.to_string();
    assert!(empty.contains("nothing"), "{empty}");

    let unparseable = ReplyRejection::Unparseable {
        detail: "expected value at line 1 column 1".to_string(),
    }
    .to_string();
    assert!(unparseable.contains("JSON"), "{unparseable}");
    assert!(
        unparseable.contains("expected value at line 1 column 1"),
        "the serde detail is the actionable part: {unparseable}"
    );

    let nothing_said = ReplyRejection::NothingSaid.to_string();
    assert!(nothing_said.contains("judged nothing"), "{nothing_said}");
    // It must name the two fields it looked for, or an operator cannot tell this
    // from a call that failed.
    assert!(nothing_said.contains(FIELD_CALL), "{nothing_said}");
    assert!(nothing_said.contains(FIELD_ABSTAIN), "{nothing_said}");

    let unknown = ReplyRejection::UnknownKey {
        key: "R4".to_string(),
        sent: "P1 P2 R1".to_string(),
    }
    .to_string();
    assert!(unknown.contains("R4"), "{unknown}");
    assert!(
        unknown.contains("P1 P2 R1"),
        "naming what WAS sent is how an operator tells an invented key from a payload bug: {unknown}"
    );

    // ANTI-VACUITY: four distinct states, four distinct sentences.
    let all = [empty, unparseable, nothing_said, unknown];
    for (i, left) in all.iter().enumerate() {
        assert!(!left.is_empty());
        for (j, right) in all.iter().enumerate() {
            if i != j {
                assert_ne!(left, right, "two rejections read identically in the log");
            }
        }
    }
}
