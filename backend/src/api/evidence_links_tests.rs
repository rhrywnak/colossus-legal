//! Tests for the accusation-link routes (task 2.10).
//!
//! The validation rule and the route table, both testable without a database or a
//! booted server. The behavioural half — a real save against a real table — is
//! the DEV click-through this task closes with.

use super::*;
use crate::domain::settings::Settings;

fn settings() -> Settings {
    Settings::for_test()
}

// ─── The one validation rule ─────────────────────────────────────────────────

#[test]
fn a_save_with_no_accusations_is_refused_in_the_stored_words() {
    let Err(error) = validate_allegation_ids(&[], &settings()) else {
        panic!("saving nothing is not a link");
    };
    let AppError::BadRequest { message, .. } = error else {
        panic!("an empty selection is the caller's mistake, not a server fault");
    };
    // R4: the sentence comes from the store, so the browser's pre-check and this
    // refusal cannot drift into telling a human two different things.
    assert_eq!(message, settings().wording.link_missing_allegation_refusal);
}

#[test]
fn blank_and_whitespace_ids_do_not_count_as_a_selection() {
    // A client sending `[""]` has ticked nothing. Accepting it would write a link
    // to an accusation with no id — a row that can never be labelled and never
    // unlinked from the panel that made it.
    for empty in [vec![String::new()], vec!["   ".to_string()], vec![]] {
        assert!(
            validate_allegation_ids(&empty, &settings()).is_err(),
            "{empty:?} is not a selection"
        );
    }
}

#[test]
fn the_ids_come_back_trimmed() {
    let ids = validate_allegation_ids(&["  alleg-7  ".to_string()], &settings()).expect("valid");
    assert_eq!(ids, vec!["alleg-7".to_string()]);
}

#[test]
fn a_repeated_accusation_is_saved_once() {
    // Without the dedup, ticking the same box twice would write the pair and then
    // immediately re-cut it — putting a `link` AND a `recut` in the ledger for one
    // human act, which misreports what happened.
    let ids = validate_allegation_ids(
        &[
            "alleg-7".to_string(),
            "alleg-9".to_string(),
            "alleg-7".to_string(),
        ],
        &settings(),
    )
    .expect("valid");
    assert_eq!(ids, vec!["alleg-7".to_string(), "alleg-9".to_string()]);
}

#[test]
fn the_order_the_human_ticked_is_the_order_that_is_saved() {
    // The card's sentence takes the FIRST link's cut and lists the accusations in
    // the order they were added, so this order reaches the screen.
    let ids = validate_allegation_ids(
        &["b".to_string(), "a".to_string(), "c".to_string()],
        &settings(),
    )
    .expect("valid");
    assert_eq!(ids, vec!["b".to_string(), "a".to_string(), "c".to_string()]);
}

// ─── The routes ──────────────────────────────────────────────────────────────

/// The write paths carry no scenario segment (the case-wide ruling).
///
/// A `:scenario_id` here would make a link mean one thing on one page and another
/// somewhere else — and it would compile, and every other test would pass.
#[test]
fn the_write_routes_are_case_wide() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/api/evidence_links.rs"),
    )
    .expect("this module is on disk");

    let routes = source
        .split_once("pub fn routes()")
        .and_then(|(_, rest)| rest.split_once("\n}"))
        .map(|(body, _)| body)
        .expect("the route table exists");

    assert!(
        routes.contains("/cases/:slug/evidence/:graph_node_id/links"),
        "the save path must be keyed by the statement alone: {routes}"
    );
    assert!(
        routes.contains("/cases/:slug/evidence/:graph_node_id/links/:allegation_id"),
        "the unlink path must name the pair: {routes}"
    );

    // The READ is scenario-scoped (the short list is a fact about the scenario);
    // the two WRITES must not be.
    for line in routes.lines() {
        if line.contains("/links") {
            assert!(
                !line.contains(":scenario_id"),
                "a scenario segment on a link write re-creates the per-scenario \
                 defect: {line}"
            );
        }
    }
}

/// Both writes are edit-gated; the read is not.
#[test]
fn the_writes_require_edit_rights_and_the_read_does_not() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/api/evidence_links.rs"),
    )
    .expect("this module is on disk");

    for handler in ["pub async fn save_links(", "pub async fn remove_link("] {
        let body = source
            .split_once(handler)
            .and_then(|(_, rest)| rest.split_once("\n}"))
            .map(|(body, _)| body)
            .unwrap_or_else(|| panic!("{handler} exists"));
        assert!(
            body.contains("require_edit(&user)?"),
            "{handler} must be edit-gated"
        );
    }

    let read = source
        .split_once("pub async fn list_allegation_options(")
        .and_then(|(_, rest)| rest.split_once("\n}"))
        .map(|(body, _)| body)
        .expect("the read handler exists");
    assert!(
        read.contains("user: Option<AuthUser>"),
        "reading what a case accuses somebody of is not an edit: {read}"
    );
}

/// The pipeline pool, not the main one.
///
/// `evidence_allegation_links` lives in `colossus_legal_v2`. Reaching for
/// `state.pg_pool` would fail at runtime against a table that is not there — on
/// the write path, after the human had typed.
#[test]
fn every_write_uses_the_pipeline_pool() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/api/evidence_links.rs"),
    )
    .expect("this module is on disk");

    let code: String = source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !(trimmed.starts_with("//") || trimmed.starts_with('*'))
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(code.contains("state.pipeline_pool"));
    assert!(
        !code.contains("state.pg_pool"),
        "these tables are in the pipeline database"
    );
}
