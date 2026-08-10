// Tests for `api::scenarios` — the CRUD route's validators, mappers, and the
// definition every created scenario now carries.
//
// Moved out of `scenarios.rs` on 2026-08-07: adding the create-side definition
// composition took that module well past the 300-line limit (Rule 17), and the
// sibling-file split is the pattern its neighbours already use
// (`scenario_gather_tests.rs`, `scenario_cards_tests.rs`). No test changed in
// the move — only where it lives.

use super::*;

fn sample_record(anchor: Option<Vec<String>>) -> ScenarioRecord {
    // A fixed epoch timestamp keeps the record deterministic (and avoids the
    // chrono `clock` feature); `to_dto` drops timestamps anyway.
    let ts = chrono::DateTime::from_timestamp(0, 0).expect("epoch is valid");
    ScenarioRecord {
        scenario_id: Uuid::nil(),
        name: "Marie is obstructive".to_string(),
        direction: "defense".to_string(),
        status: "draft".to_string(),
        case_slug: "awad_v_catholic_family_service".to_string(),
        feeds_count_id: None,
        anchor_allegation_ids: anchor,
        definition: json!({}),
        created_at: ts,
        updated_at: ts,
        // Every scenario carries a code after the 2026-08-01 backfill;
        // a fixture without one would be a state the column forbids.
        code_ordinal: 1,
        // Unframed: a scenario is created before anyone writes its theme, and
        // `None` is the honest value rather than invented prose.
        theme_statement: None,
        motivation: None,
        // Task 2.11: nobody has written the plain-words accusation for this
        // fixture, which is the honest default — the page renders its gap.
        accusation_text: None,
        accusation_text_authored_by: None,
        accusation_text_authored_at: None,
        theme_authored_by: None,
        theme_authored_at: None,
    }
}

#[test]
fn validate_name_rejects_blank() {
    // The error must be a BadRequest naming the field (the response contract),
    // not merely "an error" — matching the direction/status validator tests.
    match validate_name("   ") {
        Err(AppError::BadRequest { details, .. }) => {
            assert_eq!(details, json!({ "field": "name" }));
        }
        other => panic!("expected BadRequest naming name, got {other:?}"),
    }
    // An empty string is rejected the same way.
    assert!(validate_name("").is_err());
}

#[test]
fn validate_name_accepts_nonempty() {
    assert!(validate_name("Marie is obstructive").is_ok());
}

#[test]
fn validate_direction_accepts_both_valid() {
    assert!(validate_direction("offense").is_ok());
    assert!(validate_direction("defense").is_ok());
}

#[test]
fn validate_direction_rejects_unknown() {
    match validate_direction("sideways") {
        Err(AppError::BadRequest { details, .. }) => {
            assert_eq!(details, json!({ "field": "direction" }));
        }
        other => panic!("expected BadRequest naming direction, got {other:?}"),
    }
}

#[test]
fn validate_status_accepts_the_two_a_scenario_may_be_created_in() {
    for s in ["draft", "ready"] {
        assert!(validate_status(s).is_ok(), "status {s} should be valid");
    }
}

#[test]
fn validate_status_rejects_unknown() {
    match validate_status("archived") {
        Err(AppError::BadRequest { details, .. }) => {
            assert_eq!(details, json!({ "field": "status" }));
        }
        other => panic!("expected BadRequest naming status, got {other:?}"),
    }
}

/// `needs_evidence` cannot be CREATED any more (task R1 Piece 4, ruled 2026-08-10).
///
/// This is the behavioural half of the fix. The create form's Status `<select>`
/// is gone, but a form is not a fence — the route is. Until .390 a caller could
/// POST `needs_evidence` and get a scenario that rendered as **Draft** on its own
/// page (`ScenarioStatusControl` has two segments and folds every non-Ready value
/// into the first) while the dashboard card called it "Needs evidence", with no
/// control able to move it out of the state.
///
/// The refusal names the two accepted values so a caller knows what to send.
#[test]
fn validate_status_refuses_needs_evidence_at_creation() {
    let Err(AppError::BadRequest { message, details }) = validate_status("needs_evidence") else {
        panic!("needs_evidence must no longer be creatable");
    };
    assert_eq!(details, json!({ "field": "status" }));
    assert!(message.contains("draft"), "{message}");
    assert!(message.contains("ready"), "{message}");
    assert!(
        !message.contains("needs_evidence"),
        "the refusal must not advertise the value it just refused: {message}"
    );
}

