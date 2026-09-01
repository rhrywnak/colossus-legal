//! Tests for the query BASIS — the token L2c reports when a gather comes back
//! thin, so a human can tell whether the pool was small because the corpus is
//! small or because nothing is attached to this scenario.
//!
//! Split from `gather_query_tests.rs`, which was at 294 code lines with no
//! room left (Rule 17). Composition cases live there; what the query rests on
//! lives here.

use super::*;

/// A scenario with a theme. Defined here rather than shared with
/// `gather_query_tests`: `#[path]` sibling modules cannot see each other's
/// private helpers, and a four-line fixture is cheaper than making one public.
fn scenario() -> ScenarioQueryInput {
    ScenarioQueryInput {
        subject: "person-george-phillips".to_string(),
        theme: Some("Everything downstream flows from one choice.".to_string()),
    }
}

fn allegation(text: &str) -> AllegationForQuery {
    AllegationForQuery {
        id: "doc-complaint:allegation:A-16".to_string(),
        label: "A-16".to_string(),
        text: text.to_string(),
        parties: vec!["person-emil-awad".to_string()],
    }
}

/// Theme alone, and the basis says so.
///
/// The state that matters to L2c: a thin pool here is thin because the scenario
/// has nothing linked, not because the corpus is empty, and the page must be
/// able to tell a human which.
#[test]
fn a_scenario_with_no_allegations_composes_on_theme_alone() {
    let query = compose_gather_query(&scenario(), &[], &[]);

    assert_eq!(query.text, "Everything downstream flows from one choice.");
    assert_eq!(query.query_basis, QueryBasis::ThemeOnly);
    assert_eq!(query.query_basis.as_str(), "theme_only");
    // Even with nothing linked, the subject is reachable — the widening must
    // never be a narrowing.
    assert_eq!(query.reachable_parties, vec!["person-george-phillips"]);
}

/// ⚑ A scenario with NOTHING — no theme, no allegations — is `no_content`, not
/// `theme_only`.
///
/// The distinction is the point of the fourth token. `theme_only` sends a human
/// to look at the corpus ("we searched on the theme and found little");
/// `no_content` sends them to the scenario ("nobody has written anything down
/// yet"). Different problems, different fixes.
#[test]
fn a_scenario_with_neither_theme_nor_allegations_composes_nothing() {
    let empty = ScenarioQueryInput {
        subject: "person-george-phillips".to_string(),
        theme: None,
    };
    let query = compose_gather_query(&empty, &[], &[]);

    assert_eq!(query.text, "", "there was nothing to compose from");
    assert_eq!(query.query_basis, QueryBasis::NoContent);
    assert_eq!(query.query_basis.as_str(), "no_content");
    assert_eq!(
        query.reachable_parties,
        vec!["person-george-phillips"],
        "the subject filter survives even when the query text does not"
    );
}

/// The basis tokens are the four the design names, and serde agrees with
/// `as_str`.
#[test]
fn the_basis_tokens_match_their_serde_spelling() {
    for (basis, token) in [
        (QueryBasis::NoContent, "no_content"),
        (QueryBasis::ThemeOnly, "theme_only"),
        (QueryBasis::ThemeAndAllegations, "theme_and_allegations"),
        (
            QueryBasis::ThemeAllegationsAndTalkingPoints,
            "theme_allegations_and_talking_points",
        ),
    ] {
        assert_eq!(basis.as_str(), token);
        assert_eq!(
            serde_json::to_value(basis).expect("serializes"),
            serde_json::json!(token),
            "as_str and the serde tag must not drift"
        );
    }
}

/// A blank theme string is the same state as no theme at all.
///
/// A scenario whose theme was typed and then cleared holds `Some("  ")`, which
/// composes to nothing just as `None` does. Reporting the first as `theme_only`
/// would be the same lie the fourth token was added to stop telling.
#[test]
fn a_theme_of_only_whitespace_is_no_theme() {
    let blank = ScenarioQueryInput {
        subject: "person-george-phillips".to_string(),
        theme: Some("   \n ".to_string()),
    };
    let query = compose_gather_query(&blank, &[], &[]);

    assert_eq!(query.text, "");
    assert_eq!(query.query_basis, QueryBasis::NoContent);
}

/// `no_content` is about what EXISTS; the other three are about what was
/// LINKED. An allegation with no text keeps the scenario out of `no_content`.
///
/// It composes an empty query all the same — which is exactly why the guard
/// against embedding lives at the embedder, on the text, and not here on the
/// basis. The two predicates are different, and only one of them makes a
/// vector meaningless.
#[test]
fn a_linked_but_empty_allegation_is_not_no_content() {
    let untitled = ScenarioQueryInput {
        subject: "person-george-phillips".to_string(),
        theme: None,
    };
    let query = compose_gather_query(&untitled, &[allegation("   ")], &[]);

    assert_eq!(query.text, "", "nothing had any words in it");
    assert_eq!(
        query.query_basis,
        QueryBasis::ThemeAndAllegations,
        "something IS linked; it is badly extracted, which is a different problem"
    );
    assert!(
        query
            .reachable_parties
            .contains(&"person-emil-awad".to_string()),
        "and its party linkage is still real"
    );
}

/// Talking points alone are content too.
///
/// The ruling worded `no_content` as "no theme and no allegations". Marie's
/// talking points are a third source of real words, so a scenario carrying
/// only those is not contentless — reported as a widening of the ruling.
#[test]
fn talking_points_alone_are_not_no_content() {
    let untitled = ScenarioQueryInput {
        subject: "person-george-phillips".to_string(),
        theme: None,
    };
    let query = compose_gather_query(
        &untitled,
        &[],
        &["They told the court I refused to pay.".to_string()],
    );

    assert_eq!(query.text, "They told the court I refused to pay.");
    assert_ne!(
        query.query_basis,
        QueryBasis::NoContent,
        "there are words here; calling it contentless would be false"
    );
    // Pinned as the positive too, not just the negative: this state reports
    // `theme_only` while carrying no theme, which is imprecise. It is asserted
    // rather than left to drift so the imprecision is a known, findable fact —
    // a fifth token for it is the architect's call, not a test's.
    assert_eq!(
        query.query_basis,
        QueryBasis::ThemeOnly,
        "no allegations linked, so it falls to theme_only — see the report's finding"
    );
}
