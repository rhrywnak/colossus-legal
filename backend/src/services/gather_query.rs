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
    /// Nothing at all: no theme written, no allegations linked, no talking
    /// points. The composed `text` is EMPTY.
    ///
    /// ⚑ This is why `theme_only` was not enough. A scenario in this state used
    /// to report `theme_only`, which reads as though a theme existed and the
    /// search had simply found little — sending a human to look at the corpus
    /// when the answer is that nobody has written anything down yet. Those are
    /// different problems with different fixes, so they are different tokens.
    ///
    /// Nothing composed this way can be embedded: see
    /// [`crate::services::embedding_service`], which refuses an empty text
    /// rather than returning the degenerate vector one would produce.
    NoContent,
    /// A theme, but no allegations linked — so the theme is all there was to
    /// compose from, and the page should say so rather than presenting a thin
    /// pool as a finished search.
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
            QueryBasis::NoContent => "no_content",
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
        query_basis: basis_of(scenario.theme.as_deref(), allegations, talking_points),
    }
}

/// Which of the four bases this query rests on.
///
/// Decided by what was LINKED, not by what produced text: a scenario with a
/// linked allegation whose text is empty is still `theme_and_allegations`,
/// because the honest report to a human is "this scenario has allegations and
/// one of them has no text", not "this scenario has no allegations".
///
/// The one exception is [`QueryBasis::NoContent`], which is about what EXISTS
/// rather than what was linked: nothing was written and nothing was attached,
/// so there is nothing to compose from and nothing to report but that.
///
/// ## Domain note: why `no_content` also checks the talking points
///
/// The ruling defined it as "no theme statement written and no allegations
/// linked". Marie's talking points are a third source of real content, and a
/// scenario carrying them is not contentless whatever else is missing — so
/// they are checked too. That is one clause wider than the ruling's words and
/// narrower than its intent would allow to slip: `no_content` means the
/// composed text is empty AND nothing was linked, which is the only state the
/// token can honestly describe.
fn basis_of(
    theme: Option<&str>,
    allegations: &[AllegationForQuery],
    talking_points: &[String],
) -> QueryBasis {
    let has_theme = theme.is_some_and(|t| !t.trim().is_empty());
    match (has_theme, allegations.is_empty(), talking_points.is_empty()) {
        (false, true, true) => QueryBasis::NoContent,
        (_, true, _) => QueryBasis::ThemeOnly,
        (_, false, true) => QueryBasis::ThemeAndAllegations,
        (_, false, false) => QueryBasis::ThemeAllegationsAndTalkingPoints,
    }
}

#[cfg(test)]
#[path = "gather_query_tests.rs"]
mod tests;

// Split from `tests` above: the composition cases and the basis cases are two
// separate subjects, and the one file was at 294 lines with no room left
// (Rule 17). `#[path]` keeps both out of this module's own line count.
#[cfg(test)]
#[path = "gather_query_basis_tests.rs"]
mod basis_tests;