// The READ path deliberately still understands `needs_evidence` (ruling 6 of
// CC_TASK_R1_RULINGS_v1): closing the write path is not retiring the column
// vocabulary, the `scenarios.status` CHECK still permits the value, and a legacy
// or hand-written row must keep rendering honestly rather than 500-ing. That
// half is already pinned where the parser lives —
// `services::scenario_dashboard::tests::parse_status_maps_each_valid_token` —
// and is NOT restated here. It is named because the two rules are only safe
// together: narrowing both halves at once is what would turn an existing row
// into an unreadable one.

// ── The ready gate's other half (task 1.5) ───────────────────────────────

#[test]
fn an_update_carrying_a_status_is_refused_and_told_where_to_go() {
    // BEHAVIOURAL, not structural: the source scan in `rehearsal_tests` proves
    // the check is present, but it would still pass if the `return Err` were
    // softened to a `tracing::warn!`. This pins what the caller actually gets.
    let Err(AppError::BadRequest { message, details }) = refuse_status_edit(Some("ready")) else {
        panic!("a status on the generic update must be refused");
    };
    assert_eq!(details, json!({ "field": "status" }));
    // The refusal has to be actionable — a caller who wanted to promote a
    // scenario still needs to know how.
    assert!(message.contains("POST"), "{message}");
    assert!(message.contains("/ready"), "{message}");
    assert!(message.contains("recorded human act"), "{message}");
}

#[test]
fn every_status_value_is_refused_including_the_drafted_ones() {
    // Not just `ready`. A demotion through this route would be equally
    // unattributed, and the withdraw direction is the one someone actually
    // asks about later.
    for status in ["draft", "needs_evidence", "ready", "archived"] {
        assert!(
            refuse_status_edit(Some(status)).is_err(),
            "'{status}' must not be settable through the generic update"
        );
    }
}

#[test]
fn an_update_without_a_status_passes_through_untouched() {
    // The common case: a rename, or a theme edit. Refusing those would break
    // every existing caller.
    assert!(refuse_status_edit(None).is_ok());
}

#[test]
fn to_dto_flattens_none_anchor_to_empty_vec() {
    let dto = to_dto(sample_record(None));
    assert_eq!(dto.anchor_allegation_ids, Vec::<String>::new());
    // The Uuid renders as its canonical string form.
    assert_eq!(dto.scenario_id, "00000000-0000-0000-0000-000000000000");
}

#[test]
fn to_dto_preserves_populated_anchor() {
    let ids = vec![
        "doc-awad-v-catholic-family-complaint-11-1-13:allegation:cd24fccb".to_string(),
        "doc-x:allegation:def".to_string(),
    ];
    let dto = to_dto(sample_record(Some(ids.clone())));
    assert_eq!(dto.anchor_allegation_ids, ids);
}

#[test]
fn map_update_error_not_found_becomes_404() {
    // A store `NotFound` (missing id OR cross-case mismatch) must surface as a
    // 404, so the response never confirms the row exists under another case.
    match map_update_error(
        PipelineRepoError::NotFound("some-uuid".to_string()),
        "awad_v_cfs",
    ) {
        AppError::NotFound { message } => assert!(message.contains("not found")),
        other => panic!("expected NotFound → 404, got {other:?}"),
    }
}

#[test]
fn map_update_error_other_becomes_500() {
    // Any non-NotFound store error is an unexpected server fault → 500, never a
    // silent success (Standing Rule 1).
    match map_update_error(
        PipelineRepoError::Database("conn refused".to_string()),
        "awad_v_cfs",
    ) {
        AppError::Internal { message } => assert!(message.contains("update scenario")),
        other => panic!("expected Internal → 500, got {other:?}"),
    }
}

#[test]
fn delete_rows_zero_becomes_404() {
    // A delete that matched no row (unknown id OR wrong case) must be a 404 —
    // never a silent 204 pretending the delete happened (Standing Rule 1).
    match delete_rows_to_status(0) {
        Err(AppError::NotFound { message }) => assert!(message.contains("not found")),
        other => panic!("expected 0 rows → NotFound (404), got {other:?}"),
    }
}

