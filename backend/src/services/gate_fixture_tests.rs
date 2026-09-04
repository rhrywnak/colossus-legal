//! Pure tests for the gate fixture's shape and its audit.
//!
//! No database, no graph, no filesystem: every case below is a hand-built
//! fixture, which is the only kind of input that can pin behaviour the live data
//! happens not to exhibit today — notably the `included ⊄ relevant` case, which
//! DEV does not currently contain and which the bin must therefore be proven to
//! report rather than fail on.

use super::*;

/// A card with just enough on it to be a card.
fn card(id: &str, quote: &str) -> CandidateCard {
    CandidateCard {
        id: id.to_string(),
        c_number: Some("C-1".to_string()),
        title: format!("title of {id}"),
        document: Some("doc-x".to_string()),
        page: Some(22),
        pinpoint: Some("p. 22".to_string()),
        quote: Some(quote.to_string()),
        significance: Some("it matters".to_string()),
        about: vec!["Catholic Family Service".to_string()],
    }
}

/// Two candidates, one of them relevant and Included, and one outside card.
fn small_fixture() -> GateFixture {
    GateFixture {
        scenario: "S-11".to_string(),
        scenario_id: "11111111-1111-1111-1111-111111111111".to_string(),
        run_id: "22222222-2222-2222-2222-222222222222".to_string(),
        run_started_at: "2026-08-29T18:35:13.530272+00:00".to_string(),
        extracted_at: "2026-09-01".to_string(),
        query: GatherQuery {
            theme: Some("Everything downstream begins with the $50,000".to_string()),
            allegations: vec![AllegationRef {
                id: "A-16".to_string(),
                text: "Allegations of theft arose regarding $50,000".to_string(),
            }],
            talking_points: Vec::new(),
            subject: "person-george-phillips".to_string(),
        },
        candidates: vec![
            card("ev-1", "the check was never deposited"),
            card("ev-2", "…"),
        ],
        opus_relevant_ids: vec!["ev-1".to_string()],
        included_ids: vec!["ev-1".to_string()],
        outside_pool: vec![card("ev-outside", "Phillips told Tighe she refused")],
    }
}

fn expected(candidates: usize, relevant: usize, included: usize, outside: usize) -> ExpectedCounts {
    ExpectedCounts {
        candidates,
        opus_relevant: relevant,
        included,
        outside_pool: outside,
    }
}

/// The shape survives a round trip, so G1 can rely on it.
///
/// This is the whole reason the fixture is typed rather than assembled as
/// `serde_json::Value`: a field renamed on the writing side and not on the
/// reading side is caught here, in G0, rather than as an empty gate result weeks
/// later.
#[test]
fn the_fixture_round_trips_through_serde() {
    let original = small_fixture();
    let json = serde_json::to_string_pretty(&original).expect("a fixture serializes");
    let back: GateFixture = serde_json::from_str(&json).expect("and deserializes");
    assert_eq!(original, back);
}

/// An absent page and an absent quote survive as absent — they never become
/// `0` or `""` on the way through JSON.
#[test]
fn absence_survives_the_round_trip() {
    let mut fixture = small_fixture();
    fixture.candidates[0].page = None;
    fixture.candidates[0].pinpoint = None;
    fixture.candidates[0].c_number = None;

    let json = serde_json::to_string(&fixture).expect("serializes");
    let back: GateFixture = serde_json::from_str(&json).expect("deserializes");

    assert_eq!(back.candidates[0].page, None);
    assert_eq!(back.candidates[0].pinpoint, None);
    assert_eq!(back.candidates[0].c_number, None);
}

/// Matching counts read as bare numbers, and the audit says so.
#[test]
fn matching_counts_print_bare_and_pass() {
    let audit = audit_fixture(&small_fixture(), expected(2, 1, 1, 1));

    assert!(audit.counts_match);
    assert!(audit.structurally_sound());
    assert_eq!(
        audit.count_line,
        "S-11 : candidates 2 · opus_relevant 1 · included 1 · outside_pool 1"
    );
}

