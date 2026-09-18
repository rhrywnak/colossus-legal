//! The backend half of the Include wire contract (CC_TASK_INCLUDE_PICKER_v1).
//!
//! ## Why a test that reads a file off disk
//!
//! The Include button returned 400 for months. Both sides were tested and both
//! suites were green: the backend's tests built a `FactActionRequest` in Rust and
//! asserted the service accepted it, and the frontend's tests asserted the
//! builder produced what the frontend's tests expected. Neither side ever saw the
//! other's bytes, so the one thing that was wrong — that they disagreed — was the
//! one thing nothing could fail on.
//!
//! `contracts/fact_action_include.json` is the shared artifact that closes it.
//! The frontend asserts its builder emits exactly those bytes; this file asserts
//! those bytes parse here and survive `include_link`. A change to either side
//! that breaks the agreement now fails a test in the OTHER language.
//!
//! ## Rust Learning: `env!("CARGO_MANIFEST_DIR")` for a path outside the crate
//!
//! `env!` reads an environment variable AT COMPILE TIME and pastes it in as a
//! literal — `CARGO_MANIFEST_DIR` is the directory holding `Cargo.toml`, which
//! cargo always sets. It is how a test reaches a file whose location is fixed
//! relative to the source tree rather than to whatever directory the test binary
//! happens to run in. The `..` walks up out of `backend/` to the repo root, where
//! `contracts/` sits beside it precisely because it belongs to neither language.

use super::scenario_facts::{FactAction, FactActionRequest};
use crate::api::scenario_fact_include::include_link;
use crate::domain::fact_card::CardStance;

/// The exact bytes the browser builds for an include.
fn contract_bytes() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../contracts/fact_action_include.json");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()))
}

/// The body the browser builds parses here, and the service accepts it. (M)
///
/// The 400-class dies at `cargo test` rather than under a curator's cursor.
#[test]
fn the_include_body_the_browser_builds_parses_as_a_fact_action() {
    let body = contract_bytes();

    let request: FactActionRequest = serde_json::from_str(body.trim())
        .unwrap_or_else(|e| panic!("the browser's own include body must parse: {e}\n{body}"));

    assert!(matches!(request.action, FactAction::Include));

    let link = include_link(&request).expect("the service must accept the browser's body");
    assert_eq!(
        link,
        Some(("alleg-7", CardStance::Supports)),
        "the accusation and the stance must survive the parse intact — this is \
         the pair that writes the Proof Matrix link"
    );
}

/// An include that names no accusation is STILL refused. (M)
///
/// The other side of the proof. Without this, a service that accepted anything
/// would pass the test above — and the refusal is the behaviour ruling R32
/// depends on, so it has to stay reachable while the browser learns to send the
/// field.
#[test]
fn an_include_body_without_the_allegation_is_still_refused() {
    let request: FactActionRequest =
        serde_json::from_str(r#"{"action":"include"}"#).expect("it parses; it is just incomplete");

    let Err(error) = include_link(&request) else {
        panic!("an include naming no accusation must be refused, not accepted");
    };
    let sentence = format!("{error:?}");
    assert!(
        sentence.contains("allegation_id"),
        "the refusal must name the field that is missing: {sentence}"
    );
}

/// The contract file is the SHAPE the request type declares, key for key.
///
/// ## Why this is not redundant beside the parse above
///
/// `FactActionRequest` carries `#[serde(deny_unknown_fields)]`, so a fixture with
/// a stray key already fails to parse. What it does NOT catch is the opposite
/// drift: a fixture that quietly stops carrying a field the include path needs
/// would still parse (both link fields are `Option`), and the test above would
/// then be asserting `include_link` on a body no browser sends. Naming the three
/// keys here makes the fixture's own shape the thing under test.
#[test]
fn the_contract_names_exactly_the_three_keys_an_include_carries() {
    let value: serde_json::Value =
        serde_json::from_str(contract_bytes().trim()).expect("the contract is JSON");
    let object = value.as_object().expect("the contract is an object");

    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, vec!["action", "allegation_id", "stance"]);
}
