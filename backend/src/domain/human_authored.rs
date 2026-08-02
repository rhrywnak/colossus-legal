// =============================================================================
// backend/src/domain/human_authored.rs — the vocabulary of human-authored content
// =============================================================================
//
// Three human-authored components exist in a scenario (v2 §2): identity (C1),
// human facts (C4), and Marie's talking points (C5). This module owns the small
// vocabularies they need and the one tunable they carry.
//
// ## Domain note: what makes this content different
//
// Everything else in the system is anchored — document, page, verbatim quote —
// precisely so it can be defended. Human-authored content has no citation and is
// not deficient for lacking one: "switching counsel meant emailing attachments"
// is true, load-bearing, and written down nowhere. What it carries instead is an
// AUTHOR, and the surface says so out loud (§8: "visibly tagged human-authored,
// no citation") so nobody mistakes one kind for the other.
//
// The system may propose, score and re-anchor record evidence all day. It may
// never touch any of this (§8's second invariant).

use serde::{Deserialize, Serialize};

// CONST: the default cap on talking points per scenario. §2b makes this a
// tunable, and `talking_points_cap()` below is its single seam — the same shape
// as `domain::confidence_band::band_for_score` and
// `services::scenario_card::context_window_chars`.
//
// TODO(1.6): serve from the settings store. When 1.6 lands, `talking_points_cap`
// reads it from there and this const is deleted. It is on 1.6's seed list.
//
// The value is v2's "~3": Marie holds three points in her head under pressure,
// not seven. It exists to keep the list sayable, not to be precise.
const DEFAULT_TALKING_POINTS_CAP: usize = 3;

/// THE SEAM. How many talking points one scenario may carry.
///
/// Enforced server-side (task 1.4 end state item 3): a cap the browser applies
/// is a suggestion, and the fourth point would be written by any client that did
/// not implement it.
pub fn talking_points_cap() -> usize {
    DEFAULT_TALKING_POINTS_CAP
}

/// How precisely a human fact's date is known.
///
/// ## Domain note: why a date needs a TYPE
///
/// The Casefleet pattern the UI study binds us to (§1.6) stores a date and a
/// date-type together, so "Around 4/21/2009" renders differently from
/// "4/21/2009". Flattening the two would state more precision than the human
/// claimed — and on a timeline that reads as a fact about when something
/// happened, which is exactly the kind of small dishonesty this codebase keeps
/// removing.
///
/// ## Rust Learning: `#[serde(rename_all = "snake_case")]` on a closed enum
///
/// Each variant maps to its snake_case token; the enum is closed, so a token
/// this build does not define fails to parse rather than defaulting to `Exact` —
/// which would be the flattering guess, and the wrong one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DateType {
    /// The date is known precisely.
    Exact,
    /// Approximately this date.
    Around,
    /// Somewhere in a range beginning at this date.
    Range,
    /// Only the ORDER relative to other events is known; the date positions it.
    Ordered,
}

impl DateType {
    /// The full, ordered vocabulary — the "extensible list in code."
    pub const ALL: &'static [DateType] = &[
        DateType::Exact,
        DateType::Around,
        DateType::Range,
        DateType::Ordered,
    ];

    /// The stable wire token, bound into `scenario_human_facts.date_type`.
    pub fn code(self) -> &'static str {
        match self {
            DateType::Exact => "exact",
            DateType::Around => "around",
            DateType::Range => "range",
            DateType::Ordered => "ordered",
        }
    }

    /// How the date reads on screen, with the qualifier the type implies.
    ///
    /// Composed here rather than in the browser: this is the sentence a human
    /// reads on a timeline, and the language law puts every such string on this
    /// side of the wire.
    pub fn describe(self, date: &str) -> String {
        match self {
            DateType::Exact => date.to_string(),
            DateType::Around => format!("Around {date}"),
            DateType::Range => format!("From {date}"),
            DateType::Ordered => format!("Ordered at {date}"),
        }
    }
}

/// The error produced when a stored date-type token is not one this build knows.
#[derive(Debug, thiserror::Error)]
#[error("unknown date-type token '{token}' — not one of exact/around/range/ordered")]
pub struct DateTypeParseError {
    pub token: String,
}

