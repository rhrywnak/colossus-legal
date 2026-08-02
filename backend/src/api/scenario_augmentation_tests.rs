//! Unit tests for [`super`] — the augmentation panel's pure shaping.
//!
//! What this module owns: composing display-ready human content. The validation
//! law lives in `services::scenario_augmentation`; the §8 write-path invariants
//! live beside the tables they protect.

use super::*;
use chrono::{TimeZone, Utc};

fn fact_record(
    date: Option<NaiveDate>,
    date_type: Option<&str>,
    edited: bool,
) -> ScenarioHumanFactRecord {
    let created = Utc
        .timestamp_opt(1_700_000_000, 0)
        .single()
        .expect("a real instant");
    ScenarioHumanFactRecord {
        id: Uuid::nil(),
        scenario_id: Uuid::nil(),
        text: "Switching counsel meant emailing attachments".to_string(),
        occurred_on: date,
        date_type: date_type.map(str::to_string),
        person_refs: Some(vec!["Phillips".to_string()]),
        authored_by: "Roman".to_string(),
        created_at: created,
        // An edited fact has a later updated stamp; a fresh one has them equal.
        updated_at: if edited {
            created + chrono::Duration::hours(1)
        } else {
            created
        },
    }
}

fn a_date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2009, 4, 21).expect("a real date")
}

/// A scenario row for the identity tests. Built here rather than borrowed from
/// another module's test helpers — a fixture shared across module boundaries
/// drifts the moment one side needs a different shape.
fn scenario_record() -> ScenarioRecord {
    let ts = Utc
        .timestamp_opt(1_700_000_000, 0)
        .single()
        .expect("a real instant");
    ScenarioRecord {
        scenario_id: Uuid::nil(),
        name: "Marie is obstructive".to_string(),
        direction: "defense".to_string(),
        status: "draft".to_string(),
        case_slug: "awad_v_catholic_family_service".to_string(),
        feeds_count_id: None,
        anchor_allegation_ids: None,
        definition: json!({}),
        created_at: ts,
        updated_at: ts,
        code_ordinal: 3,
        theme_statement: None,
        motivation: None,
    }
}

#[test]
fn a_human_fact_carries_its_authored_tag() {
    // The tag IS this content's provenance — it has no citation by design, so
    // the author is what stands behind it (§8).
    let dto = to_fact_dto(fact_record(None, None, false));
    assert_eq!(dto.authored_tag, "Added by Roman");
}

#[test]
fn an_approximate_date_reads_as_approximate() {
    // The qualifier survives to the screen. A bare date would state precision the
    // human never claimed.
    let dto = to_fact_dto(fact_record(Some(a_date()), Some("around"), false));
    assert_eq!(dto.date_label.as_deref(), Some("Around 2009-04-21"));
}

#[test]
fn an_exact_date_reads_bare() {
    let dto = to_fact_dto(fact_record(Some(a_date()), Some("exact"), false));
    assert_eq!(dto.date_label.as_deref(), Some("2009-04-21"));
}

#[test]
fn a_dateless_fact_has_no_date_label() {
    // The common case — most human facts are undated, and that is not a gap.
    let dto = to_fact_dto(fact_record(None, None, false));
    assert_eq!(dto.date_label, None);
}

#[test]
fn an_unreadable_date_type_falls_back_to_the_bare_date() {
    // Understating precision is the safe direction: a qualifier this build cannot
    // read must not be guessed at.
    let dto = to_fact_dto(fact_record(Some(a_date()), Some("someday"), false));
    assert_eq!(dto.date_label.as_deref(), Some("2009-04-21"));
}

#[test]
fn person_refs_are_never_reported_as_linked_before_b0() {
    // They are names a human typed. Reporting them as resolved entities would
    // claim the system knows WHICH Phillips was meant. It does not.
    let dto = to_fact_dto(fact_record(None, None, false));
    assert_eq!(dto.person_refs, vec!["Phillips".to_string()]);
    assert!(!dto.person_refs_are_linked);
}

#[test]
fn an_untouched_fact_is_not_reported_as_edited() {
    assert!(!to_fact_dto(fact_record(None, None, false)).edited);
    assert!(to_fact_dto(fact_record(None, None, true)).edited);
}

#[test]
fn the_identity_block_keeps_our_theme_and_their_attack_apart() {
    // The distinction 1.5's rehearsal mode depends on: three sentences, three
    // points of view. Collapsing any two loses the one that matters.
    let mut record = scenario_record();
    record.theme_statement = Some("They cannot show what they never disclosed".to_string());
    record.motivation = Some("They want the jury to think Marie is difficult".to_string());
    record.definition = json!({ "attack_text": "Marie is obstructive", "schema_v": 2 });

    let dto = to_identity_dto(&record);
    assert_eq!(dto.attack_text.as_deref(), Some("Marie is obstructive"));
    assert_eq!(
        dto.theme_statement.as_deref(),
        Some("They cannot show what they never disclosed")
    );
    assert_ne!(dto.attack_text, dto.theme_statement);
}

#[test]
fn a_half_authored_scenario_still_yields_an_identity_block() {
    // C1 editing must work before the definition is written — that is WHEN a
    // human writes the theme. An empty definition yields no attack line rather
    // than an error.
    let mut record = scenario_record();
    record.definition = json!({});
    let dto = to_identity_dto(&record);
    assert_eq!(dto.attack_text, None);
    assert_eq!(dto.theme_statement, None);
}
