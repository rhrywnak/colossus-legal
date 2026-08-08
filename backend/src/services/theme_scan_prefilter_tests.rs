//! Behaviour tests for the scan's pre-filter (task 2.15 Tier 2).
//!
//! Every test here fails if a user-visible promise breaks: a duplicate judged
//! twice, a set-aside candidate reaching the judge, a short ANSWER losing its
//! question, or the conservation arithmetic not adding up. Nothing here restates
//! the shape of a struct.

use super::*;

/// A candidate carrying only what the pre-filter looks at.
fn candidate(id: &str, quote: Option<&str>) -> BiasInstance {
    BiasInstance {
        evidence_id: id.to_string(),
        title: String::new(),
        verbatim_quote: quote.map(str::to_string),
        question: None,
        statement_type: None,
        page_number: None,
        pattern_tags: Vec::new(),
        stated_by: None,
        about: Vec::new(),
        document: None,
    }
}

/// The settings as the migration seeds them.
fn seeded() -> Vec<String> {
    vec!["referral".to_string()]
}

fn config(min_chars: usize, types: &[String]) -> PrefilterConfig<'_> {
    PrefilterConfig {
        min_chars,
        dropped_statement_types: types,
    }
}

/// The 60-character bar the migration seeds. Long enough that the short fixtures
/// below are unambiguously under it.
const SEEDED_MIN_CHARS: usize = 60;

/// A quote comfortably over the seeded bar.
const LONG_QUOTE: &str =
    "There was a request to divide the property, however it could not be divided amicably.";

#[test]
fn byte_identical_quotes_are_judged_once_and_the_group_names_both_nodes() {
    // The measured S-4 defect: C-45/C-46 were the same sentence, judged twice and
    // put in front of Roman twice. One group, one LLM call, both ids carried.
    let types = seeded();
    let prepared = prepare_pool(
        vec![
            candidate("ev-45", Some(LONG_QUOTE)),
            candidate("ev-46", Some(LONG_QUOTE)),
        ],
        config(SEEDED_MIN_CHARS, &types),
    );

    assert_eq!(prepared.groups.len(), 1, "one quote, one call");
    assert_eq!(
        prepared.groups[0].members,
        vec!["ev-45".to_string(), "ev-46".to_string()],
        "both nodes ride the group, so both get a verdict row and one merge rules \
         the set"
    );
    assert_eq!(prepared.conservation.duplicates_collapsed, 1);
    assert_eq!(prepared.conservation.judged, 1);
}

#[test]
fn quotes_that_merely_resemble_each_other_are_not_collapsed() {
    // Near-duplicate detection is Tier 3 and unmeasured on this pool. Collapsing
    // two DIFFERENT statements would silence evidence, so the bar is exact text.
    let types = seeded();
    let mut second = candidate("ev-2", Some(&format!("{LONG_QUOTE} ")));
    second.verbatim_quote = Some(format!("{LONG_QUOTE} "));
    let prepared = prepare_pool(
        vec![candidate("ev-1", Some(LONG_QUOTE)), second],
        config(SEEDED_MIN_CHARS, &types),
    );

    assert_eq!(
        prepared.groups.len(),
        2,
        "one trailing space is a different quote — the collapse must not guess"
    );
    assert_eq!(prepared.conservation.duplicates_collapsed, 0);
}

#[test]
fn an_empty_quote_never_reaches_the_judge_and_is_counted_as_empty() {
    let types = seeded();
    let prepared = prepare_pool(
        vec![
            candidate("ev-blank", Some("   ")),
            candidate("ev-none", None),
            candidate("ev-real", Some(LONG_QUOTE)),
        ],
        config(SEEDED_MIN_CHARS, &types),
    );

    let judged: Vec<&str> = prepared
        .groups
        .iter()
        .map(|g| g.representative.evidence_id.as_str())
        .collect();
    assert_eq!(judged, vec!["ev-real"], "no empty quote is judged");
    assert_eq!(prepared.conservation.excluded_empty, 2);
}

#[test]
fn a_referral_is_set_aside_and_named_by_its_kind() {
    // Measured: "See the responses to the previous interrogatories." was ADMITTED
    // as relevant by the 07-29 run. It carries no assertion at all.
    let types = seeded();
    let mut referral = candidate(
        "ev-ref",
        Some("See the responses to the previous interrogatories, which are incorporated here."),
    );
    referral.statement_type = Some("Referral".to_string());

    let prepared = prepare_pool(
        vec![referral, candidate("ev-real", Some(LONG_QUOTE))],
        config(SEEDED_MIN_CHARS, &types),
    );

    assert_eq!(prepared.groups.len(), 1);
    assert_eq!(prepared.conservation.excluded_statement_type, 1);
    assert_eq!(
        prepared.excluded[0].reason,
        ExclusionReason::StatementType("referral".to_string()),
        "the stored kind is matched case-insensitively, and the reason names it"
    );
}

