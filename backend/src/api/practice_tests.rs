//! Unit tests for [`super`] — the drill's URLs, and the two refusals that are
//! about what a client sent.
//!
//! The storage behaviour is DEV-verified (the house convention for a live
//! `PgExecutor`); the composed sentences are pinned in `services::practice_page`
//! and `services::practice_sheet`; the read's rules in
//! `services::practice_read_parse`.
//!
//! ## Why the URL assertions exist at all
//!
//! The .377 build shipped a client calling a path the router did not serve — a
//! whole feature returning 404 with nothing on either side saying why. The
//! frontend half of this guard lives in `services/__tests__/practice.test.ts`;
//! this is the server half, and between them a path can only drift if BOTH are
//! edited to agree.

/// Every path this module serves, with the methods it serves on each.
///
/// Hand-written rather than read back from the `Router`: axum 0.7 exposes no
/// route inventory, and a test that asked the router what it serves would be
/// asking the thing under test to describe itself.
const ROUTES: &[(&str, &[&str])] = &[
    ("/cases/:slug/scenarios/:scenario_id/practice", &["GET"]),
    (
        "/cases/:slug/scenarios/:scenario_id/practice/sessions",
        &["POST"],
    ),
    ("/practice/answers", &["POST"]),
    ("/practice/answers/:answer_id/help", &["POST"]),
    ("/practice/answers/:answer_id/close", &["POST"]),
    ("/practice/sessions/:session_id/end", &["POST"]),
    // Flow v1: the only PUT the drill serves. See the verb test below for why
    // it is a PUT and why that does not make it destructive.
    ("/practice/questions/:question_id/flag", &["PUT"]),
];

#[test]
fn no_path_carries_the_api_prefix_the_gateway_adds() {
    // The .377 failure class from the server side. The backend mounts under
    // `/api`, so a route that spelled it here would serve `/api/api/…` —
    // reachable by nothing, and on screen indistinguishable from a feature
    // nobody built.
    for (path, _) in ROUTES {
        assert!(
            !path.contains("/api/"),
            "{path} names the gateway's own prefix"
        );
    }
}

/// Anything naming a SCENARIO is case-fenced; anything naming a server-minted
/// handle is not, and deliberately.
///
/// The distinction is the module header's: a slug on `/practice/answers/:id` would
/// be ceremony, because the id is unguessable and the real fence — the question
/// must belong to the session's scenario — is enforced in the handler. Asserting
/// the split here is what stops a later route being added to the wrong family by
/// accident.
#[test]
fn scenario_routes_are_case_fenced_and_handle_routes_are_not() {
    for (path, _) in ROUTES {
        if path.contains(":scenario_id") {
            assert!(
                path.starts_with("/cases/:slug/scenarios/:scenario_id/"),
                "{path} names a scenario without a case to fence it"
            );
        } else {
            assert!(
                path.starts_with("/practice/"),
                "{path} is neither case-fenced nor a handle route"
            );
        }
    }
}

/// The drill's verbs, and the one it still refuses to expose.
///
/// ## Domain note: there is no DELETE here, and that is the FRE 612 posture
///
/// The log is the record of what a witness was asked and what she said. Nothing
/// in this build can remove one — not a session, not an answer, not a question.
/// A retraction is a conversation with Chuck, not a button.
///
/// ## Why flow v1's flag is a PUT, and why that is not a loosening
///
/// Writing the same note twice leaves the same row, and clearing is the same
/// call with nothing in it — that is idempotent, which is what PUT means. It
/// removes no RECORD: clearing a flag removes Marie's complaint about a
/// question, never the question, and never anything she said. The verb this
/// test exists to keep out is DELETE, and it is still absent.
#[test]
fn the_drill_exposes_one_read_six_writes_and_no_destructive_verb() {
    let mut reads = 0;
    let mut writes = 0;
    for (_, methods) in ROUTES {
        for method in *methods {
            match *method {
                "GET" => reads += 1,
                "POST" | "PUT" => writes += 1,
                other => panic!("{other} is not a verb this drill serves"),
            }
        }
    }
    assert_eq!((reads, writes), (1, 6));
}

