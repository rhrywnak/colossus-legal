//! The generated template is what Roman rules from. If it is wrong, or if it
//! ever pre-fills something other than SKIP, people get merged by accident.

use super::*;

fn party(id: &str, label: &str, name: &str, statements: u64) -> PartyNode {
    PartyNode {
        id: id.to_string(),
        label: label.to_string(),
        display_name: name.to_string(),
        statement_count: statements,
        source_documents: vec!["doc-judge-tighe-opinion-and-order-041212".to_string()],
        aliases: vec!["the Court".to_string()],
    }
}

/// The measured worst cluster: one judge, two nodes, 101 statements split.
fn tighe() -> Vec<PartyNode> {
    vec![
        party("person-karen-a-tighe", "Person", "Karen A. Tighe", 39),
        party("person-tighe", "Person", "Tighe", 62),
        party("person-george-phillips", "Person", "George Phillips", 158),
    ]
}

#[test]
fn honorifics_and_initials_do_not_become_identity_tokens() {
    assert_eq!(identity_tokens("Karen A. Tighe"), vec!["karen", "tighe"]);
    assert_eq!(identity_tokens("Judge Tighe"), vec!["tighe"]);
    assert_eq!(identity_tokens("Mr. Phillips"), vec!["phillips"]);
    assert_eq!(
        identity_tokens("Penzien & McBride, PLLC"),
        vec!["penzien", "mcbride"],
        "an entity suffix would group every firm in the case with every other"
    );
    assert_eq!(
        identity_tokens("C.J."),
        Vec::<String>::new(),
        "an initials-only name carries no identity token at all"
    );
}

#[test]
fn parties_sharing_a_name_token_are_suggested_together() {
    let groups = suggested_groups(&tighe());
    assert_eq!(
        groups,
        vec![(
            "tighe".to_string(),
            vec![
                "person-karen-a-tighe".to_string(),
                "person-tighe".to_string()
            ]
        )],
        "only tokens held by two or more parties become a suggestion"
    );
}

#[test]
fn a_party_can_appear_under_more_than_one_suggestion() {
    // "Camille Handley" and "Camille Hanley" share only a first name; the OCR
    // n/d confusion means their surnames never match. Both must still reach
    // Roman, and they do — via the shared first name.
    let parties = vec![
        party("person-camille-handley", "Person", "Camille Handley", 0),
        party("person-camille-hanley", "Person", "Camille Hanley", 0),
        party("person-james-handley", "Person", "James Handley", 0),
    ];
    let groups = suggested_groups(&parties);
    let tokens: Vec<&str> = groups.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(tokens, vec!["camille", "handley"]);
    assert!(
        groups[1].1.contains(&"person-camille-handley".to_string()),
        "person-camille-handley belongs to both suggestions, and appearing twice \
         is safe: the parser refuses a node ruled in two blocks"
    );
}

#[test]
fn a_repeated_token_in_one_name_does_not_list_a_party_twice() {
    let parties = vec![
        party("person-awad-awad", "Person", "Awad Awad", 0),
        party("person-marie-awad", "Person", "Marie Awad", 54),
    ];
    let groups = suggested_groups(&parties);
    assert_eq!(groups[0].1.len(), 2);
}

#[test]
fn every_generated_block_is_pre_filled_with_skip() {
    // The single most important property of this file: returned unedited, it
    // merges nothing.
    let template = render_template(&tighe());
    let block_count =
        template.matches("\nCLUSTER ").count() + usize::from(template.starts_with("CLUSTER "));
    let skip_count = template.matches("\nSKIP ").count();
    assert!(block_count > 0, "expected at least one block");
    assert_eq!(
        skip_count, block_count,
        "every CLUSTER block must carry a SKIP; a block without one would be a \
         merge instruction the tool wrote for itself"
    );
    assert!(
        !template.contains("\nSURVIVOR "),
        "the template must never pre-fill a survivor"
    );
    assert!(!template.contains("\nMERGE "));
}

#[test]
fn the_generated_template_parses_and_merges_nothing() {
    // The strongest form of the previous assertion: hand it straight to the
    // parser and check the result is all-skip.
    let template = render_template(&tighe());
    let parsed = crate::partymerge::rulings::parse(&template).expect("the template parses");
    assert_eq!(parsed.merges().count(), 0);
    assert_eq!(parsed.skips().count(), parsed.clusters.len());
}

#[test]
fn every_party_appears_in_the_full_census_even_with_no_suggestion() {
    // A cluster the token heuristic misses can only be hand-written from a list
    // that holds everything.
    let template = render_template(&tighe());
    for party in tighe() {
        assert!(
            template.contains(&party.id),
            "{} is missing from the template",
            party.id
        );
    }
    assert!(template.contains("FULL CENSUS"));
}

#[test]
fn a_party_block_carries_the_facts_a_ruling_needs() {
    let template = render_template(&tighe());
    assert!(template.contains("person-tighe · Person · 62 statement(s)"));
    assert!(template.contains("doc-judge-tighe-opinion-and-order-041212"));
    assert!(template.contains("aliases  : the Court"));
}

#[test]
fn a_party_with_no_documents_or_aliases_renders_a_dash_not_an_empty_line() {
    let bare = PartyNode {
        id: "org-archdiocese-of-detroit".to_string(),
        label: "Organization".to_string(),
        display_name: "Archdiocese of Detroit".to_string(),
        statement_count: 0,
        source_documents: Vec::new(),
        aliases: Vec::new(),
    };
    let template = render_template(&[bare]);
    assert!(template.contains("documents: —"));
    assert!(template.contains("aliases  : —"));
}

#[test]
fn two_renders_of_one_census_are_byte_identical() {
    let mut reversed = tighe();
    reversed.reverse();
    assert_eq!(
        render_template(&tighe()),
        render_template(&reversed),
        "the file must not depend on the order the graph returned rows, or two \
         regenerations diff against each other for no reason"
    );
}

#[test]
fn the_header_states_the_measured_census_shape() {
    let template = render_template(&tighe());
    assert!(template.contains("3 parties — 3 Person, 0 Organization"));
    assert!(template.contains("3 carry statements"));
}
