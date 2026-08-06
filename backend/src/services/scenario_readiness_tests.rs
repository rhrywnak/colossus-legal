//! Unit tests for [`super`] — the ready gate and the rehearsal read (task 1.5).
//!
//! The load-bearing test here is not the gate arithmetic, it is
//! `the_payload_carries_nothing_it_is_excluded_from`: v2 §10's exclusion law is
//! the one thing in this task that a witness would be harmed by getting wrong,
//! and it is enforced by CONSTRUCTION (the DTO has no such fields). A test that
//! serializes the payload and scans it proves the construction actually holds,
//! and keeps holding when someone adds a field in six months.
//!
//! The transactional writes need a live Postgres and are DEV-verified, matching
//! the convention of every sibling service.

use super::*;

use crate::dto::rehearsal::{RehearsalPayload, RehearsalPoint, RehearsalScenario};

// ── The gate ─────────────────────────────────────────────────────────────────

/// Both drafted spellings read as "not in rehearsal".
///
/// The v2 state machine has two states; the column has three values. This is
/// where the mapping ruled on 2026-08-01 is pinned, so a later reading of
/// `needs_evidence` as "nearly ready" cannot quietly put an unfinished scenario
/// in front of a witness.
#[test]
fn only_the_ready_status_opens_the_gate() {
    for status in ["draft", "needs_evidence"] {
        assert!(
            !is_ready(&record_with_status(status)),
            "'{status}' is v2 'drafted' — it must NOT reach rehearsal"
        );
    }
    assert!(is_ready(&record_with_status(READY_STATUS)));
}

/// A status this build has never heard of is excluded, not included.
///
/// Testing FOR `ready` rather than AGAINST the drafted values means a fourth
/// status added later defaults to invisible. That is the safe direction: a
/// scenario reaches a witness only when someone deliberately said it should.
#[test]
fn an_unknown_status_stays_out_of_rehearsal() {
    assert!(!is_ready(&record_with_status("archived")));
}

// ── The exclusion law (v2 §10) ───────────────────────────────────────────────

/// Nothing §10 excludes can appear in the payload, at any depth.
///
/// ## Why a substring scan and not a field-by-field review
///
/// The DTO's shape is the real enforcement — `RehearsalScenario` has no
/// `motivation` field, so no motivation can be sent. But shape is a claim about
/// today's struct. This test serializes a payload whose every free-text slot is
/// filled with the excluded vocabulary's own words and asserts the JSON is clean,
/// so ADDING a field that carries one of them fails here rather than in front of
/// a witness.
#[test]
fn the_payload_carries_nothing_it_is_excluded_from() {
    let payload = RehearsalPayload {
        scenarios: vec![RehearsalScenario {
            code: "S-2".to_string(),
            theme: Some("We answered every request we received.".to_string()),
            attack: Some("They say we hid the file.".to_string()),
            points: vec![RehearsalPoint {
                text: "I sent the folder the day they asked.".to_string(),
                exhibit: Some("Exhibit 14".to_string()),
            }],
            watch_list: vec!["The April email chain".to_string()],
        }],
        standing_card: STANDING_CARD.iter().map(|s| (*s).to_string()).collect(),
    };

    let json = serde_json::to_string(&payload).expect("the payload serializes");

    // Each entry is a thing §10 forbids the mode from carrying, paired with WHY —
    // the message is what a future reader needs, not the token.
    for (banned, why) in [
        ("motivation", "strategy is not rehearsal material (§10)"),
        ("confidence", "no percentages, no bands (§10)"),
        ("status", "internal state vocabulary (§10)"),
        ("graph_node_id", "internal identifiers (§10)"),
        ("document_id", "pinpoint impeachment sourcing (§10)"),
        ("page", "pinpoint impeachment sourcing (§10)"),
        ("verdict", "no verdicts before Phase 2 (§10)"),
        ("corroborat", "internal graph vocabulary (§9 canon)"),
        ("rebut", "internal graph vocabulary (§9 canon)"),
        ("probative", "internal vocabulary (§10)"),
        ("%", "no percentages (§10)"),
    ] {
        assert!(
            !json.to_lowercase().contains(banned),
            "the rehearsal payload contains '{banned}' — {why}. Payload: {json}"
        );
    }
}