#[test]
fn delete_rows_one_becomes_204() {
    // Exactly one row deleted (the normal case, since scenario_id is the PK) is
    // a 204 No Content.
    match delete_rows_to_status(1) {
        Ok(status) => assert_eq!(status, StatusCode::NO_CONTENT),
        other => panic!("expected 1 row → 204, got {other:?}"),
    }
}

#[test]
fn delete_rows_many_still_succeeds_204() {
    // The PK fence makes a count above 1 impossible, but the mapper treats
    // "≥ 1" as success rather than panicking on an unexpected count.
    match delete_rows_to_status(2) {
        Ok(status) => assert_eq!(status, StatusCode::NO_CONTENT),
        other => panic!("expected ≥1 rows → 204, got {other:?}"),
    }
}

// ── Creating a scenario that is actually defined (2026-08-07) ────────────

fn wording() -> ScenarioAuthoringWording {
    ScenarioAuthoringWording::for_test()
}

/// The defect, stated as a test: what this route writes must be a definition
/// that RESOLVES TO A SUBJECT — not one that falls through to whatever the
/// case default happens to be.
///
/// This is the whole fix in one assertion. If `authored_definition` ever
/// stopped writing a target, the row it produced would look fine, parse
/// fine, and gather 148 candidates belonging to somebody else's scenario.
#[test]
fn a_created_scenario_gathers_over_the_target_the_human_chose() {
    let value = authored_definition(
        "person-marie-awad",
        "George said Marie refused to divide the property.",
        &wording(),
    )
    .expect("a target and an accusation are enough to define a scenario");

    let stored = ScenarioDefinition::from_value(value)
        .expect("what create writes must parse as a v2 definition");

    let subject = crate::services::scenario_subject::resolve_scenario_subject(&stored)
        .expect("a scenario created through this route must resolve to a subject");
    assert_eq!(
        subject, "person-marie-awad",
        "the pool must be gathered over the person the human chose"
    );
}

/// ONE ATTACK BOX (Roman, 2026-08-10): the accusation seeds `attack_text` ALONE.
///
/// Supersedes ruling Q2(a), which had this seed both fields so the parse contract
/// was satisfied and the scan had criteria. It did both of those things and it
/// also shipped every UI-made scenario with two byte-identical texts — asked once,
/// stored twice, rendered on one surface, judged on the other.
///
/// Asserted from both sides because the failure modes are opposite and both look
/// like a working scenario until somebody opens the page: an unfilled `attack_text`
/// makes the definition un-authored, and a seeded `attack_meaning` re-creates the
/// duplicate this ruling removed.
#[test]
fn the_accusation_seeds_the_attack_text_and_nothing_else() {
    let accusation = "George said Marie refused to divide the property.";
    let value = authored_definition("person-marie-awad", accusation, &wording())
        .expect("a valid definition");
    let stored = ScenarioDefinition::from_value(value).expect("it parses");

    assert_eq!(
        stored.attack_text, accusation,
        "attack_text is required by the parse contract AND is what the scan now \
         judges against, so an unfilled one makes the scenario un-authored"
    );
    assert_eq!(
        stored.attack_meaning, None,
        "the gloss is not seeded any more — one question, one box, one stored \
         answer. A copy here is the duplicate the ruling removed."
    );
}

/// Surrounding whitespace must not become a definition.
///
/// A target of `"   "` would be stored, would parse, and would then be sent
/// to the graph as a node id — matching nothing and returning an empty pool
/// with no explanation. That is the silent-empty state this task removes, so
/// it is refused at the boundary instead.
#[test]
fn a_blank_target_or_accusation_is_refused_by_name_and_writes_nothing() {
    let words = wording();

    match authored_definition("   ", "a real accusation", &words) {
        Err(AppError::BadRequest { message, details }) => {
            assert_eq!(details, json!({ "field": "target" }));
            assert_eq!(message, words.create_target_required_refusal);
        }
        other => panic!("expected a named refusal for a blank target, got {other:?}"),
    }

    match authored_definition("person-marie-awad", "  \n ", &words) {
        Err(AppError::BadRequest { message, details }) => {
            assert_eq!(details, json!({ "field": "accusation" }));
            assert_eq!(message, words.create_accusation_required_refusal);
        }
        other => panic!("expected a named refusal for a blank accusation, got {other:?}"),
    }
}
