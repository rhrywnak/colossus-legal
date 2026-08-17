// Tests for `api::pipeline::party_alias`.
//
// The property this module exists for: a mention spelled the way ANOTHER
// document spelled it must land on the node that already carries that spelling,
// and a mention that could belong to two people must land on neither.
//
// Every fixture below is copied from the LIVE graph as it stood after
// `merge_parties --apply` on 2026-08-17. It is public court data, and it is
// verbatim on purpose: the id-arm defect one branch ago shipped because the
// fixtures were invented in a shape the pipeline never produces.

use super::*;

const PERSON: &str = "Person";
const ORG: &str = "Organization";

/// `person-judge-tighe` exactly as the merge left it — twelve aliases from four
/// documents, three of them generic role words, one of them ("the Court") also
/// held by Judge Murphy.
fn tighe_aliases() -> Vec<&'static str> {
    vec![
        "Probate Judge",
        "Tighe",
        "Honorable Karen A. Tighe",
        "Judge of Family Division",
        "the Court",
        "THE COURT",
        "Judge Tighe",
        "HONORABLE KAREN A. TIGHE",
        "Karen A. Tighe",
        "The Court",
        "Circuit Judge",
        "Family Division",
    ]
}

/// The live graph in miniature: the two judges who share "the Court", plus an
/// organization, so type-scoping is exercised too.
fn live_index() -> PartyAliasIndex {
    let mut rows: Vec<(String, String, String)> = Vec::new();
    let mut add = |t: &str, id: &str, s: &str| {
        rows.push((t.to_string(), id.to_string(), s.to_string()));
    };
    add(PERSON, "person-judge-tighe", "Judge Tighe");
    for a in tighe_aliases() {
        add(PERSON, "person-judge-tighe", a);
    }
    add(PERSON, "person-william-b-murphy", "William B. Murphy");
    for a in ["Murphy", "C.J.", "the Court"] {
        add(PERSON, "person-william-b-murphy", a);
    }
    add(PERSON, "person-jeffrey-humphrey", "Jeffrey Humphrey");
    for a in ["Jeff", "Humphrey", "Jeff from In Your Golden Years"] {
        add(PERSON, "person-jeffrey-humphrey", a);
    }
    add(
        ORG,
        "org-catholic-family-services",
        "Catholic Family Services",
    );
    for a in ["CFS", "Catholic Family Service", "Defendant CFS"] {
        add(ORG, "org-catholic-family-services", a);
    }
    PartyAliasIndex::build(rows)
}

// ── normalization ───────────────────────────────────────────────────────

/// Table-driven, one row per thing normalization is supposed to absorb.
#[test]
fn normalization_absorbs_case_honorifics_punctuation_and_whitespace() {
    let cases: &[(&str, &str, &str)] = &[
        ("case", "KAREN A. TIGHE", "karen tighe"),
        ("case", "karen a. tighe", "karen tighe"),
        ("honorific: judge", "Judge Tighe", "tighe"),
        ("honorific: mr", "Mr. Dalek", "dalek"),
        ("honorific: dr", "Dr. Armaly", "armaly"),
        ("honorific: ms", "Ms. Wurdock", "wurdock"),
        (
            "honorific: honorable",
            "Honorable Karen A. Tighe",
            "karen tighe",
        ),
        ("honorific: messrs", "Messrs. Shaw", "shaw"),
        ("honorific stack", "Hon. Judge Tighe", "tighe"),
        (
            "punctuation",
            "Camille Hanley--Hanley",
            "camille hanley hanley",
        ),
        ("punctuation", "C.J.", "c j"),
        ("whitespace", "  Marie   Awad \n", "marie awad"),
        ("the-prefix", "The Court", "court"),
        (
            "corporate suffix",
            "Penzien & McBride, PLLC",
            "penzien & mcbride",
        ),
        ("middle initial", "William B. Murphy", "william murphy"),
        // Live forms the corpus actually contains, pinned so a change to the
        // parenthesis or apostrophe rule cannot pass unnoticed.
        (
            "bar number in parens",
            "CHARLES M. PENZIEN (P56491)",
            "charles penzien p56491",
        ),
        ("possessive apostrophe", "Marty Wagner's", "marty wagner"),
        (
            "possessive on a name",
            "Emil Awad's estate",
            "emil awad estate",
        ),
        // The slash splits the words — and "attorney" is itself a leading
        // honorific, so it goes too. Both halves of that are deliberate.
        (
            "slash",
            "the attorney/guardian ad litem",
            "guardian ad litem",
        ),
        ("nfc", "Le d\u{e9}p\u{f4}t", "le d\u{e9}p\u{f4}t"),
        (
            "nfd folds to nfc",
            "Le de\u{301}po\u{302}t",
            "le d\u{e9}p\u{f4}t",
        ),
    ];
    for (what, input, expected) in cases {
        assert_eq!(
            normalize_party_key(input),
            *expected,
            "{what}: {input:?} should normalize to {expected:?}",
        );
    }
}

