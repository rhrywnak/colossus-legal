//! Stage 0 of the gather cascade: compose the query, and decide who the search
//! may reach.
//!
//! **Pure. No database, no graph, no search.** It composes text and a party set
//! and returns them. Embedding that text is L2b; searching with it, fusing the
//! results and flagging the behaviour are L2b and L2c. Nothing here touches an
//! endpoint or the page.
//!
//! ## What this replaces
//!
//! Today a scenario's gather runs one read — every Evidence node filed ABOUT the
//! scenario's subject — and there is no query at all, only a subject id. That is
//! why S-9 and S-11, two scenarios about different things that happen to name
//! the same person, receive byte-identical pools of 292 cards.
//!
//! ## ⚑ The party set is the load-bearing half
//!
//! Measured from G0's fixture: of the seven $50,000 admissions S-11 cannot see
//! today, only three are filed ABOUT CFS. **Four are about Emil Awad alone.** So
//! a widening of "the subject or CFS" recovers three of seven and AT-2 —
//! five of seven in the top 20, all seven in the top 60 — fails before the
//! reranker is even asked.
//!
//! The widening therefore reaches **every party the linked allegations name**.
//! A-16…A-20 name Emil Awad throughout, which is what brings the other four into
//! reach. `reachable_parties` is that set, and it is the reason this task exists.

use serde::{Deserialize, Serialize};

/// One allegation, as the composer needs it: its handle, its words, and the
/// parties it names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllegationForQuery {
    /// The graph node id.
    pub id: String,
    /// `A-16` — the handle a human says out loud. Display only; the query text
    /// is built from [`Self::text`].
    pub label: String,
    /// The allegation's **verbatim** words from the complaint.
    pub text: String,
    /// Party ids this allegation is filed ABOUT.
    pub parties: Vec<String>,
}

/// What the query was built from. Not cosmetic — L2c reports it when a gather
/// comes back thin, so a human can see whether the pool was small because the
/// corpus is small or because the scenario has nothing linked to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryBasis {
    /// No allegations linked, so the theme is all there was to compose from.
    ///
    /// ⚑ It says nothing about whether a theme EXISTS. A scenario with no theme
    /// written and no allegations linked composes to an EMPTY `text`, and is
    /// still `theme_only` — because the honest report is still "nothing is
    /// linked to this scenario". The emptiness is not hidden: it is visible in
    /// [`GatherQuery::text`], and a caller must check it before embedding,
    /// since an empty string embeds to a degenerate vector that would match
    /// arbitrarily. L2b owns that check; this enum does not encode it, because
    /// the three basis tokens are design vocabulary (§4 Stage 0) and inventing
    /// a fourth is the architect's call, not this task's.
    ThemeOnly,
    ThemeAndAllegations,
    ThemeAllegationsAndTalkingPoints,
}

impl QueryBasis {
    /// The wire token.
    ///
    /// Kept beside the enum so the string and the variant cannot drift; the
    /// `rename_all` above produces the same spelling through serde, and a test
    /// asserts the two agree.
    // STRUCTURAL: these three literals are API wire vocabulary — the JSON tokens
    // for the variants, named by the design (§4 Stage 0) and read by the page.
    // They are not deployment-variable: no environment wants a different
    // spelling, and changing one would be a wire break, not a config change.
    pub fn as_str(self) -> &'static str {
        match self {
            QueryBasis::ThemeOnly => "theme_only",
            QueryBasis::ThemeAndAllegations => "theme_and_allegations",
            QueryBasis::ThemeAllegationsAndTalkingPoints => "theme_allegations_and_talking_points",
        }
    }
}

