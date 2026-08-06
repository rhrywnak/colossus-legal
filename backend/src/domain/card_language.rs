// =============================================================================
// backend/src/domain/card_language.rs — the naming canon, in code
// =============================================================================
//
// Every user-visible string on a candidate card is produced here. The frontend
// renders what it is given and translates nothing (task 1.2, v2 §7); this module
// is the single place where internal vocabulary becomes plain trial language.
//
// ## Why a module and not a scattering of `match` arms
//
// The July failure was a card reading "contradicts · 70%" — a graph edge class
// and a raw score, shown to a human who then could not rule on it. The canon
// (CASE_STATE_UNIFICATION_REQUIREMENT_v1 §9) fixes one word per concept
// precisely so that cannot recur. A canon enforced by convention is a canon that
// drifts the first time someone writes a `format!` in a handler, so every
// mapping lives here, each tagged with the canon row it implements, and a
// source-scanning test asserts no banned word reaches a payload string.
//
// ## Domain note: two role tokens collapse to one word, deliberately
//
// `FactRole` has four variants but the canon has two words for them:
// CORROBORATES and "supports" both mean the item BACKS the accusation; REBUTS
// and CONTRADICTS both mean it DISPUTES the accusation. The distinctions the
// enum draws (independent confirmation vs. broad support; sworn rebuttal vs.
// factual conflict) are real to the extraction layer and invisible to the human
// ruling on the card — they would be four words where the trial has two. The
// canon chose two; this module implements that choice rather than re-litigating
// it.
//
// One word is RESERVED and must never appear: "contradicts" belongs to the future
// evidence-vs-evidence impeachment layer (canon §9, last row). Using it for an
// evidence-vs-allegation stance would spend the word before that layer exists.

use crate::domain::fact_role::FactRole;
use crate::domain::fact_status::FactStatus;

/// Words that must never appear in a card payload string.
///
/// Drawn from the "Banned for this concept" column of the canon table, plus the
/// raw graph vocabulary the payload exists to translate. The
/// `no_banned_word_reaches_a_payload_string` test walks every string this module
/// can produce and asserts none contains any of these.
///
/// ## Why some near-synonyms are absent
///
/// `ready` and `draft` are NOT banned: the canon explicitly keeps them as
/// workflow status, "a different axis" from verdicts. `strong` / `weak` /
/// `moderate` ARE banned — they are the retired element-verdict vocabulary, and
/// reusing them as confidence bands would put one word on two meanings, which is
/// the exact failure the canon exists to prevent (see `confidence_band`).
pub const BANNED_CARD_WORDS: &[&str] = &[
    // Canon row: "≥1 Supports/Disputes/Comments-on → Allegation" → working evidence
    "probative",
    "linked",
    "connected",
    // Canon row: "ABOUT only" → mentions
    "topical",
    // Canon row: "No edge to any Allegation" → not connected
    "inert",
    "gap",
    // Canon row: "Quote located in source" → grounded
    "proven",
    // Canon row: "Metadata presence" → citability
    "completeness",
    // Canon row: "Reserved for future E→E"
    "contradicts",
    // Retired element-verdict vocabulary (canon row: "Element verdicts")
    "strong",
    "weak",
    "moderate",
    // Raw graph edge classes — the C-222 leak itself
    "corroborates",
    "rebuts",
    "characterizes",
];

/// The plain-trial verb for a stance, from the scan's proposed role.
///
/// Canon row: *CORROBORATES / REBUTS / CHARACTERIZES / ABOUT → supports /
/// disputes / comments on / mentions*.
///
/// ## Rust Learning: an exhaustive `match` over an enum from another module
///
/// No `_ =>` arm. If `FactRole` ever gains a fifth variant, THIS function stops
/// compiling until someone decides which canon word it takes — which is the
/// right place for that decision to surface, because inventing a fifth word
/// silently is how a canon dies.
pub fn stance_verb(role: FactRole) -> &'static str {
    match role {
        // Both mean the item BACKS the accusation. The enum's distinction
        // (independent confirmation vs. broad support) is an extraction concern.
        FactRole::Supports | FactRole::Corroborates => "supports",
        // Both mean the item DISPUTES the accusation. Note `Contradicts` maps to
        // "disputes" and NOT to the word "contradicts", which the canon reserves.
        FactRole::Rebuts | FactRole::Contradicts => "disputes",
    }
}

