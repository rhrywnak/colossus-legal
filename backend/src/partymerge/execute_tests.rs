//! The error messages an operator reads when the party merge refuses.
//!
//! Both variants here fire BEFORE anything is written, and both are the only
//! signal behind an exit code — so what they say is the whole of what the
//! operator has to work with.

use super::*;

#[test]
fn an_unknown_edge_type_names_it_and_says_what_to_do() {
    // `preflight_edge_types` refuses the whole run rather than let the merge
    // delete an edge it does not know how to move. The operator has to be told
    // WHICH type and WHERE to add it, or the refusal is a dead end.
    let message = PartyMergeError::UnknownEdgeTypes {
        types: "BLAMES, INDEMNIFIES".to_string(),
    }
    .to_string();

    assert!(message.contains("BLAMES"), "got: {message}");
    assert!(message.contains("INDEMNIFIES"), "got: {message}");
    assert!(message.contains("PARTY_EDGE_TYPES"), "got: {message}");
    assert!(message.contains("Nothing was written"), "got: {message}");
    assert!(
        message.contains("would delete them"),
        "the reason the run refuses has to be in the message, or it reads as \
         pedantry: {message}"
    );
}

#[test]
fn the_invariant_error_does_not_send_the_operator_to_edit_a_constant() {
    // This is why the variant exists. The defensive arm in `apply_cluster` used
    // to borrow `UnknownEdgeTypes`, whose message ends "add them to
    // PARTY_EDGE_TYPES" — advice that would have had someone paste a cluster
    // label into a relationship-type list.
    let message = PartyMergeError::InvariantViolated {
        what: "apply_cluster reached a non-Merge disposition for cluster Tighe".to_string(),
    }
    .to_string();

    assert!(message.contains("BUG"), "got: {message}");
    assert!(message.contains("Tighe"), "got: {message}");
    assert!(message.contains("Nothing was written"), "got: {message}");
    assert!(
        !message.contains("PARTY_EDGE_TYPES"),
        "this must NOT tell the operator to edit the edge-type list: {message}"
    );
    assert!(
        message.contains("not a change to the rulings file"),
        "and it must not send them back to the rulings file either: {message}"
    );
}

#[test]
fn every_store_error_names_the_operation_that_failed() {
    let postgres = PartyMergeError::Postgres {
        operation: "repoint",
        source: sqlx::Error::RowNotFound,
    };
    assert!(postgres.to_string().contains("repoint"));

    let neo4j = PartyMergeError::Neo4jDecode {
        operation: "count_statements",
        source: neo4rs::DeError::PropertyMissingButRequired,
    };
    assert!(neo4j.to_string().contains("count_statements"));
}

#[test]
fn the_edge_type_list_covers_every_direction_the_graph_uses() {
    // Measured on DEV 2026-08-15. If a type is dropped from this list the
    // preflight would refuse every run; if a direction is flipped the repoint
    // would silently move nothing.
    let incoming: Vec<&str> = PARTY_EDGE_TYPES
        .iter()
        .filter(|(_, d)| *d == Direction::Incoming)
        .map(|(t, _)| *t)
        .collect();
    let outgoing: Vec<&str> = PARTY_EDGE_TYPES
        .iter()
        .filter(|(_, d)| *d == Direction::Outgoing)
        .map(|(t, _)| *t)
        .collect();

    assert_eq!(
        incoming,
        vec!["ABOUT", "STATED_BY", "CHARACTERIZES", "SUFFERED_BY"]
    );
    assert_eq!(
        outgoing,
        vec!["CONTAINED_IN"],
        "a party points AT its document; everything else points at the party"
    );
}
