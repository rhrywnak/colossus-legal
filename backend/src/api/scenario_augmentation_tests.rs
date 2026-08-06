//! Unit tests for [`super`] — the augmentation family's route shape.
//!
//! The panel's display shaping moved to `scenario_augmentation_read_tests` with
//! the handler it belongs to (task 2.11 C, Rule 17). What is left is the shape of
//! the URLs the writes hang on, which is where the case fence actually lives.

use super::*;
// ── The route shape (task 2.11 C, ruling C4b) ────────────────────────────────
//
// Hand-written rather than read back from the `Router`: axum 0.7 exposes no
// route inventory, and a test that asked the router what it serves would be
// asking the thing under test to describe itself. Same pattern as
// `api::scenario_accusation_tests::ROUTES`.
const ROUTES: &[(&str, &[&str])] = &[
    ("/cases/:slug/scenarios/:scenario_id/augmentation", &["GET"]),
    ("/cases/:slug/scenarios/:scenario_id/human-facts", &["POST"]),
    (
        "/cases/:slug/scenarios/:scenario_id/human-facts/:fact_id",
        &["DELETE", "PUT"],
    ),
    (
        "/cases/:slug/scenarios/:scenario_id/talking-points",
        &["PUT"],
    ),
    (
        "/cases/:slug/scenarios/:scenario_id/talking-points/:position",
        &["PUT"],
    ),
];

#[test]
fn every_path_is_case_and_scenario_scoped() {
    // The fence is only as good as the URL it hangs on: a route without `:slug`
    // could not be case-fenced at all, and one without `:scenario_id` would be
    // writing human content with no scenario to key it to. The two edit routes
    // added in 2.11 C join the existing fence rather than starting a new one,
    // and this is what makes that checkable.
    for (path, _) in ROUTES {
        assert!(
            path.starts_with("/cases/:slug/scenarios/:scenario_id/"),
            "{path}"
        );
    }
}

#[test]
fn no_path_carries_the_api_prefix_the_gateway_adds() {
    // The .377 failure class, from the server side. The backend mounts under
    // `/api`, so a route that spelled it here would serve `/api/api/…` —
    // reachable by nothing, and indistinguishable on screen from a feature
    // nobody built.
    for (path, _) in ROUTES {
        assert!(
            !path.contains("/api/"),
            "{path} names the gateway's own prefix"
        );
    }
}

#[test]
fn the_two_edits_address_a_row_and_not_a_statement() {
    // §10's surviving half, and ruling C1's limit. This surface may address the
    // scenario, one talking point and one watch item — the things it edits. It
    // may NOT address a statement: marking and pairing stay on the working view,
    // and `:graph_node_id` appearing here would mean that moved without anybody
    // deciding it should.
    for (path, _) in ROUTES {
        assert!(
            !path.contains(":graph_node_id"),
            "{path} addresses a statement; this module edits human prose"
        );
    }
}

#[test]
fn one_talking_point_is_addressed_by_its_printed_position() {
    // The pill beside the point and the URL segment must be one number, or
    // editing point 2 lands on point 1. `TalkingPointDto::position` is 1-based
    // for this reason.
    let (path, methods) = ROUTES
        .iter()
        .find(|(p, _)| p.ends_with("talking-points/:position"))
        .expect("the per-point route is declared");

    assert_eq!(*methods, &["PUT"], "{path} is an update and nothing else");
    assert!(
        !path.contains(":item_id") && !path.contains(":point_id"),
        "{path} carries an internal id onto a witness surface"
    );
}

#[test]
fn the_whole_list_write_survives_beside_the_per_row_one() {
    // They answer different intentions and both are needed: the list write is
    // "rearrange or drop one", the row write is "fix a typo in point 2". Losing
    // the first would make reordering impossible; losing the second re-stamps
    // every row's author on every edit, which is what ruling C4b ended.
    let paths: Vec<&str> = ROUTES
        .iter()
        .map(|(p, _)| *p)
        .filter(|p| p.contains("talking-points"))
        .collect();
    assert_eq!(paths.len(), 2, "{paths:?}");
}

#[test]
fn the_router_builds_with_every_declared_path() {
    // matchit refuses some sibling shapes outright — a param and a static child
    // at the same position — and it refuses them by PANICKING at construction.
    // `talking-points` and `talking-points/:position` are exactly that shape, so
    // this is the cheapest possible proof they co-exist, and that they co-exist
    // with every other route in the app.
    let _group = routes();
    let _app = crate::api::router();
}