/// The scenario's own contribution to the query.
///
/// ## Domain note: `theme_statement` is the theme, and the others are not
///
/// A scenario carries four sentence-shaped fields and only one belongs here:
///
/// - **`theme_statement`** — "our one-sentence answer to this attack", the
///   tagline. This is the theme, and it is the one used.
/// - `definition.attack_text` — the OTHER side's verbatim words. Using it would
///   query for the accusation rather than for our answer to it.
/// - `accusation_text` — a human's plain-words restatement of the attack. Same
///   objection, plus it is somebody's paraphrase.
/// - `motivation` — what the other side wants the jury to believe. That is
///   analysis, not the record.
///
/// The design's rule (§0.3) is that the query is built from the record and from
/// Marie's own words, never from architect notes or internal commentary. Three
/// of those four fields are commentary of one kind or another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioQueryInput {
    /// The party the pool must still be about — the subject filter, kept beside
    /// the query text rather than folded into it (design §4 Stage 0).
    pub subject: String,
    /// `None` when the scenario has no theme written yet, which is a real state
    /// and distinct from an empty one.
    pub theme: Option<String>,
}

/// The composed query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatherQuery {
    /// The text L2b will embed: theme, then every allegation's verbatim words,
    /// then Marie's talking points.
    pub text: String,
    pub subject: String,
    /// The subject plus every party the linked allegations name. Sorted and
    /// deduplicated — it is a set, and callers compare it.
    pub reachable_parties: Vec<String>,
    pub query_basis: QueryBasis,
}

/// Compose the gather query for one scenario.
///
/// Pure: everything it needs is in its arguments, so every case below is
/// testable without a database or a graph.
///
/// ## The order is fixed, and the composition is deterministic
///
/// Theme, then allegations in the order given, then talking points in the order
/// given. The caller sorts the allegations (see
/// `repositories::gather_query_repository`), so composing the same scenario
/// twice produces byte-identical text — which matters because the text is
/// embedded, and an unstable query would produce a different vector, a different
/// pool and a different set of proposals on every run for no reason anyone could
/// see.
///
/// ## Empty pieces are skipped, not rendered as blank lines
///
/// An allegation whose text is empty contributes nothing to the query but STILL
/// contributes its parties — a linked allegation with no extracted text is still
/// a statement about somebody, and the widening should reach them. It also still
/// counts for the basis: the scenario does have allegations linked, whatever
/// their text.
pub fn compose_gather_query(
    scenario: &ScenarioQueryInput,
    allegations: &[AllegationForQuery],
    talking_points: &[String],
) -> GatherQuery {
    let mut pieces: Vec<&str> = Vec::new();

    if let Some(theme) = scenario.theme.as_deref() {
        if !theme.trim().is_empty() {
            pieces.push(theme.trim());
        }
    }
    for allegation in allegations {
        if !allegation.text.trim().is_empty() {
            pieces.push(allegation.text.trim());
        }
    }
    for point in talking_points {
        if !point.trim().is_empty() {
            pieces.push(point.trim());
        }
    }

    // ## Rust Learning: `BTreeSet` for a set that is also an ordered list
    //
    // The party set must be deduplicated AND stable — two runs must produce the
    // same `Vec` so a caller can compare them and a test can assert one. A
    // `HashSet` gives the first property and not the second; a `BTreeSet` gives
    // both, and `into_iter()` yields the members already sorted.
    let mut parties: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    // The subject is always reachable: the pool must still be allowed to contain
    // what it contains today, or the widening would be a narrowing.
    parties.insert(scenario.subject.clone());
    for allegation in allegations {
        for party in &allegation.parties {
            if !party.trim().is_empty() {
                parties.insert(party.clone());
            }
        }
    }

    GatherQuery {
        text: pieces.join("\n"),
        subject: scenario.subject.clone(),
        reachable_parties: parties.into_iter().collect(),
        query_basis: basis_of(allegations, talking_points),
    }
}

/// Which of the three bases this query rests on.
///
/// Decided by what was LINKED, not by what produced text: a scenario with a
/// linked allegation whose text is empty is still `theme_and_allegations`,
/// because the honest report to a human is "this scenario has allegations and
/// one of them has no text", not "this scenario has no allegations".
fn basis_of(allegations: &[AllegationForQuery], talking_points: &[String]) -> QueryBasis {
    match (allegations.is_empty(), talking_points.is_empty()) {
        (true, _) => QueryBasis::ThemeOnly,
        (false, true) => QueryBasis::ThemeAndAllegations,
        (false, false) => QueryBasis::ThemeAllegationsAndTalkingPoints,
    }
}

#[cfg(test)]
#[path = "gather_query_tests.rs"]
mod tests;
