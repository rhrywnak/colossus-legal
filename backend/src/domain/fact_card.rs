// =============================================================================
// backend/src/domain/fact_card.rs — the five sentences on a witness's card
// =============================================================================
//
// FACT_CARD_v2 §1 and §2. This module owns three closed vocabularies and one
// cap:
//
//   * [`CardField`]  — which of the five fields an edit addresses.
//   * [`CardStance`] — which way a card cuts against one accusation.
//   * [`CardAuthor`] — whether a field is still the machine's draft.
//   * [`SUPPORTS_CAP`] — at most two accusations per card.
//
// ## Why the vocabularies live here and not in a database CHECK
//
// The house argument, and it is sharper on this table than most: a CHECK cannot
// tell you the PARSER is missing. A `field` token Postgres accepts but this build
// cannot read would be a ledger row nobody can attribute, on the one record that
// says a human replaced a machine's words.
//
// ## Domain note: a machine-authored field is a DRAFT, not a fact
//
// Job B wrote 59 of these cards. Every one is a model's guess at what a witness
// should say, and §2 puts a grey "draft" mark on each until a human edits it.
// [`CardAuthor::is_draft`] is that rule, stated once, so the mark, the export and
// any future readiness check cannot disagree about which fields count as
// prepared.

use serde::{Deserialize, Serialize};

/// How many accusations one card may name (§1).
///
/// ## Rust Learning: `const` for a structural cap, not a tunable
///
/// Standing Rule 2 governs values that vary across environments, cases or
/// configurations — this varies across none of them. It is a property of the
/// CARD's layout: the Supports row is one line a witness reads at a glance, and
/// three accusations on it stops being readable. Job B was given the same cap and
/// reports `over_supports_cap` when it had to drop one, so the number is already
/// baked into the input this loader reads; making it editable would let a setting
/// silently disagree with the data on disk.
// STRUCTURAL: a layout property of the card, not a tunable — see the doc comment
// above for why Standing Rule 2 does not reach it.
pub const SUPPORTS_CAP: usize = 2;

/// Which field of a card an edit addresses.
///
/// ## Domain note: `Card` is not a sixth field
///
/// It is the ledger's word for "the loader wrote this whole row at once", which
/// is a different act from a human rewriting one sentence and has to read
/// differently in the record. It is deliberately NOT accepted by the edit route:
/// a client that could PUT `card` would replace five fields in one write and the
/// ledger would say only that something happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardField {
    /// What Marie says in three seconds.
    Title,
    /// Which talking point this card backs, by position.
    BacksPosition,
    /// Which accusations it goes to, and which way.
    Supports,
    /// How the other side uses it.
    WatchOut,
    /// Her reply.
    Answer,
    /// The whole row, written by the loader. Ledger-only — see the type doc.
    Card,
}

impl CardField {
    /// The five fields a HUMAN may edit, in the order the card renders them.
    ///
    /// `Card` is absent, and that absence is the edit route's whole allowlist.
    pub const EDITABLE: &'static [CardField] = &[
        CardField::Title,
        CardField::BacksPosition,
        CardField::Supports,
        CardField::WatchOut,
        CardField::Answer,
    ];

    /// Every token this build can read, including the ledger-only one.
    pub const ALL: &'static [CardField] = &[
        CardField::Title,
        CardField::BacksPosition,
        CardField::Supports,
        CardField::WatchOut,
        CardField::Answer,
        CardField::Card,
    ];

    /// The token stored in `scenario_fact_card_events.field`.
    ///
    /// It is also the COLUMN name of the field itself and of its authorship
    /// column with `_authored_by` appended — deliberately, so the writer never
    /// builds a column name from a second mapping that could drift from this one.
    pub fn code(self) -> &'static str {
        match self {
            CardField::Title => "title",
            CardField::BacksPosition => "backs_position",
            CardField::Supports => "supports",
            CardField::WatchOut => "watch_out",
            CardField::Answer => "answer",
            CardField::Card => "card",
        }
    }

    /// Whether a human may PUT this field.
    pub fn is_editable(self) -> bool {
        !matches!(self, CardField::Card)
    }

    /// The Postgres cast the writer must apply to this field's bound value.
    ///
    /// ## Why every field is bound as TEXT and cast here
    ///
    /// One statement serves all five columns, and the alternative was five bind
    /// types and five near-identical upserts. The columns are not all TEXT,
    /// though — `backs_position` is INTEGER and `supports` is JSONB — so the
    /// difference has to live somewhere, and it lives HERE, on the enum that
    /// already owns the column NAME.
    ///
    /// ## Domain note: this was a real defect, found by a live run
    ///
    /// `backs_position` had no cast and the writer bound a string. Every
    /// SQL-SHAPE test passed — they compare statements, and the statement was
    /// well-formed — and Postgres refused it on the first `--apply` against a
    /// real file: *column "backs_position" is of type integer but expression is
    /// of type text*. The cast is a property of the FIELD, so it is asserted per
    /// field below rather than spelled at the one call site that needs it.
    pub fn sql_cast(self) -> &'static str {
        match self {
            CardField::BacksPosition => "::integer",
            CardField::Supports => "::jsonb",
            // The three sentence fields are TEXT columns and a TEXT bind, so a
            // cast would be noise. `Card` is not a column at all.
            CardField::Title | CardField::WatchOut | CardField::Answer | CardField::Card => "",
        }
    }
}