/// The plain-trial verb for a stance, from the graph edge that carries it.
///
/// Canon row: *CORROBORATES / REBUTS / CHARACTERIZES / ABOUT → supports /
/// disputes / comments on / mentions*. This is that row implemented literally —
/// [`stance_verb`] is its sibling for the scan's proposed-role vocabulary.
///
/// ## Domain note: the edge is the more honest stance
///
/// A scan's proposed role is a claim about the scenario; an edge is a fact about
/// the graph, and it comes with its OBJECT attached (the Allegation it points
/// at). §7.5 requires a stance to be served WITH its object, so the edge is what
/// the card's stance line is built from. The proposed role is used only to
/// explain a defer.
///
/// Returns `None` for an edge class this build does not map, so an unknown edge
/// can never render as a stance it is not (Standing Rule 1). The
/// `neo4j::schema` constants are the source of the match arms — no re-typed
/// literals (Rule 16).
pub fn stance_verb_for_edge(edge_class: &str) -> Option<&'static str> {
    match edge_class {
        crate::neo4j::schema::CORROBORATES => Some("supports"),
        crate::neo4j::schema::REBUTS => Some("disputes"),
        crate::neo4j::schema::CHARACTERIZES => Some("comments on"),
        crate::neo4j::schema::ABOUT => Some("mentions"),
        _ => None,
    }
}

/// The plain-trial label for a candidate's workbench status.
///
/// Not a canon-table row: workflow status is explicitly "a different axis" from
/// verdicts (canon §9, Scenario verdicts row), so these are plain renderings of
/// the `FactStatus` vocabulary rather than canon words.
pub fn status_label(status: FactStatus) -> &'static str {
    match status {
        FactStatus::Undecided => "Not yet decided",
        FactStatus::Included => "In the scenario",
        FactStatus::Dropped => "Set aside",
    }
}

/// The plain-trial label for a quote's grounding state.
///
/// Canon row: *Quote located in source → **grounded***, with "proven" banned.
/// The four states are the vocabulary the ingest path writes
/// (`exact` / `normalized` / `not_found`), plus `pending` as the honest
/// catch-all for anything not yet checked — the same bucketing
/// `api::pipeline::report_data` already applies.
///
/// An unrecognized token returns `None` rather than a plausible default: a
/// grounding state this build does not understand must not render as though it
/// had been verified (Standing Rule 1).
pub fn grounding_label(state: &str) -> Option<&'static str> {
    match state {
        "exact" => Some("Grounded — found on the page"),
        "normalized" => Some("Grounded — found, wording normalized"),
        "not_found" => Some("Not found on the page"),
        "pending" => Some("Not yet checked"),
        _ => None,
    }
}

/// The viewer link for a statement, assembled server-side.
///
/// Matches the route the document workspace already serves (`/documents/:id`)
/// with the page anchor it already reads. Built on this side of the wire because
/// v2 §7 item 2 puts every display decision here.
///
/// ## Why it moved into this module (task 2.11 B2)
///
/// It was private to `services::scenario_card` until the rehearsal page needed
/// the same address. Two functions producing the same URL is one too many: the
/// day the viewer's tab parameter changes, a missed site is a dead link a human
/// clicks in front of opposing counsel, and no compiler error stands behind it.
/// This module is where a display decision belongs.
pub fn viewer_href(document_id: &str, page: Option<i64>) -> String {
    match page {
        Some(page) => format!("/documents/{document_id}?page={page}&tab=document"),
        None => format!("/documents/{document_id}?tab=document"),
    }
}