/// ## Rust Learning: `TryFrom<&str>` as the READ boundary
///
/// The database column arrives as a raw `String`, so this is where it becomes a
/// typed value or a loud error — never a silent default. Same discipline as
/// `FactStatus::try_from`.
impl TryFrom<&str> for DateType {
    type Error = DateTypeParseError;

    fn try_from(token: &str) -> Result<Self, Self::Error> {
        match token {
            "exact" => Ok(DateType::Exact),
            "around" => Ok(DateType::Around),
            "range" => Ok(DateType::Range),
            "ordered" => Ok(DateType::Ordered),
            other => Err(DateTypeParseError {
                token: other.to_string(),
            }),
        }
    }
}

/// Which KIND of human-authored note this is.
///
/// ## Domain note: one table, two kinds
///
/// A human fact and a watch-list note are structurally the same thing — text, an
/// author, no citation — and v2 §8 protects both identically. They share a table
/// and a write path so the invariants are enforced once rather than twice; task
/// 1.4's gate review found an asymmetry of exactly that sort (C5 lacked a guard
/// C4 had), and duplicating guards is how that happens.
///
/// What differs is only where they appear: a fact is knowledge the case relies
/// on; a watch-list note is what the other side will wave around (§10's fourth
/// rehearsal block).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HumanFactKind {
    /// Knowledge in no document — the C4 default.
    Fact,
    /// A hazard the other side will raise — §10's watch-list.
    WatchList,
}

impl HumanFactKind {
    /// The full vocabulary — the "extensible list in code."
    pub const ALL: &'static [HumanFactKind] = &[HumanFactKind::Fact, HumanFactKind::WatchList];

    /// The stable wire token, bound into `scenario_human_facts.kind`.
    pub fn code(self) -> &'static str {
        match self {
            HumanFactKind::Fact => "fact",
            HumanFactKind::WatchList => "watch_list",
        }
    }
}

/// The error produced when a stored kind token is not one this build knows.
#[derive(Debug, thiserror::Error)]
#[error("unknown human-fact kind '{token}' — not one of fact/watch_list")]
pub struct HumanFactKindParseError {
    pub token: String,
}

impl TryFrom<&str> for HumanFactKind {
    type Error = HumanFactKindParseError;

    fn try_from(token: &str) -> Result<Self, Self::Error> {
        match token {
            "fact" => Ok(HumanFactKind::Fact),
            "watch_list" => Ok(HumanFactKind::WatchList),
            other => Err(HumanFactKindParseError {
                token: other.to_string(),
            }),
        }
    }
}

/// The standing card shown on every rehearsal screen (v2 §10).
///
/// ## Why this is served by the backend and is not configuration
///
/// It is witness-prep DOCTRINE, not a preference — ABA Formal Opinion 508's
/// substance in four sentences, and the reason rehearsal mode is theme-level
/// rather than a script. A deployment cannot sensibly want different advice, and
/// a browser that held its own copy could drift from the one the exports use.
///
/// Served as data rather than compiled into the page for the same reason every
/// other user-visible string is: one source, one wording.
pub const STANDING_CARD: &[&str] = &[
    "Tell the truth.",
    "Answer only what's asked.",
    "\"I don't recall\" is fine if it's true.",
    "Don't guess.",
];

