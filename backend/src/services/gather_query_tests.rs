//! Pure tests for the gather query composer.
//!
//! No database, no graph, no fixture file — every case is hand-built, which is
//! the only way to pin behaviour the live data does not currently exhibit (a
//! scenario with no allegations, an allegation with no text, a duplicate party).
//!
//! The one test that DOES read live data is the G0 fixture check at the bottom,
//! and it is `#[ignore]`d because the fixture lives in the documents folder
//! rather than the repo.

use super::*;

fn allegation(label: &str, text: &str, parties: &[&str]) -> AllegationForQuery {
    AllegationForQuery {
        id: format!("doc-complaint:allegation:{label}"),
        label: label.to_string(),
        text: text.to_string(),
        parties: parties.iter().map(|p| (*p).to_string()).collect(),
    }
}

fn scenario() -> ScenarioQueryInput {
    ScenarioQueryInput {
        subject: "person-george-phillips".to_string(),
        theme: Some("Everything downstream flows from one choice.".to_string()),
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

/// Theme then allegations, in the order given.
#[test]
fn theme_and_allegations_compose_in_order() {
    let query = compose_gather_query(
        &scenario(),
        &[
            allegation("A-16", "Allegations of theft arose.", &["person-emil-awad"]),
            allegation(
                "A-17",
                "The money was never returned.",
                &["person-camille-hanley"],
            ),
        ],
        &[],
    );

    assert_eq!(
        query.text,
        "Everything downstream flows from one choice.\n\
         Allegations of theft arose.\n\
         The money was never returned."
    );
    assert_eq!(query.query_basis, QueryBasis::ThemeAndAllegations);
    assert_eq!(query.query_basis.as_str(), "theme_and_allegations");
}

/// Marie's talking points come last, and change the basis.
#[test]
fn talking_points_are_appended_and_change_the_basis() {
    let query = compose_gather_query(
        &scenario(),
        &[allegation("A-16", "Allegations of theft arose.", &[])],
        &["They told the court I refused to pay.".to_string()],
    );

    assert!(query
        .text
        .ends_with("They told the court I refused to pay."));
    assert_eq!(
        query.query_basis,
        QueryBasis::ThemeAllegationsAndTalkingPoints
    );
    assert_eq!(
        query.query_basis.as_str(),
        "theme_allegations_and_talking_points"
    );
}

/// An allegation with EMPTY text contributes no text, still contributes its
/// parties, and still counts for the basis.
///
/// All three halves matter. Rendering it would put a blank line in the embedded
/// text; dropping its parties would narrow the search for a linkage that really
/// exists; and calling the scenario `theme_only` would tell a human it has
/// nothing linked when it has something linked and badly extracted.
#[test]
fn an_allegation_with_no_text_still_widens_the_search() {
    let query = compose_gather_query(
        &scenario(),
        &[
            allegation("A-16", "Allegations of theft arose.", &["person-emil-awad"]),
            allegation("A-99", "   ", &["org-catholic-family-services"]),
        ],
        &[],
    );

    assert_eq!(
        query.text, "Everything downstream flows from one choice.\nAllegations of theft arose.",
        "the empty allegation must not become a blank line in the embedded text"
    );
    assert!(
        query
            .reachable_parties
            .contains(&"org-catholic-family-services".to_string()),
        "its party linkage is real even though its text is missing"
    );
    assert_eq!(
        query.query_basis,
        QueryBasis::ThemeAndAllegations,
        "the scenario HAS allegations; one of them is badly extracted"
    );
}

/// A scenario with no theme composes on its allegations alone rather than
/// leading with a blank line.
#[test]
fn a_scenario_with_no_theme_does_not_lead_with_a_blank_line() {
    let untitled = ScenarioQueryInput {
        subject: "person-george-phillips".to_string(),
        theme: None,
    };
    let query = compose_gather_query(
        &untitled,
        &[allegation("A-16", "Allegations of theft arose.", &[])],
        &[],
    );
    assert_eq!(query.text, "Allegations of theft arose.");
}

/// The party set is a SET: deduplicated, sorted, subject always present.
#[test]
fn the_party_set_is_a_sorted_deduplicated_set_containing_the_subject() {
    let query = compose_gather_query(
        &scenario(),
        &[
            allegation(
                "A-16",
                "x",
                &["person-emil-awad", "org-catholic-family-services"],
            ),
            // The same two parties again, plus a blank that must be ignored.
            allegation("A-17", "y", &["person-emil-awad", "", "  "]),
            allegation("A-19", "z", &["org-catholic-family-services"]),
        ],
        &[],
    );

    assert_eq!(
        query.reachable_parties,
        vec![
            "org-catholic-family-services",
            "person-emil-awad",
            "person-george-phillips",
        ],
        "sorted, deduplicated, subject included, blanks dropped"
    );
}

/// ⚑ The widening reaches Emil Awad, which is what AT-2 turns on.
///
/// Four of the seven $50,000 admissions are filed ABOUT Emil Awad ALONE. A
/// widening of "the subject or CFS" reaches three of seven and AT-2 fails. This
/// pins the property that makes the difference, with the real party ids.
#[test]
fn the_widening_reaches_every_party_the_allegations_name() {
    let query = compose_gather_query(
        &scenario(),
        &[
            allegation(
                "A-16",
                "…$50,000…",
                &["person-camille-hanley", "person-emil-awad"],
            ),
            allegation(
                "A-19",
                "…CFS took possession…",
                &["org-catholic-family-services", "person-emil-awad"],
            ),
        ],
        &[],
    );

    for required in [
        "person-george-phillips",       // the subject
        "person-emil-awad",             // the four admissions about him alone
        "org-catholic-family-services", // the three about CFS
        "person-camille-hanley",
    ] {
        assert!(
            query.reachable_parties.contains(&required.to_string()),
            "the search must be allowed to reach {required}"
        );
    }
}

/// Composing twice produces byte-identical output — pinned against a literal.
///
/// ## Why this is not a tautology
///
/// Comparing two calls to a pure function to each other proves nothing the type
/// system does not already give. So this asserts the composition against a
/// WRITTEN-OUT expected string: the separator, the order and the trimming are
/// pinned as bytes, and any future change to how pieces are joined fails here
/// rather than silently changing every embedded vector in the corpus.
///
/// The other half of determinism — that the allegations arrive in a stable
/// ORDER — is not the composer's to guarantee and is not tested here. It is
/// pinned by `the_read_is_ordered_so_the_query_is_deterministic` in
/// `repositories::gather_query_repository`, on the Cypher `ORDER BY`.
#[test]
fn composing_the_same_scenario_twice_is_byte_identical() {
    let allegations = [
        allegation(
            "A-16",
            "one",
            &["person-emil-awad", "org-catholic-family-services"],
        ),
        allegation("A-17", "  two  ", &["person-camille-hanley"]),
    ];
    let points = ["a talking point".to_string()];

    let first = compose_gather_query(&scenario(), &allegations, &points);
    let second = compose_gather_query(&scenario(), &allegations, &points);

    assert_eq!(
        first.text, "Everything downstream flows from one choice.\none\ntwo\na talking point",
        "newline-joined, in order, each piece trimmed"
    );
    assert_eq!(
        first.reachable_parties,
        vec![
            "org-catholic-family-services",
            "person-camille-hanley",
            "person-emil-awad",
            "person-george-phillips",
        ]
    );
    assert_eq!(first, second);
}

/// ⚑ A scenario with NOTHING — no theme, no allegations — composes an EMPTY
/// query, and that state is visible rather than dressed up.
///
/// L2b must check this before embedding: an empty string embeds to a degenerate
/// vector that matches arbitrarily, which would fill the pool with noise and
/// look like a working search. The basis still reads `theme_only`, which is why
/// `text.is_empty()` — not the basis — is the check that matters.
#[test]
fn a_scenario_with_neither_theme_nor_allegations_composes_nothing() {
    let empty = ScenarioQueryInput {
        subject: "person-george-phillips".to_string(),
        theme: None,
    };
    let query = compose_gather_query(&empty, &[], &[]);

    assert_eq!(query.text, "", "there was nothing to compose from");
    assert_eq!(query.query_basis, QueryBasis::ThemeOnly);
    assert_eq!(
        query.reachable_parties,
        vec!["person-george-phillips"],
        "the subject filter survives even when the query text does not"
    );
}

/// The basis tokens are the three the design names, and serde agrees with
/// `as_str`.
#[test]
fn the_basis_tokens_match_their_serde_spelling() {
    for (basis, token) in [
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

/// L2a.4 — the composer reproduces the nine allegations G0 froze for S-11.
///
/// `#[ignore]`d because the fixture lives in `~/Documents/colossus-legal/GATE/`,
/// outside the repo (law 6), so this cannot run in a checkout that has no
/// documents folder. Run by hand:
///
/// ```text
/// cargo test -p colossus-legal-backend --lib \
///   services::gather_query::tests::the_composer_matches_g0s_frozen_allegation_set \
///   -- --ignored --nocapture
/// ```
///
/// It asserts the ID SET and that every text is non-empty. It deliberately does
/// NOT assert the texts are equal to the fixture's: G0 froze
/// `coalesce(summary, title)` and this composer uses `verbatim_quote`, because
/// L2a.1 says verbatim. That difference is real and is reported as a finding
/// rather than papered over here.
#[test]
#[ignore = "reads G0's fixture from ~/Documents/colossus-legal/GATE/, outside the repo"]
fn the_composer_matches_g0s_frozen_allegation_set() {
    let path = std::env::var("GATE_FIXTURE_S11").unwrap_or_else(|_| {
        format!(
            "{}/Documents/colossus-legal/GATE/s11_gate_fixture_v1.json",
            std::env::var("HOME").expect("HOME is set")
        )
    });
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read the G0 fixture at {path}: {e}"));
    let fixture: serde_json::Value = serde_json::from_str(&raw).expect("the fixture is JSON");

    let frozen = fixture["query"]["allegations"]
        .as_array()
        .expect("the fixture carries its query's allegations");
    assert_eq!(frozen.len(), 9, "S-11 bears on nine allegations");

    // Feed the FROZEN allegations through the real composer. This is the round
    // trip that makes the test worth having: the fixture supplies the input, the
    // composer produces the output, and the assertions are about what the
    // composer did with it.
    let allegations: Vec<AllegationForQuery> = frozen
        .iter()
        .map(|a| AllegationForQuery {
            id: a["id"].as_str().expect("each has an id").to_string(),
            label: a["id"].as_str().expect("each has an id").to_string(),
            text: a["text"].as_str().expect("each has text").to_string(),
            parties: Vec::new(),
        })
        .collect();
    let input = ScenarioQueryInput {
        subject: fixture["query"]["subject"]
            .as_str()
            .expect("the fixture records the subject")
            .to_string(),
        theme: fixture["query"]["theme"].as_str().map(str::to_string),
    };
    let query = compose_gather_query(&input, &allegations, &[]);

    // This first assertion is about the FIXTURE, not the composer: it checks the
    // frozen input is still the nine A-numbers the 08-30 mockup listed. Named
    // for what it is, so nobody later reads it as a composer check.
    let frozen_ids: Vec<&str> = allegations.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(
        frozen_ids,
        vec!["A-16", "A-17", "A-18", "A-19", "A-20", "A-24", "A-27", "A-28", "A-29"],
        "the frozen allegation set is the one the 08-30 mockup listed — if this differs, \
         print both lists and treat it as a finding, not a test to adjust"
    );

    // These are about the COMPOSER: every frozen allegation reached the output,
    // and nothing else did. A dropped allegation is evidence the query will
    // never reach, so the line count is asserted as well as the containment.
    assert_eq!(
        query.text.lines().count(),
        allegations.len() + usize::from(input.theme.is_some()),
        "one line per allegation, plus the theme if there is one — a differing count \
         means a piece was dropped or a blank line was rendered"
    );
    for allegation in &allegations {
        assert!(
            !allegation.text.trim().is_empty(),
            "{} carries no text",
            allegation.label
        );
        assert!(
            query.text.contains(allegation.text.trim()),
            "{}'s words must appear in the composed query — if one is missing, the query \
             silently drops the evidence that allegation was meant to reach",
            allegation.label
        );
    }
    assert_eq!(
        query.query_basis,
        QueryBasis::ThemeAndAllegations,
        "S-11 has nine allegations and no talking points"
    );
    assert!(query
        .reachable_parties
        .contains(&"person-george-phillips".to_string()));

    println!(
        "S-11 composed from G0's fixture: {} chars, {} allegations, basis {}",
        query.text.chars().count(),
        allegations.len(),
        query.query_basis.as_str()
    );
}