/// The flag route is served, and it is the only PUT.
#[test]
fn the_flag_route_is_served_and_is_the_drills_only_put() {
    let puts: Vec<&str> = ROUTES
        .iter()
        .filter(|(_, m)| m.contains(&"PUT"))
        .map(|(p, _)| *p)
        .collect();
    assert_eq!(puts, vec!["/practice/questions/:question_id/flag"]);
}

/// The answer route is the one the task names, spelled exactly.
///
/// It is quoted in CC_TASK_PRACTICE_SESSION_V0_v1 §5 and in the design, so a
/// rename is a documentation change as well as a code one. Pinning the literal
/// makes that visible in a diff.
#[test]
fn the_answer_route_is_the_path_the_task_names() {
    assert!(ROUTES
        .iter()
        .any(|(p, m)| *p == "/practice/answers" && m.contains(&"POST")));
}

/// A blank note CLEARS the flag; a real one is stored trimmed.
///
/// The three behaviours the handler documents, pinned where a unit test can
/// reach them. This decides whether the database receives a note or a NULL, and
/// it is the difference between "she withdrew her complaint" and "she filed one
/// made of spaces" — which would print as an empty complaint on Chuck's sheet.
#[test]
fn a_blank_flag_note_clears_and_a_real_one_is_stored_trimmed() {
    use crate::api::practice_answers::normalize_flag_note;

    assert_eq!(normalize_flag_note(None), None);
    assert_eq!(normalize_flag_note(Some(String::new())), None);
    assert_eq!(normalize_flag_note(Some("   ".to_string())), None);
    assert_eq!(normalize_flag_note(Some("\t \n".to_string())), None);
    assert_eq!(
        normalize_flag_note(Some("  too soft  ".to_string())),
        Some("too soft".to_string())
    );
    assert_eq!(
        normalize_flag_note(Some("too soft".to_string())),
        Some("too soft".to_string())
    );
}

/// The reveal settles a row `fine` or `repeat` — and never `skipped`.
///
/// `skipped` is a legal value of the column (the flow v1 migration widened the
/// CHECK to three), which is exactly why this gate needs a test: a reader who
/// sees three stored values and two accepted ones would reasonably "fix" the
/// list. The asymmetry is the point. A `skipped` row is written by the
/// mid-sitting control, for a question that never reached a reveal; letting the
/// reveal write it would put her typed answer on Chuck's sheet under a mark
/// saying she never gave one.
#[test]
fn the_reveal_settles_fine_or_repeat_and_refuses_the_skipped_mark() {
    use crate::api::practice_answers::is_settleable_mark;

    assert!(is_settleable_mark("fine"));
    assert!(is_settleable_mark("repeat"));
    assert!(
        !is_settleable_mark("skipped"),
        "the reveal must not relabel an answered question as one she set aside"
    );
    assert!(!is_settleable_mark(""));
    assert!(!is_settleable_mark("Fine"));
}

/// A sitting may only name questions this scenario's deck holds.
///
/// The queue and today's skips are composed in the browser, which makes them
/// client input. Without the fence, a sitting could be opened whose queue named
/// ANOTHER scenario's questions — and Chuck's sheet would carry a question Marie
/// was never asked, with nothing on the page looking wrong.
#[test]
fn a_sitting_naming_a_question_outside_the_deck_is_refused() {
    use crate::api::practice_fences::fence_queue;
    use std::collections::HashSet;
    use uuid::Uuid;

    let a = Uuid::from_u128(1);
    let b = Uuid::from_u128(2);
    let stray = Uuid::from_u128(99);
    let known: HashSet<Uuid> = [a, b].into_iter().collect();

    // Everything belongs.
    assert_eq!(fence_queue(&[a, b], &[], &known), None);
    assert_eq!(fence_queue(&[b], &[a], &known), None);

    // A stray in the QUEUE — the dealt questions.
    assert_eq!(fence_queue(&[a, stray], &[], &known), Some(&stray));

    // A stray in TODAY'S SKIPS deals no question, and is still refused: it is
    // written to the row as the record of what she was offered, so a foreign id
    // there is a lie in the record.
    assert_eq!(fence_queue(&[a], &[stray], &known), Some(&stray));

    // An empty sitting names nothing foreign.
    assert_eq!(fence_queue(&[], &[], &known), None);
}
