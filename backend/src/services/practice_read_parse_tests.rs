// Tests for `services::practice_read_parse`.
//
// Everything here is about what reaches a witness's screen. The model is a
// stranger to this case and to this witness; these are the rules that keep what
// it says inside the shape the design promised her.

use super::*;

/// The rules as the migration seeds them: 25 words, six after "Fine.".
///
/// Read from `Settings::for_test()` rather than written here, so a migration that
/// moved a cap and left this file alone fails the pins in `settings_store_tests`
/// instead of quietly testing a number the product no longer uses.
fn rules() -> ReadRules<'static> {
    // The fixture is a `&'static` fallback of the same values the store seeds;
    // `Settings::for_test()` cannot be borrowed from a temporary here.
    ReadRules {
        max_words: 25,
        max_words_after_fine: 6,
        fine_token: "Fine.",
    }
}

/// The fixture above and the settings snapshot agree.
///
/// ANTI-DRIFT. Without this, `rules()` is three numbers this file invented, and
/// every assertion below would keep passing after Roman raised the cap in
/// Settings — testing a product that no longer exists.
#[test]
fn the_fixture_rules_are_the_rules_the_store_seeds() {
    use crate::domain::settings::Settings;
    let stored = Settings::for_test();

    assert_eq!(
        u32::try_from(rules().max_words).unwrap(),
        stored.practice_read.max_words
    );
    assert_eq!(
        u32::try_from(rules().max_words_after_fine).unwrap(),
        stored.practice_read.max_words_after_fine
    );
    assert_eq!(rules().fine_token, stored.practice_read.fine_token);
}

/// The OK word is COUPLED to the prompt file, and both are stored.
///
/// The prompt teaches the model to write it; this teaches the parser to see it.
/// An operator who edits one without the other gets every read marked as a fault,
/// silently. This test is the only place that connection is written down where a
/// build can check it.
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

#[test]
fn a_one_sentence_read_naming_a_tactic_is_accepted_and_marked_not_fine() {
    let read = parse_read(
        "You accepted the words \"at each other's throats\" — the false premise. Correct it once, then stop.",
        rules(),
    )
    .expect("a 15-word read");

    assert!(!read.ok, "naming a tactic is the not-fine arm");
    assert!(read.text.starts_with("You accepted"));
}

#[test]
fn fine_alone_and_fine_with_a_short_tail_are_both_the_ok_arm() {
    assert_eq!(
        parse_read("Fine.", rules()),
        Ok(ReadLine {
            text: "Fine.".to_string(),
            ok: true
        })
    );

    let read = parse_read("Fine. Short, and yours.", rules()).expect("three words after Fine.");
    assert!(read.ok);
    assert_eq!(read.text, "Fine. Short, and yours.");
}

/// A model that wrote a paragraph gets NO read, not a truncated one.
///
/// This is the rule with the sharpest consequence. Half a sentence about
/// testimony can invert its meaning, and the screen has an honest way to say
/// nothing — the same line it shows when the model is unreachable.
#[test]
fn a_paragraph_is_refused_rather_than_truncated() {
    let long = "You accepted a premise you should not have accepted, and then you went on \
                to explain a great deal about your sisters and the auction and the letter, \
                none of which anybody asked you about at any point.";
    match parse_read(long, rules()) {
        Err(ReadRejection::TooLong { words, limit }) => {
            assert_eq!(limit, 25);
            assert!(words > 25, "{words}");
        }
        other => panic!("a paragraph must be refused: {other:?}"),
    }

    // And the same cap applies to the friendly arm, with its own shorter limit —
    // "Fine." followed by a speech is still a speech.
    match parse_read(
        "Fine. That was really very good indeed and exactly what Chuck wants.",
        rules(),
    ) {
        Err(ReadRejection::TooLong { limit, .. }) => assert_eq!(limit, 6),
        other => panic!("a long tail after Fine. must be refused: {other:?}"),
    }
}

/// A refusal SAYS which refusal it was.
///
/// `read_error` is this string, and it is the column an operator queries when
/// every read in a session came back empty. "the model replied with nothing" and
/// "the read was 42 words" send them to two entirely different problems — a dead
/// endpoint against a prompt that stopped working — so the two must never
/// collapse into one message.
#[test]
fn each_refusal_says_which_refusal_it_was() {
    assert_eq!(
        ReadRejection::Empty.to_string(),
        "the model replied with nothing"
    );

    let long = ReadRejection::TooLong {
        words: 42,
        limit: 25,
    }
    .to_string();
    assert!(long.contains("42 words"), "{long}");
    assert!(long.contains("limit is 25"), "{long}");
    assert!(
        long.contains("refused rather than truncated"),
        "the message must say the read was WITHHELD, not shortened: {long}"
    );
}