/// The two spellings the merge had to reconcile must land on one key — that is
/// the whole mechanism by which "Karen A. Tighe" finds "Judge Tighe"'s node.
#[test]
fn the_spellings_a_merge_reconciled_share_one_key() {
    let key = normalize_party_key("Tighe");
    for spelling in ["Tighe", "Judge Tighe", "JUDGE TIGHE", "  Judge  Tighe  "] {
        assert_eq!(normalize_party_key(spelling), key, "{spelling:?}");
    }
}

/// A name that is ONLY an honorific, or only an initial, must not normalize to
/// the empty string — every such name would otherwise collide with every other.
#[test]
fn a_name_that_is_all_honorific_or_initial_keeps_a_key() {
    assert_eq!(normalize_party_key("J."), "j");
    assert_eq!(normalize_party_key("I"), "i");
    assert_eq!(normalize_party_key("Judge"), "judge");
    assert_eq!(normalize_party_key("Dr."), "dr");
    assert_ne!(normalize_party_key("J."), normalize_party_key("I"));
}

/// Only leading honorifics go. A surname is not a title.
#[test]
fn an_honorific_inside_or_at_the_end_of_a_name_survives() {
    assert_eq!(
        normalize_party_key("Attorney General Nessel"),
        "general nessel"
    );
    assert_eq!(normalize_party_key("Marie Judge"), "marie judge");
}

// ── matching ────────────────────────────────────────────────────────────

#[test]
fn a_mention_spelled_like_an_alias_binds_to_that_node() {
    let idx = live_index();
    for spelling in ["Karen A. Tighe", "KAREN A. TIGHE", "Tighe", "Judge Tighe"] {
        assert_eq!(
            idx.lookup(PERSON, spelling),
            AliasLookup::Matched("person-judge-tighe".to_string()),
            "{spelling:?} should have found Tighe",
        );
    }
    assert_eq!(
        idx.lookup(PERSON, "Jeff"),
        AliasLookup::Matched("person-jeffrey-humphrey".to_string()),
    );
    assert_eq!(
        idx.lookup(ORG, "Catholic Family Service"),
        AliasLookup::Matched("org-catholic-family-services".to_string()),
    );
}

/// Type-scoped: an organization's alias never binds a person mention, which is
/// what stops "Phillips" the man from landing on "Phillips Corp".
#[test]
fn an_alias_only_binds_within_its_own_entity_type() {
    let idx = live_index();
    assert_eq!(idx.lookup(PERSON, "CFS"), AliasLookup::NoMatch);
    assert_eq!(idx.lookup(ORG, "Tighe"), AliasLookup::NoMatch);
}

#[test]
fn an_unknown_name_does_not_bind_to_anything() {
    let idx = live_index();
    assert_eq!(
        idx.lookup(PERSON, "Someone Entirely New"),
        AliasLookup::NoMatch
    );
}

/// NO FUZZY MATCHING. The rule Roman set: anything not equal after
/// normalization is a new node, and a human merges it.
#[test]
fn a_near_miss_is_not_a_match() {
    let idx = live_index();
    // One letter apart from "Humphrey", and Jaro-Winkler would score it ~0.97.
    assert_eq!(idx.lookup(PERSON, "Humphries"), AliasLookup::NoMatch);
    // The live Handley/Hanley pair, which is a DIFFERENT FAMILY spelling.
    assert_eq!(idx.lookup(PERSON, "Tighc"), AliasLookup::NoMatch);
}

// ── the two guards ──────────────────────────────────────────────────────

/// "the Court" is on Judge Tighe AND Judge Murphy. Binding it either way would
/// attribute one judge's ruling to the other.
#[test]
fn a_string_two_nodes_claim_never_binds() {
    // Built WITHOUT the stoplist in play, to prove the ambiguity guard itself
    // fires rather than the stoplist masking it.
    let idx = PartyAliasIndex::build(vec![
        (PERSON, "person-judge-tighe", "Tighe"),
        (PERSON, "person-judge-tighe", "the presiding judge"),
        (PERSON, "person-william-b-murphy", "Murphy"),
        (PERSON, "person-william-b-murphy", "the presiding judge"),
    ]);
    assert_eq!(
        idx.lookup(PERSON, "the presiding judge"),
        AliasLookup::Ambiguous(vec![
            "person-judge-tighe".to_string(),
            "person-william-b-murphy".to_string(),
        ]),
    );
    // The unambiguous alias on the same node still binds.
    assert_eq!(
        idx.lookup(PERSON, "Tighe"),
        AliasLookup::Matched("person-judge-tighe".to_string()),
    );
}

