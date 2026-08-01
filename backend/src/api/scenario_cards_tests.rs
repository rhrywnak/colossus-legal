//! Unit tests for [`super`] — the card endpoint's pure assembly and partition.
//!
//! The per-card §7 contract is tested in `services::scenario_card_tests`. What is
//! tested here is what this module owns: the partition into pool/set-aside, the
//! stable ordering, the page-text join, and the loud status decode.

use super::*;
use crate::bias::dto::DocumentRef;
use crate::repositories::pipeline_repository::ScenarioFactRefRecord;

fn instance(id: &str, page: Option<i64>) -> BiasInstance {
    BiasInstance {
        evidence_id: id.to_string(),
        title: String::new(),
        verbatim_quote: Some("I do not recall.".to_string()),
        question: None,
        page_number: page,
        pattern_tags: Vec::new(),
        stated_by: None,
        about: Vec::new(),
        document: Some(DocumentRef {
            id: "doc-7".to_string(),
            title: "Deposition".to_string(),
            document_type: None,
        }),
    }
}

fn fact_ref(node: &str, status: &str) -> ScenarioFactRefRecord {
    ScenarioFactRefRecord {
        scenario_id: uuid::Uuid::nil(),
        graph_node_id: node.to_string(),
        role_in_this_scenario: None,
        status: status.to_string(),
        note: None,
        confidence: None,
        source_run_id: None,
        tagged_at: chrono::DateTime::<chrono::Utc>::UNIX_EPOCH,
        defer_reason: None,
    }
}

fn ordinals(pairs: &[(&str, i32)]) -> HashMap<String, i32> {
    pairs
        .iter()
        .map(|(id, n)| ((*id).to_string(), *n))
        .collect()
}

#[test]
fn an_unknown_status_token_is_a_loud_failure_not_a_default() {
    // Standing Rule 1. Bucketing an unrecognized status as "undecided" would show
    // the human a card labelled "Not yet decided" for an item already ruled on.
    let result = build_ref_states(vec![fact_ref("ev-1", "archived")]);
    assert!(
        matches!(result, Err(AppError::Internal { .. })),
        "an undefined status token must fail loudly"
    );
}