/// A wrong count prints what we GOT next to what was expected, and does not
/// alter the fixture. This is the guard on the task's ⚑ rule: a tuned query that
/// produces the expected number is a fabricated fixture.
#[test]
fn a_wrong_count_prints_both_numbers() {
    let audit = audit_fixture(&small_fixture(), expected(292, 1, 1, 1));

    assert!(!audit.counts_match);
    assert!(
        audit.count_line.contains("candidates 2 (EXPECTED 292)"),
        "the line must carry both numbers, got: {}",
        audit.count_line
    );
    // The structural checks are independent of the count: the fixture is still
    // internally consistent, it is the HISTORY that differs from the note.
    assert!(audit.structurally_sound());
}

/// Roman Included a card Opus never called relevant. Reported, not failed.
#[test]
fn an_included_card_opus_missed_is_reported_not_failed() {
    let mut fixture = small_fixture();
    fixture.included_ids.push("ev-2".to_string());

    let audit = audit_fixture(&fixture, expected(2, 1, 2, 1));

    assert_eq!(audit.included_not_relevant, vec!["ev-2".to_string()]);
    assert!(
        audit.structurally_sound(),
        "an Included card Opus missed is a finding about the judge prompt, not a failure"
    );
}

/// A relevant id that is not in the pool is a real failure — it means the run
/// and the pool have drifted apart and the gate would be scoring a shorter list
/// than it reports.
#[test]
fn a_relevant_id_outside_the_pool_fails() {
    let mut fixture = small_fixture();
    fixture.opus_relevant_ids.push("ev-vanished".to_string());

    let audit = audit_fixture(&fixture, expected(2, 2, 1, 1));

    assert!(!audit.structurally_sound());
    let check = audit
        .checks
        .iter()
        .find(|c| c.name == "opus_relevant_ids ⊆ candidate ids")
        .expect("the subset check is always present");
    assert!(!check.passed);
    assert!(check.detail.contains("ev-vanished"));
}

/// An outside-pool card that is actually IN the pool fails: the whole claim of
/// the AT-1 / AT-2 tests is that these cards are invisible today.
#[test]
fn an_outside_card_that_is_in_the_pool_fails() {
    let mut fixture = small_fixture();
    fixture.outside_pool = vec![card("ev-1", "already in the pool")];

    let audit = audit_fixture(&fixture, expected(2, 1, 1, 1));

    assert!(!audit.structurally_sound());
    let check = audit
        .checks
        .iter()
        .find(|c| c.name == "no id in both candidates and outside_pool")
        .expect("the overlap check is always present");
    assert!(!check.passed);
    assert!(check.detail.contains("ev-1"));
}

/// A blank quote fails, on an outside card as much as on a pool card: the
/// reranker cannot form a pair without one.
#[test]
fn a_blank_quote_fails_wherever_it_sits() {
    let mut fixture = small_fixture();
    fixture.outside_pool[0].quote = Some("   ".to_string());

    let audit = audit_fixture(&fixture, expected(2, 1, 1, 1));

    assert!(!audit.structurally_sound());
    let check = audit
        .checks
        .iter()
        .find(|c| c.name == "every candidate has a non-empty quote")
        .expect("the quote check is always present");
    assert!(!check.passed);
    assert!(check.detail.contains("ev-outside"));
}

/// A blank allegation text fails — the query composer would silently contribute
/// nothing for that allegation and the query would be a different query.
#[test]
fn a_blank_allegation_text_fails() {
    let mut fixture = small_fixture();
    fixture.query.allegations[0].text = String::new();

    let audit = audit_fixture(&fixture, expected(2, 1, 1, 1));

    assert!(!audit.structurally_sound());
    let check = audit
        .checks
        .iter()
        .find(|c| c.name == "every allegation text is non-empty")
        .expect("the allegation check is always present");
    assert!(!check.passed);
    assert!(check.detail.contains("A-16"));
}