/// A name on one node and an alias on another is the signature of a merge that
/// has not happened yet — `dalek` is exactly this in the live graph. It must
/// block, not silently pick a side.
#[test]
fn a_name_colliding_with_another_nodes_alias_is_ambiguous() {
    let idx = PartyAliasIndex::build(vec![
        (PERSON, "person-mr-dalek", "Mr. Dalek"),
        (PERSON, "person-gerald-dalek", "Gerald Dalek"),
        (PERSON, "person-gerald-dalek", "Mr. Dalek"),
    ]);
    assert_eq!(
        idx.lookup(PERSON, "Dalek"),
        AliasLookup::Ambiguous(vec![
            "person-gerald-dalek".to_string(),
            "person-mr-dalek".to_string(),
        ]),
    );
}

#[test]
fn the_ambiguity_report_lists_every_contested_string_once() {
    let idx = live_index();
    let ambiguous = idx.ambiguous_keys();
    assert_eq!(ambiguous.len(), 1, "expected exactly one: {ambiguous:?}");
    assert_eq!(ambiguous[0].key, "court");
    assert_eq!(
        ambiguous[0].node_ids,
        vec!["person-judge-tighe", "person-william-b-murphy"],
    );
}

/// Role words never bind, however many nodes carry them.
#[test]
fn a_generic_role_word_never_binds() {
    let idx = live_index();
    for role in [
        "the Court",
        "THE COURT",
        "Probate Judge",
        "Circuit Judge",
        "Family Division",
        "Plaintiff",
        "Defendant",
        "counsel",
        "Affiant",
        "the personal representative",
        "decedent",
        "my father",
        "I",
    ] {
        assert_eq!(
            idx.lookup(PERSON, role),
            AliasLookup::Stoplisted,
            "{role:?} must not resolve",
        );
    }
}

/// The stoplist is matched on the NORMALIZED form, so every casing and
/// honorific variant of a role word is covered by one entry.
#[test]
fn the_stoplist_matches_regardless_of_case_or_punctuation() {
    for variant in ["the Court", "THE COURT", "The  Court", "the court."] {
        assert!(is_stoplisted(&normalize_party_key(variant)), "{variant:?}");
    }
}

/// Stoplisted strings stay ON the node — they are still aliases for display and
/// search. The guard is at the lookup, not at the index.
#[test]
fn a_stoplisted_alias_is_still_indexed_for_its_node() {
    let idx = live_index();
    // "the Court" is indexed under BOTH judges — that is why `ambiguous_keys`
    // reports it — even though `lookup` refuses to bind it.
    let ambiguous = idx.ambiguous_keys();
    assert!(ambiguous.iter().any(|a| a.key == "court"));
}

/// A name-bearing role phrase is NOT stoplisted: "Defendant Phillips" names a
/// person and should keep resolving.
#[test]
fn a_role_word_carrying_a_name_still_binds() {
    let idx = PartyAliasIndex::build(vec![
        (PERSON, "person-george-phillips", "George Phillips"),
        (PERSON, "person-george-phillips", "Defendant Phillips"),
    ]);
    assert_eq!(
        idx.lookup(PERSON, "Defendant Phillips"),
        AliasLookup::Matched("person-george-phillips".to_string()),
    );
    // ...while the bare role word does not.
    assert_eq!(idx.lookup(PERSON, "Defendant"), AliasLookup::Stoplisted);
}

/// A surface form that normalizes to nothing is never indexed under an empty
/// key, and never looked up as one.
#[test]
fn a_surface_form_that_normalizes_to_nothing_is_refused() {
    let idx = PartyAliasIndex::build(vec![
        (PERSON, "person-a", "..."),
        (PERSON, "person-b", "   "),
        (PERSON, "person-c", "Real Name"),
    ]);
    assert_eq!(idx.lookup(PERSON, "..."), AliasLookup::NoMatch);
    assert_eq!(idx.lookup(PERSON, ""), AliasLookup::NoMatch);
    assert!(
        idx.ambiguous_keys().is_empty(),
        "two unindexable forms must not become one ambiguous empty key",
    );
}

/// Every entry in the shipped stoplist must survive normalization — an entry
/// that normalized to nothing would silently stop stoplisting anything.
#[test]
fn every_stoplist_entry_normalizes_to_a_usable_key() {
    for entry in GENERIC_ROLE_STOPLIST {
        let key = normalize_party_key(entry);
        assert!(!key.is_empty(), "{entry:?} normalized to nothing");
        assert!(is_stoplisted(&key), "{entry:?} does not match its own key");
    }
}
