// Tests for `services::review_permission` — who may press "Done reviewing".
//
// The table below is the whole rule. It is written as a table because the two
// ways in (listed, admin) and the two refusals (stranger, blank id) are one
// decision, and a reader settling an argument about it should see all four at
// once.

use super::may_review;

fn bench() -> Vec<String> {
    vec!["cpenzien".to_string(), "jdoe".to_string()]
}

/// The defect this task exists for: an admin who is on no display list may
/// still review, and a stranger still may not.
#[test]
fn listed_or_admin_may_review_and_nobody_else() {
    let bench = bench();
    let cases: &[(&str, &str, bool, &[String], bool)] = &[
        ("a listed reviewer", "cpenzien", false, &bench, true),
        ("the last listed entry", "jdoe", false, &bench, true),
        // Roman, 2026-09-22: off the war room list, still an administrator.
        ("an admin who is not listed", "roman", true, &bench, true),
        ("an admin with an EMPTY list", "roman", true, &[], true),
        ("a listed admin", "cpenzien", true, &bench, true),
        ("a stranger", "docmarie", false, &bench, false),
        ("a stranger, empty list", "docmarie", false, &[], false),
    ];
    for (what, user, is_admin, shown, expected) in cases {
        assert_eq!(
            may_review(user, *is_admin, shown),
            *expected,
            "{what}: {user} (admin={is_admin})"
        );
    }
}

/// A blank id is refused BEFORE the admin flag is consulted.
///
/// The auth layer could not name this caller. `"" == ""` would otherwise be
/// true against a blank list entry, and an admin flag on a nameless request is
/// exactly the shape a header-stripping proxy bug would take.
#[test]
fn a_blank_id_is_refused_even_for_an_admin() {
    for blank in ["", "   ", "\t"] {
        assert!(!may_review(blank, false, &bench()));
        assert!(!may_review(blank, true, &bench()), "even as an admin");
        assert!(!may_review(blank, true, &["".to_string()]));
    }
}

/// Usernames are ids: no case folding, no trimming.
///
/// A mistyped row must not silently grant the button, and a login that differs
/// only in case is a different account to Authentik.
#[test]
fn the_shown_list_is_matched_exactly() {
    let bench = bench();
    assert!(!may_review("CPENZIEN", false, &bench), "case is not folded");
    assert!(
        !may_review("cpenzien ", false, &bench),
        "nor is space trimmed"
    );
    assert!(
        !may_review("cpenzie", false, &bench),
        "nor is a prefix enough"
    );
}
