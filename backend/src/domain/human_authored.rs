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

// The talking-points cap is a STORED parameter (task 1.6, v2 §2b).
//
// It was an in-code default here behind a `talking_points_cap()` seam, marked
// `TODO(1.6)`. It now lives in `app_settings` and reaches its two enforcement
// points as `Settings::talking_points_cap` — read from the snapshot the handler
// took, never compiled in.
//
// The seeded value is v2's "~3": Marie holds three points in her head under
// pressure, not seven. It exists to keep the list sayable, not to be precise —
// and now Roman can change it without a rebuild, which was always the point.

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
///
/// ## Two ANCHOR kinds joined them in task 2.11
///
/// `AccusationInstance` and `AnswerPairing` are the same table's third shape:
/// they carry no prose at all, only a pointer at the statement they judge. They
/// live here rather than in a table of their own for the reason above — one
/// write path, one set of §8 guards — and because a pairing IS a human-authored
/// judgment, which is precisely what this table holds.
///
/// ## Domain note: the machine never asserts either of them
///
/// "This statement is them making the accusation" and "this is what we said
/// back" are both human judgments. The second especially: asserting that one
/// record item REBUTS another is the contradiction layer the requirement defers,
/// and a rehearsal page that guessed it would put words in a witness's mouth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HumanFactKind {
    /// Knowledge in no document — the C4 default.
    Fact,
    /// A hazard the other side will raise — §10's watch-list.
    WatchList,
    /// This included statement IS them making the accusation (task 2.11).
    ///
    /// Anchored to the statement by graph node id, carrying no text of its own —
    /// the words are already in the record; what a human adds is the judgment
    /// that this one counts as an instance.
    AccusationInstance,
    /// This record item is what we said back to an anchored accusation.
    ///
    /// Two anchors: `anchor_graph_node_id` is the accusation, and
    /// `answers_graph_node_id` is the answer. One per accusation, enforced by a
    /// partial unique index, so re-pairing overwrites rather than leaving the
    /// page to choose between two answers with no basis for choosing.
    AnswerPairing,
}

impl HumanFactKind {
    /// The full vocabulary — the "extensible list in code."
    pub const ALL: &'static [HumanFactKind] = &[
        HumanFactKind::Fact,
        HumanFactKind::WatchList,
        HumanFactKind::AccusationInstance,
        HumanFactKind::AnswerPairing,
    ];

    /// Whether this kind is a POINTER at a statement rather than authored prose.
    ///
    /// The database asks the same question structurally — its non-blank CHECK is
    /// "says something OR points at something" — and deliberately names no kind,
    /// so a future kind gets the safe default without a migration. This is that
    /// question in code, for the write path to branch on.
    pub fn is_anchored(self) -> bool {
        matches!(
            self,
            HumanFactKind::AccusationInstance | HumanFactKind::AnswerPairing
        )
    }

    /// The stable wire token, bound into `scenario_human_facts.kind`.
    pub fn code(self) -> &'static str {
        match self {
            HumanFactKind::Fact => "fact",
            HumanFactKind::WatchList => "watch_list",
            HumanFactKind::AccusationInstance => "accusation_instance",
            HumanFactKind::AnswerPairing => "answer_pairing",
        }
    }
}

/// The error produced when a stored kind token is not one this build knows.
#[derive(Debug, thiserror::Error)]
#[error(
    "unknown human-fact kind '{token}' — not one of \
     fact/watch_list/accusation_instance/answer_pairing"
)]
pub struct HumanFactKindParseError {
    pub token: String,
}

impl TryFrom<&str> for HumanFactKind {
    type Error = HumanFactKindParseError;

    fn try_from(token: &str) -> Result<Self, Self::Error> {
        match token {
            "fact" => Ok(HumanFactKind::Fact),
            "watch_list" => Ok(HumanFactKind::WatchList),
            "accusation_instance" => Ok(HumanFactKind::AccusationInstance),
            "answer_pairing" => Ok(HumanFactKind::AnswerPairing),
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
        // Task 2.11's two anchor kinds. These literals are load-bearing twice
        // over: the write path binds them, and the migration's two PARTIAL
        // UNIQUE INDEXES name them in their WHERE clauses. A rename here without
        // a migration would silently stop enforcing "one answer per accusation".
        assert_eq!(
            HumanFactKind::AccusationInstance.code(),
            "accusation_instance"
        );
        assert_eq!(HumanFactKind::AnswerPairing.code(), "answer_pairing");
        assert_eq!(HumanFactKind::ALL.len(), 4);
    }

    #[test]
    fn only_the_anchor_kinds_point_instead_of_speaking() {
        // The database asks this structurally ("says something OR points at
        // something") and names no kind, so this is the code half of the same
        // question. A fact or a watch-list note that reported itself anchored
        // would be allowed to store blank text and render as nothing.
        assert!(HumanFactKind::AccusationInstance.is_anchored());
        assert!(HumanFactKind::AnswerPairing.is_anchored());
        assert!(!HumanFactKind::Fact.is_anchored());
        assert!(!HumanFactKind::WatchList.is_anchored());
    }

    #[test]
    fn every_kind_round_trips_through_its_token() {
        // Write then read must land on the same variant for all four, or a
        // judgment stored today is unreadable tomorrow.
        for &kind in HumanFactKind::ALL {
            assert_eq!(
                HumanFactKind::try_from(kind.code()).expect("its own token decodes"),
                kind
            );
        }
    }

    #[test]
    fn an_unknown_kind_token_is_refused_and_named() {
        // Standing Rule 1: a kind this build does not define must not collapse to
        // `fact`, which would render somebody's pairing as a note in the wrong
        // block with nothing saying so.
        let error = HumanFactKind::try_from("rebuttal").expect_err("undefined kind");
        assert_eq!(error.token, "rebuttal");
        assert!(error.to_string().contains("rebuttal"));
    }
}