/// The boundary is INCLUSIVE, so a read of exactly the cap survives.
///
/// Worth pinning because an off-by-one here would silently discard the longest
/// legitimate reads — the ones that name the tactic AND the counter, which are
/// the most useful ones there are.
#[test]
fn a_read_of_exactly_the_cap_is_accepted() {
    let twenty_five = (1..=25)
        .map(|n| format!("w{n}"))
        .collect::<Vec<_>>()
        .join(" ");
    assert!(parse_read(&twenty_five, rules()).is_ok());

    let twenty_six = (1..=26)
        .map(|n| format!("w{n}"))
        .collect::<Vec<_>>()
        .join(" ");
    assert!(parse_read(&twenty_six, rules()).is_err());
}

/// Only the first line survives, and a reply of nothing is nothing.
#[test]
fn only_the_first_line_survives_and_an_empty_reply_is_refused() {
    let read = parse_read(
        "\n\n  Fine.  \nAnd here is a second thought nobody asked for.\n",
        rules(),
    )
    .expect("the first non-empty line");
    assert_eq!(read.text, "Fine.");

    assert_eq!(parse_read("", rules()), Err(ReadRejection::Empty));
    assert_eq!(parse_read("   \n  \n", rules()), Err(ReadRejection::Empty));
    assert_eq!(parse_read("\"\"", rules()), Err(ReadRejection::Empty));
}

/// Quotation marks a model wrapped the line in are stripped, not shown.
#[test]
fn wrapping_quotes_are_stripped_from_either_style() {
    assert_eq!(
        parse_read("\"Fine.\"", rules())
            .expect("straight quotes")
            .text,
        "Fine."
    );
    assert_eq!(
        parse_read("\u{201c}Fine.\u{201d}", rules())
            .expect("curly quotes")
            .text,
        "Fine."
    );
}

/// The message carries her answer, her points and the ALWAYS card — and NOTHING
/// about the case beyond them.
///
/// ## Why the absence assertions are the important half
///
/// "Nothing reads the whole graph" is a design law, and an LLM input is the one
/// place it could be broken without a query appearing anywhere: a helpful edit
/// that added "the scenario's included facts" to this message would leak the
/// pool the drill is explicitly built to keep off the screen.
#[test]
fn the_user_message_carries_the_answer_the_points_and_the_card_and_no_more() {
    let points = vec![
        "I asked in writing to divide Dad's things.".to_string(),
        "They admitted they got my letter.".to_string(),
    ];
    let message = build_user_message(&ReadInputs {
        question: "Weren't you at each other's throats?",
        tactic: Some("false premise"),
        side: "George's side",
        kind: "cross",
        answer: "Well, we did argue.",
        points: &points,
        watch_for: Some("WATCH FOR — the characterization."),
        always: "Tell the truth · Answer only what's asked",
    });

    for expected in [
        "Weren't you at each other's throats?",
        "false premise",
        "Well, we did argue.",
        "1. I asked in writing to divide Dad's things.",
        "2. They admitted they got my letter.",
        "WATCH FOR — the characterization.",
        "Tell the truth · Answer only what's asked",
    ] {
        assert!(
            message.contains(expected),
            "missing {expected} in:\n{message}"
        );
    }
}

/// A Chuck question has no tactic, and the message says so rather than leaving a
/// blank the model will fill in for itself.
///
/// A prompt line reading "THE TACTIC:" with nothing after it invites the model to
/// invent one — and a read that names a trap in a friendly question is worse than
/// no read at all.
#[test]
fn a_question_with_no_tactic_says_so_rather_than_leaving_the_line_blank() {
    let message = build_user_message(&ReadInputs {
        question: "Did you ever refuse to divide the property?",
        tactic: None,
        side: "Chuck",
        kind: "direct",
        answer: "No.",
        points: &[],
        watch_for: None,
        always: "Tell the truth",
    });

    assert!(message.contains("THE TACTIC: none — this is a direct question"));
    assert!(message.contains("(no watch-for was written for this question)"));
    assert!(message.contains("(none recorded)"));
}
