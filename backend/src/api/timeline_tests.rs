//! Tests for the timeline handlers' pure helpers.
//!
//! The handlers themselves need an `AppState` (two pools, a graph, a registry),
//! which this project has no test tier for — the same gap the chronology's own
//! validation guard names. What IS reachable is the target-collection helper,
//! and it is the one place a bug would silently ask the database the wrong
//! question.

use super::*;
use chrono::{TimeZone, Utc};

fn link(target_type: &str, target_id: &str) -> ChronologyLinkRow {
    ChronologyLinkRow {
        event_id: Uuid::from_u128(1),
        target_type: target_type.to_string(),
        target_id: target_id.to_string(),
        label: None,
        pinpoint: None,
        created_by: None,
        created_at: Utc
            .with_ymd_and_hms(2026, 8, 25, 12, 0, 0)
            .single()
            .expect("a real instant"),
    }
}

#[test]
fn only_document_targets_are_asked_about() {
    let links = vec![
        link("document", "doc-a"),
        link("scenario", "S-5"),
        link("paperless_document", "42"),
    ];
    assert_eq!(checkable_target_ids(&links), vec!["doc-a".to_string()]);
}

#[test]
fn the_same_document_twice_is_asked_about_once() {
    let links = vec![
        link("document", "doc-a"),
        link("document", "doc-a"),
        link("document", "doc-b"),
    ];
    assert_eq!(
        checkable_target_ids(&links),
        vec!["doc-a".to_string(), "doc-b".to_string()]
    );
}

#[test]
fn no_links_asks_nothing() {
    assert!(checkable_target_ids(&[]).is_empty());
}