#[test]
fn a_statement_kind_nobody_configured_is_judged() {
    // The vocabulary belongs to the extractor, not to this build: only kinds the
    // settings row names are dropped, and an empty list drops nothing.
    let mut court = candidate("ev-court", Some(LONG_QUOTE));
    court.statement_type = Some("court_finding".to_string());

    let types = seeded();
    let with_seed = prepare_pool(vec![court.clone()], config(SEEDED_MIN_CHARS, &types));
    assert_eq!(
        with_seed.conservation.judged, 1,
        "court findings are judged"
    );

    let none: Vec<String> = Vec::new();
    let mut referral = candidate("ev-ref", Some(LONG_QUOTE));
    referral.statement_type = Some("referral".to_string());
    let disabled = prepare_pool(vec![referral], config(SEEDED_MIN_CHARS, &none));
    assert_eq!(
        disabled.conservation.excluded_statement_type, 0,
        "an empty list (the stored token `none`) disables the rule entirely"
    );
}

#[test]
fn a_short_answer_survives_when_its_question_gives_it_meaning() {
    // Roman's note on ruling R4, and the reason the length rule is safe at all:
    // "That would be correct." is decisive when the interrogatory is in evidence,
    // and the judge is shown both.
    let types = seeded();
    let mut answered = candidate("ev-yes", Some("That would be correct."));
    answered.question = Some("Did you receive the funds on June 1?".to_string());
    let orphan = candidate("ev-frag", Some("Yeah, we couldn't."));

    let prepared = prepare_pool(vec![answered, orphan], config(SEEDED_MIN_CHARS, &types));

    let judged: Vec<&str> = prepared
        .groups
        .iter()
        .map(|g| g.representative.evidence_id.as_str())
        .collect();
    assert_eq!(
        judged,
        vec!["ev-yes"],
        "the paired question is what separates a short ANSWER from a severed \
         fragment"
    );
    assert_eq!(prepared.conservation.excluded_too_short, 1);
}

#[test]
fn a_blank_question_does_not_rescue_a_fragment() {
    // An empty `question` property reads as absent everywhere else (the judge's
    // user message normalises it the same way); if it rescued a fragment here the
    // two surfaces would disagree about whether the quote is rulable.
    let types = seeded();
    let mut fragment = candidate("ev-frag", Some("Yeah."));
    fragment.question = Some("   ".to_string());

    let prepared = prepare_pool(vec![fragment], config(SEEDED_MIN_CHARS, &types));

    assert_eq!(prepared.conservation.judged, 0);
    assert_eq!(prepared.conservation.excluded_too_short, 1);
}

#[test]
fn a_minimum_of_zero_judges_everything_that_has_text() {
    // The row is tunable from the Settings page with no build, and 0 must mean
    // "off" rather than "drop nothing shorter than nothing".
    let types = seeded();
    let prepared = prepare_pool(vec![candidate("ev-frag", Some("Yeah."))], config(0, &types));

    assert_eq!(prepared.conservation.judged, 1);
    assert_eq!(prepared.conservation.excluded_too_short, 0);
}

#[test]
fn conservation_holds_over_a_pool_that_exercises_every_branch() {
    // THE test of item 1c: every gathered row lands in exactly one bucket, and the
    // buckets add back up to the pool. A candidate that went missing would show up
    // here as a sum that does not reconcile.
    let types = seeded();
    let mut referral = candidate(
        "ev-ref",
        Some("See the previous responses for this answer."),
    );
    referral.statement_type = Some("referral".to_string());
    let mut answered = candidate("ev-yes", Some("Correct."));
    answered.question = Some("Was the property sold in March?".to_string());

    let pool = vec![
        candidate("ev-1", Some(LONG_QUOTE)),
        candidate("ev-2", Some(LONG_QUOTE)), // duplicate of ev-1
        candidate("ev-3", Some("   ")),      // empty
        referral,                            // statement type
        candidate("ev-frag", Some("Yeah.")), // too short, no question
        answered,                            // short but answered → judged
    ];
    let total = pool.len();

    let c = prepare_pool(pool, config(SEEDED_MIN_CHARS, &types)).conservation;

    assert_eq!(c.pool, total);
    assert_eq!(
        c.excluded_empty
            + c.excluded_statement_type
            + c.excluded_too_short
            + c.duplicates_collapsed
            + c.judged,
        c.pool,
        "pool = empty + statement_type + too_short + collapsed + judged — the \
         identity the on-screen line reconciles"
    );
    assert_eq!(c.judged, 2, "ev-1 (with its twin folded in) and ev-yes");
    assert_eq!(c.duplicates_collapsed, 1);
}

#[test]
fn the_pools_own_order_is_preserved() {
    // The results list shows picks in candidate order, and two scans of one
    // unchanged pool should judge the same quotes in the same sequence.
    let types = seeded();
    let prepared = prepare_pool(
        vec![
            candidate("ev-c", Some(&format!("{LONG_QUOTE} C"))),
            candidate("ev-a", Some(&format!("{LONG_QUOTE} A"))),
            candidate("ev-b", Some(&format!("{LONG_QUOTE} B"))),
        ],
        config(SEEDED_MIN_CHARS, &types),
    );

    let order: Vec<&str> = prepared
        .groups
        .iter()
        .map(|g| g.representative.evidence_id.as_str())
        .collect();
    assert_eq!(order, vec!["ev-c", "ev-a", "ev-b"]);
}