/// The plain-trial name for a kind of statement, from the extraction's
/// `statement_type`.
///
/// Not a canon row — `statement_type` is extraction vocabulary with an open set,
/// so this humanizes rather than translates: underscores become spaces and the
/// token is left otherwise intact. Returning the token itself (tidied) for an
/// unknown value is right here, unlike `grounding_label`: an unfamiliar statement
/// kind is still meaningful to a lawyer reading it ("partial_admission" reads
/// fine as "partial admission"), whereas an unfamiliar grounding state would be
/// a claim about verification that this build cannot make.
pub fn statement_kind_label(statement_type: &str) -> String {
    statement_type.replace('_', " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_role_maps_to_a_canon_word() {
        // The canon's two words, and the collapse that produces them.
        assert_eq!(stance_verb(FactRole::Supports), "supports");
        assert_eq!(stance_verb(FactRole::Corroborates), "supports");
        assert_eq!(stance_verb(FactRole::Rebuts), "disputes");
        assert_eq!(stance_verb(FactRole::Contradicts), "disputes");
    }

    #[test]
    fn the_reserved_word_never_appears_as_a_stance() {
        // "contradicts" belongs to the future evidence-vs-evidence layer. The
        // `Contradicts` ROLE exists, but its rendered word must not be that.
        for &role in FactRole::ALL {
            assert_ne!(
                stance_verb(role),
                "contradicts",
                "{} rendered the reserved word",
                role.code()
            );
        }
    }

    #[test]
    fn every_edge_class_maps_to_its_canon_word() {
        // The canon row, literally. If any of these four drift, the card starts
        // speaking graph vocabulary again — which is the July defect.
        use crate::neo4j::schema;
        assert_eq!(stance_verb_for_edge(schema::CORROBORATES), Some("supports"));
        assert_eq!(stance_verb_for_edge(schema::REBUTS), Some("disputes"));
        assert_eq!(
            stance_verb_for_edge(schema::CHARACTERIZES),
            Some("comments on")
        );
        assert_eq!(stance_verb_for_edge(schema::ABOUT), Some("mentions"));
    }

    #[test]
    fn an_unmapped_edge_class_yields_no_stance() {
        // CONTRADICTS is a real schema edge, but it is the future
        // evidence-vs-evidence relation — it has no canon word for THIS layer and
        // must not be rendered as one.
        use crate::neo4j::schema;
        assert_eq!(stance_verb_for_edge(schema::CONTRADICTS), None);
        assert_eq!(stance_verb_for_edge("INVENTED_EDGE"), None);
    }

    #[test]
    fn an_unknown_grounding_state_has_no_label() {
        // Loud absence, not a flattering default: a state this build does not
        // know must never render as "found on the page".
        assert_eq!(grounding_label("teleported"), None);
        assert_eq!(grounding_label(""), None);
    }

    #[test]
    fn every_known_grounding_state_has_a_label() {
        for state in ["exact", "normalized", "not_found", "pending"] {
            assert!(
                grounding_label(state).is_some(),
                "{state} is written by the ingest path and must have a label"
            );
        }
    }

    #[test]
    fn grounding_labels_use_the_canon_word_not_the_banned_one() {
        // Canon: the word is "grounded"; "proven" is retired from the product
        // vocabulary until a human rules something proven.
        assert!(grounding_label("exact").is_some_and(|l| l.contains("Grounded")));
        assert!(grounding_label("normalized").is_some_and(|l| l.contains("Grounded")));
    }

    #[test]
    fn statement_kinds_are_humanized_not_translated() {
        assert_eq!(
            statement_kind_label("partial_admission"),
            "partial admission"
        );
        assert_eq!(statement_kind_label("denial"), "denial");
        // An unknown kind still reads: it is lawyer-legible extraction vocabulary,
        // not a claim this build has to stand behind.
        assert_eq!(statement_kind_label("some_new_kind"), "some new kind");
    }

    /// No string this module can produce contains a banned word.
    ///
    /// This is the contract test for the whole language law: the payload is the
    /// enforcement point, and this module is the payload's only source of prose.
    /// Checked case-insensitively and as a substring, so "Probative" and
    /// "non-probative" both trip it.
    #[test]
    fn no_banned_word_reaches_a_payload_string() {
        let mut produced: Vec<String> = Vec::new();
        for &role in FactRole::ALL {
            produced.push(stance_verb(role).to_string());
        }
        for &status in FactStatus::ALL {
            produced.push(status_label(status).to_string());
        }
        for state in ["exact", "normalized", "not_found", "pending"] {
            if let Some(label) = grounding_label(state) {
                produced.push(label.to_string());
            }
        }
        for edge in crate::domain::case_state::partition::ConnectionTier::Topical.edge_types() {
            if let Some(verb) = stance_verb_for_edge(edge) {
                produced.push(verb.to_string());
            }
        }

        for string in &produced {
            let lowered = string.to_lowercase();
            for banned in BANNED_CARD_WORDS {
                assert!(
                    !lowered.contains(banned),
                    "card string {string:?} contains the banned word {banned:?} — \
                     see the naming canon (§9)"
                );
            }
        }
    }

    #[test]
    fn the_banned_list_covers_the_july_failure() {
        // The card that started this: "contradicts · 70%". Both halves of that
        // string are what the canon and the band seam exist to prevent, so assert
        // the word itself is on the list rather than trusting it is.
        assert!(BANNED_CARD_WORDS.contains(&"contradicts"));
        assert!(BANNED_CARD_WORDS.contains(&"probative"));
    }
}
