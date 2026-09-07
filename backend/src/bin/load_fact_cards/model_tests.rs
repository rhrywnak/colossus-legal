// Tests for the loader's input model.
//
// The two caps and the JSONL reader. What matters most is that a file produced
// against a DIFFERENT rule stops the load rather than being quietly truncated —
// these files are the record of what a model decided, and a loader that edits
// them on the way in makes that record untrue.

use super::*;
use std::io::Write;

fn card(title: &str, supports: usize) -> DraftedCard {
    DraftedCard {
        title: title.to_string(),
        backs: None,
        supports: (0..supports)
            .map(|n| SupportEntry {
                allegation_id: format!("a{n}"),
                stance: CardStance::Supports,
            })
            .collect(),
        watch_out: None,
        answer_draft: None,
        card_id: "doc-x:evidence:8a240a75".to_string(),
        c_code: Some("C98".to_string()),
        over_supports_cap: false,
        dropped_not_in_list: Vec::new(),
        date: None,
        speaker: None,
    }
}

/// A title within the limit is returned unchanged.
#[test]
fn a_short_enough_title_passes() {
    let c = card("The court ordered the $50,000 back to the estate", 0);
    assert_eq!(
        c.checked_title(14).expect("eight words is under fourteen"),
        "The court ordered the $50,000 back to the estate"
    );
}

/// A title at EXACTLY the limit passes — the cap is a maximum, not a boundary to
/// fall short of.
#[test]
fn a_title_at_the_limit_passes() {
    let fourteen =
        "one two three four five six seven eight nine ten eleven twelve thirteen fourteen";
    assert!(card(fourteen, 0).checked_title(14).is_ok());
}

/// A STANDALONE EM DASH IS NOT A WORD.
///
/// The real `B_S11.jsonl` carries "It came back to the estate — never to my
/// father, who asked for it": fourteen words and one dash. Counting the dash
/// refused a real file on the first run, over a counting artifact rather than
/// anything Job B did.
#[test]
fn punctuation_between_words_is_not_counted() {
    let with_dash = "It came back to the estate — never to my father, who asked for it";
    assert_eq!(word_count(with_dash), 14);
    assert!(card(with_dash, 0).checked_title(14).is_ok());
}

/// Hyphenated words and possessives count ONCE, which is what a reader would say.
#[test]
fn a_hyphenated_word_counts_once() {
    assert_eq!(word_count("court-ordered"), 1);
    assert_eq!(word_count("Dad's money"), 2);
    assert_eq!(word_count("$50,000 returned"), 2);
}

/// A title of nothing but punctuation counts as no words.
#[test]
fn punctuation_alone_counts_as_no_words() {
    assert_eq!(word_count("— · …"), 0);
}

/// A title past the limit is REFUSED, and the refusal names the card, the count
/// and the words.
///
/// Truncating would store a sentence Job B did not write, on a card a witness
/// reads aloud.
#[test]
fn a_long_title_is_refused_and_names_the_card() {
    let fifteen =
        "one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen";
    let err = card(fifteen, 0)
        .checked_title(14)
        .expect_err("fifteen words exceeds fourteen");
    let message = format!("{err}");
    assert!(message.contains("8a240a75"), "names the card: {message}");
    assert!(message.contains("15-word"), "names the count: {message}");
    assert!(message.contains("14"), "names the limit: {message}");
}

/// Two accusations pass; three are refused by name.
///
/// Job B was given the same cap and reports `over_supports_cap` when it dropped
/// one. A file that exceeds it was produced against a different rule, which is
/// worth stopping for.
#[test]
fn the_supports_cap_is_enforced_on_the_way_in() {
    assert_eq!(
        card("t", 2).checked_supports().expect("two is fine").len(),
        2
    );
    let err = card("t", 3)
        .checked_supports()
        .expect_err("three exceeds the cap");
    let message = format!("{err}");
    assert!(message.contains("8a240a75"), "{message}");
    assert!(message.contains('3'), "{message}");
}