/// The error produced when a stored or submitted field token is unknown.
#[derive(Debug, thiserror::Error, PartialEq)]
#[error("unknown card field '{token}' — this build knows title/backs_position/supports/watch_out/answer")]
pub struct CardFieldParseError {
    pub token: String,
}

impl TryFrom<&str> for CardField {
    type Error = CardFieldParseError;

    fn try_from(token: &str) -> Result<Self, Self::Error> {
        CardField::ALL
            .iter()
            .copied()
            .find(|f| f.code() == token)
            .ok_or_else(|| CardFieldParseError {
                token: token.to_string(),
            })
    }
}

/// Which way a card cuts against one accusation.
///
/// ## Domain note: two words, not a boolean
///
/// The verb is printed — "Supports A-22", "Disputes A-22" — and a reader skimming
/// five cards must not have to parse a negation. A `bool` would also make the
/// stored JSON say `{"supports": false}`, which reads as an absence rather than
/// as the opposite claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardStance {
    /// The card helps the accusation stand.
    Supports,
    /// The card cuts against it.
    ///
    /// Domain note: the token is `rebuts`, not `disputes`. That is the word the
    /// GRAPH's own `r.stance` carries (802 `supports` / 142 `rebuts`, measured)
    /// and the word Job B wrote into the 59 drafted cards on disk. A prettier
    /// synonym here would have made every one of those files unreadable — it did,
    /// on the first dry run.
    Rebuts,
}

impl CardStance {
    /// The full vocabulary — the "extensible list in code".
    pub const ALL: &'static [CardStance] = &[CardStance::Supports, CardStance::Rebuts];

    /// The token stored inside `scenario_fact_cards.supports`.
    pub fn code(self) -> &'static str {
        match self {
            CardStance::Supports => "supports",
            CardStance::Rebuts => "rebuts",
        }
    }

    /// The corresponding `evidence_allegation_links.cut` (§2).
    ///
    /// ## Domain note: two vocabularies, one act, and the map is here
    ///
    /// Including a fact writes BOTH a card stance and a link cut. They are not the
    /// same words — a cut says which way a statement runs for OUR SIDE
    /// (`supports` / `against`), a stance says which way it runs for the
    /// ACCUSATION — and the mapping is stated once, here, rather than at the call
    /// site where a later reader would have to re-derive it.
    ///
    /// A card that SUPPORTS an accusation against Marie is a HAZARD, so it maps to
    /// `Against`; one that REBUTS the accusation helps her, so `Supports`.
    pub fn to_link_cut(self) -> crate::domain::link_cut::LinkCut {
        match self {
            CardStance::Supports => crate::domain::link_cut::LinkCut::Against,
            CardStance::Rebuts => crate::domain::link_cut::LinkCut::Supports,
        }
    }
}

/// The error produced when a stored or submitted stance token is unknown.
#[derive(Debug, thiserror::Error, PartialEq)]
#[error("unknown card stance '{token}' — a card cuts one of two ways: supports/rebuts")]
pub struct CardStanceParseError {
    pub token: String,
}

impl TryFrom<&str> for CardStance {
    type Error = CardStanceParseError;

    fn try_from(token: &str) -> Result<Self, Self::Error> {
        CardStance::ALL
            .iter()
            .copied()
            .find(|s| s.code() == token)
            .ok_or_else(|| CardStanceParseError {
                token: token.to_string(),
            })
    }
}

/// The `authored_by` value the loader stamps on every field it writes.
// STRUCTURAL: a provenance token this build and every ledger row share, in the
// same category as a status word — not a deployment value. Changing it would
// orphan every draft mark the loader has already written.
pub const MACHINE_AUTHOR: &str = "machine:job_b_v1";

/// The prefix that marks any authorship value as a machine's.
// STRUCTURAL: the prefix half of the token above. A second drafting job would
// write `machine:job_c_v1` and must still read as a draft, so the prefix has to
// stay stable; it is not chosen deployment by deployment.
pub const MACHINE_AUTHOR_PREFIX: &str = "machine:";

/// Who wrote one field.
///
/// ## Rust Learning: a newtype over `&str` instead of a bare string test
///
/// The rule "a field is a draft when its author is a machine" is one comparison,
/// and it is read in three places — the card payload, the rehearsal page and the
/// loader's own reporting. Written inline it would be three chances to test the
/// wrong prefix; as a type it is one function with one test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardAuthor<'a>(pub &'a str);

impl CardAuthor<'_> {
    /// Whether this field is still a machine's draft.
    ///
    /// Matches on the PREFIX, not on the whole token: a second drafting pass
    /// would write `machine:job_c_v1`, and a card that stopped showing the draft
    /// mark because the job name changed would silently claim a human had been
    /// through it.
    pub fn is_draft(self) -> bool {
        self.0.starts_with(MACHINE_AUTHOR_PREFIX)
    }
}

#[cfg(test)]
#[path = "fact_card_tests.rs"]
mod tests;