/// The visible tag that marks content as human-authored (§8).
///
/// ## Domain note: why the tag names the person
///
/// "Added by Roman" and "added by a human" are different claims. The first can be
/// asked about; the second cannot. Since this content has an author in place of a
/// citation, the author is the provenance, and hiding it behind a generic label
/// would throw away the only thing standing behind the statement.
pub fn authored_tag(author: &str) -> String {
    format!("Added by {author}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_type_tokens_match_serde() {
        for &kind in DateType::ALL {
            assert_eq!(
                serde_json::json!(kind),
                serde_json::json!(kind.code()),
                "{} drifted from its serde token",
                kind.code()
            );
        }
    }

    #[test]
    fn an_unknown_date_type_is_a_loud_error_not_a_default() {
        // Defaulting to `Exact` would be the flattering guess: it would claim a
        // precision the human never stated.
        let result = DateType::try_from("someday");
        assert!(result.is_err());
        assert!(DateType::try_from("").is_err());
    }

    #[test]
    fn the_parse_error_names_the_offending_token() {
        // `is_err()` proves the branch; only the message proves the operator can
        // see WHICH value the database held.
        let Err(error) = DateType::try_from("someday") else {
            panic!("refused");
        };
        let message = error.to_string();
        assert!(message.contains("someday"), "{message}");
        for known in DateType::ALL {
            assert!(message.contains(known.code()), "{message}");
        }
    }

    #[test]
    fn every_known_token_round_trips() {
        for &kind in DateType::ALL {
            assert_eq!(DateType::try_from(kind.code()).expect("round trips"), kind);
        }
    }

    #[test]
    fn an_approximate_date_reads_as_approximate() {
        // The whole point of the type: the qualifier survives to the screen.
        assert_eq!(DateType::Exact.describe("2009-04-21"), "2009-04-21");
        assert_eq!(DateType::Around.describe("2009-04-21"), "Around 2009-04-21");
        assert_eq!(DateType::Range.describe("2009-04-21"), "From 2009-04-21");
        assert_eq!(
            DateType::Ordered.describe("2009-04-21"),
            "Ordered at 2009-04-21"
        );
    }

    #[test]
    fn only_an_exact_date_renders_bare() {
        // Every other type must add a qualifier — a bare date from an `Around`
        // would state more than the human claimed.
        for &kind in DateType::ALL {
            let rendered = kind.describe("2009-04-21");
            if kind == DateType::Exact {
                assert_eq!(rendered, "2009-04-21");
            } else {
                assert_ne!(
                    rendered,
                    "2009-04-21",
                    "{} must qualify the date",
                    kind.code()
                );
            }
        }
    }

    #[test]
    fn the_authored_tag_names_the_person() {
        // "Added by a human" would be unaskable. The author IS the provenance
        // here, since there is no citation.
        assert_eq!(authored_tag("Roman"), "Added by Roman");
        assert!(authored_tag("Marie").contains("Marie"));
    }

    #[test]
    fn the_cap_is_reachable_through_its_seam() {
        // Task 1.6 changes the seam's body; nothing else moves. The value is
        // asserted so a silent change to the default is visible in a diff.
        assert_eq!(talking_points_cap(), 3);
        assert!(talking_points_cap() > 0, "a cap of zero would forbid C5");
    }

    // ── HumanFactKind (task 1.5) ─────────────────────────────────────────────
    //
    // The same four tests `DateType` carries, for the same reason: this token IS
    // the filter that decides what reaches a witness. `WatchList.code()` is
    // compared against the stored column in `scenario_readiness`, so a drift
    // between the enum and the string would empty rehearsal's fourth block with
    // no error at all.

    #[test]
    fn human_fact_kind_tokens_match_serde() {
        for &kind in HumanFactKind::ALL {
            assert_eq!(
                serde_json::json!(kind),
                serde_json::json!(kind.code()),
                "{} drifted from its serde token",
                kind.code()
            );
        }
    }

    #[test]
    fn an_unknown_kind_is_a_loud_error_not_a_default() {
        // Defaulting to `Fact` would file a watch-list note as a human fact —
        // the wrong kind of statement, shown in the wrong place, silently.
        assert!(HumanFactKind::try_from("hazard").is_err());
    }

    #[test]
    fn the_kind_parse_error_names_the_offending_token_and_the_vocabulary() {
        let Err(error) = HumanFactKind::try_from("hazard") else {
            panic!("an unknown kind must be refused");
        };
        let message = error.to_string();
        assert!(message.contains("hazard"), "the bad token: {message}");
        for known in HumanFactKind::ALL {
            assert!(
                message.contains(known.code()),
                "{} must be offered: {message}",
                known.code()
            );
        }
    }

    #[test]
    fn every_known_kind_round_trips() {
        for &kind in HumanFactKind::ALL {
            assert_eq!(
                HumanFactKind::try_from(kind.code()).expect("its own token parses"),
                kind
            );
        }
    }

    #[test]
    fn the_kind_vocabulary_is_the_one_the_column_stores() {
        // The migration's DEFAULT is `'fact'`, and the watch-list filter compares
        // against `'watch_list'`. Pinning the literals here means a rename of an
        // enum VARIANT cannot quietly change what is already in the database.
        assert_eq!(HumanFactKind::Fact.code(), "fact");
        assert_eq!(HumanFactKind::WatchList.code(), "watch_list");
        assert_eq!(HumanFactKind::ALL.len(), 2);
    }
}