/// An empty list is fine — most cards name no accusation.
#[test]
fn a_card_may_name_no_accusation() {
    assert!(card("t", 0)
        .checked_supports()
        .expect("none is fine")
        .is_empty());
}

/// The reader parses one struct per line and skips blank ones.
#[test]
fn the_reader_parses_one_record_per_line() {
    let mut file = tempfile::NamedTempFile::new().expect("a temp file");
    writeln!(file, r#"{{"title":"first","card_id":"a","supports":[]}}"#).expect("write");
    writeln!(file).expect("write");
    writeln!(file, r#"{{"title":"second","card_id":"b","supports":[]}}"#).expect("write");

    let cards: Vec<DraftedCard> = read_jsonl(file.path()).expect("the file parses");
    assert_eq!(cards.len(), 2, "the blank line is skipped, not parsed");
    assert_eq!(cards[1].title, "second");
}

/// A malformed line stops the load and names its LINE NUMBER.
///
/// A file half-read is not loaded at all. Without the line number an operator
/// holding a 148-line file has to bisect it by hand.
#[test]
fn a_malformed_line_is_refused_by_line_number() {
    let mut file = tempfile::NamedTempFile::new().expect("a temp file");
    writeln!(file, r#"{{"title":"first","card_id":"a","supports":[]}}"#).expect("write");
    writeln!(file, r#"{{"title":"second"}}"#).expect("write");

    let err = read_jsonl::<DraftedCard>(file.path()).expect_err("line 2 has no card_id");
    let message = format!("{err:#}");
    assert!(message.contains(":2"), "names the line: {message}");
}

/// An absent optional field is `None`, not a parse failure.
///
/// Job B leaves `backs`, `watch_out` and `answer_draft` out on many cards, and a
/// loader that demanded them would refuse the real files.
#[test]
fn the_optional_fields_may_be_absent() {
    let card: DraftedCard =
        serde_json::from_str(r#"{"title":"t","card_id":"a"}"#).expect("the minimum parses");
    assert_eq!(card.backs, None);
    assert!(card.supports.is_empty());
    assert_eq!(card.answer_draft, None);
    assert!(!card.over_supports_cap);
}

/// An unknown STANCE stops the load rather than defaulting.
///
/// A stance decides whether a fact is ammunition or a hazard; guessing would put
/// the wrong one on a card.
#[test]
fn an_unknown_stance_stops_the_load() {
    let result: Result<DraftedCard, _> = serde_json::from_str(
        r#"{"title":"t","card_id":"a","supports":[{"allegation_id":"x","stance":"mentions"}]}"#,
    );
    assert!(result.is_err());
}

// ─── The real files still parse ──────────────────────────────────────────────

/// A VERBATIM line from `AUDITS/LINKING_RUN_v1/B_S11.jsonl`.
///
/// Every model here is `deny_unknown_fields`, which turns a key nobody declared
/// into a refused file. That is the behaviour we want — a sentence the loader
/// would otherwise drop stops the run — but it only stays safe if the shapes on
/// disk are pinned by a test. These three lines are copied unedited from the real
/// files; if Job B or Job D adds a field, exactly one of these fails and names it.
const REAL_CARD_LINE: &str = r#"{"title": "The court ordered the $50,000 returned — a gift isn't ordered back.", "backs": null, "supports": [{"allegation_id": "45984d77", "stance": "supports"}, {"allegation_id": "08c0731b", "stance": "supports"}], "watch_out": "CFS will say this very order resolved the $50,000 years ago, and that the funds went to legitimate estate expenses, so I have no live grievance.", "answer_draft": "DRAFT: The court ordered that money replaced — you don't order a gift returned. My question isn't the label. It's who held the check for two and a half months, unappointed.", "dropped_not_in_list": [], "over_supports_cap": false, "card_id": "doc-judge-tighe-opinion-and-order-041212:evidence:8a240a75", "c_code": "C98"}"#;

/// A line from `AUDITS/RANKING_v1/D_talking_points.jsonl`, trimmed to one point.
const REAL_TALKING_POINTS_LINE: &str = r#"{"scenario_code": "S-9", "scenario_id": "12611977-59ef-453b-afdb-0641635e9b10", "name": "Marie refused to pay for the funeral", "model": "claude-opus-5", "cards_shown": 2, "talking_points": [{"position": 1, "text": "I never refused to pay for my father's funeral. Nobody has ever shown one document proving I did.", "backed_by": [1, 2], "why_these_cards": "Both cards state no proof was offered for the refusal accusation.", "backed_by_card_ids": ["doc-awad-v-catholic-family-motion-for-default-and-default-judgment-as-to-phillips:evidence:b4e571b5", "doc-awad-v-catholic-family-motion-for-default-and-default-judgment-as-to-phillips:evidence:92cb5197"], "backed_by_c_codes": ["C181", "C183"]}], "validation_problems": []}"#;

/// A line from `AUDITS/RANKING_v1/D_S-12_candidates.jsonl`, trimmed to one pick.
const REAL_CANDIDATES_LINE: &str = r#"{"scenario_code": "S-12", "scenario_id": "27dc199f-71b0-4d29-a28c-4b7cde27d9ef", "name": "George Phillips ran Unsupervised Estate as Supervised", "model": "claude-opus-5", "pool_size": 40, "gather_file": "/Users/roman/Documents/colossus-legal/AUDITS/RANKING_v1/gather_2026-09-06/S-12_ranked_gather.md", "read_depth": 240, "picks": [{"pick": 1, "k": 11, "reason": "Phillips admits court only 'directed' appointment; no order ever signed \u2014 blessing without authority.", "graph_node_id": "doc-george-phillips-admissions-response:evidence:01d3e125", "c_code": null, "title": "Phillips admits court directed CFS appointment but formal order never signed", "gather_rank": 11}], "talking_points": [{"position": 1, "text": "They say a judge told them to act as guardian, but no order was ever signed, and my father died first.", "backed_by": [11, 30], "why_these_cards": "Both are Phillips's own sworn admissions: direction without a signed order, death before signing.", "backed_by_card_ids": ["doc-george-phillips-admissions-response:evidence:01d3e125", "doc-george-phillips-admissions-response:evidence:8df54011"], "backed_by_c_codes": [null, null]}], "validation_problems": []}"#;

#[test]
fn a_real_job_b_card_line_parses_with_every_key_declared() {
    let card: DraftedCard =
        serde_json::from_str(REAL_CARD_LINE).expect("the real B_S11 line must parse");
    assert_eq!(card.supports.len(), 2);
    assert!(card.backs.is_none());
    assert!(card.card_id.contains(":evidence:"));
}

#[test]
fn a_real_talking_points_line_parses_with_every_key_declared() {
    let file: TalkingPointsFile =
        serde_json::from_str(REAL_TALKING_POINTS_LINE).expect("the real D line must parse");
    assert_eq!(file.scenario_code, "S-9");
    assert_eq!(file.talking_points.len(), 1);
    assert_eq!(file.talking_points[0].position, 1);
}

#[test]
fn a_real_candidates_line_parses_with_every_key_declared() {
    let file: CandidatesFile =
        serde_json::from_str(REAL_CANDIDATES_LINE).expect("the real D candidates line must parse");
    assert_eq!(file.scenario_code, "S-12");
    assert_eq!(file.picks.len(), 1);
    assert_eq!(file.picks[0].pick, 1);
    assert!(
        file.picks[0].reason.is_some(),
        "the unstored reason is still read"
    );
}

/// AND an undeclared key is REFUSED, by name.
///
/// This is the other half: without it, the three tests above would still pass if
/// somebody quietly removed `deny_unknown_fields`.
#[test]
fn an_undeclared_key_refuses_the_line() {
    let mut line: serde_json::Value = serde_json::from_str(REAL_CARD_LINE).expect("valid json");
    line["a_field_nobody_declared"] = serde_json::json!("some sentence");
    let err = serde_json::from_value::<DraftedCard>(line)
        .expect_err("an undeclared key must refuse the line");
    assert!(
        err.to_string().contains("a_field_nobody_declared"),
        "the refusal must name the key: {err}"
    );
}