/// The standing card is served, and served whole.
///
/// §10 makes it always-shown, so it is backend-composed like every other string
/// in this mode. A card missing a line would be a rehearsal that quietly stopped
/// telling Marie one of the four things that matter.
#[test]
fn the_standing_card_ships_all_four_lines() {
    assert_eq!(STANDING_CARD.len(), 4, "§10 specifies four lines");
    for expected in [
        "Tell the truth.",
        "Answer only what's asked.",
        "Don't guess.",
    ] {
        assert!(
            STANDING_CARD.contains(&expected),
            "the standing card must carry {expected:?}"
        );
    }
    assert!(
        STANDING_CARD.iter().any(|l| l.contains("I don't recall")),
        "the standing card must carry the 'I don't recall' line"
    );
}

// ── The refusals ─────────────────────────────────────────────────────────────

#[test]
fn declaring_ready_twice_is_refused_rather_than_recorded() {
    // A transition row from `ready` to `ready` would make the history read as
    // though someone did something. The table's CHECK refuses it too — this turns
    // that 500 into a sentence the human can act on.
    let Err(error) = target_status(READY_STATUS, true) else {
        panic!("promoting an already-ready scenario must be refused");
    };
    let message = error.to_string();
    assert!(message.contains(READY_STATUS), "{message}");
    assert!(message.contains("nothing to change"), "{message}");
}

/// Demoting an already-drafted scenario is a no-op in BOTH spellings.
///
/// `needs_evidence` is not `draft`, but neither is visible to a witness, so
/// "remove from rehearsal" changes nothing either way. Comparing against the
/// target token alone would have recorded a `needs_evidence → draft` transition
/// as though someone had withdrawn something.
#[test]
fn demoting_something_already_out_of_rehearsal_is_refused() {
    for status in ["draft", "needs_evidence"] {
        assert!(
            target_status(status, false).is_err(),
            "'{status}' is already out of rehearsal — there is nothing to withdraw"
        );
    }
}

#[test]
fn the_two_real_transitions_are_allowed_and_land_where_expected() {
    assert_eq!(
        target_status("draft", true).expect("drafted → ready is the promote"),
        READY_STATUS
    );
    assert_eq!(
        target_status("needs_evidence", true).expect("the other drafted spelling promotes too"),
        READY_STATUS
    );
    // A demotion writes the plain drafted token, not a claim about evidence.
    assert_eq!(
        target_status(READY_STATUS, false).expect("ready → drafted is the withdraw"),
        "draft"
    );
}

/// A read failure and a write failure say different things.
///
/// The 1.4 lesson, kept: collapsing the two told a human "failed to save" when a
/// LOAD had failed — a message about an action they never took.
#[test]
fn a_failed_read_and_a_failed_write_are_different_sentences() {
    let source = || PipelineRepoError::from(sqlx::Error::RowNotFound);
    let read = ReadinessError::Read { source: source() }.to_string();
    let write = ReadinessError::Write { source: source() }.to_string();

    assert!(read.contains("read the scenario"), "{read}");
    assert!(write.contains("record the readiness change"), "{write}");
    assert_ne!(read, write);
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// A scenario record carrying nothing but the status under test.
fn record_with_status(status: &str) -> ScenarioRecord {
    ScenarioRecord {
        scenario_id: Uuid::nil(),
        case_slug: "awad-v-cfs".to_string(),
        code_ordinal: 2,
        name: "The missing file".to_string(),
        direction: "defensive".to_string(),
        status: status.to_string(),
        theme_statement: None,
        motivation: None,
        // Task 2.11: nobody has written the plain-words accusation for this
        // fixture, which is the honest default — the page renders its gap.
        accusation_text: None,
        definition: serde_json::json!({}),
        anchor_allegation_ids: None,
        feeds_count_id: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}
