// =============================================================================
// backend/src/domain/ruling_anchor.rs — the ruling-anchor vocabulary and the
// quote normalization law
// =============================================================================
//
// A RULING is a human decision about one candidate fact inside one scenario. An
// ANCHOR is the durable description of what that decision was made ABOUT:
// document, page, the verbatim words, and who said them.
//
// ## Domain note: why an anchor exists at all
//
// Rulings used to key on `scenario_fact_refs.graph_node_id` alone. That id is a
// CONTENT HASH: re-extract the document and every node gets a new one. On
// 2026-07-24/25 a re-extraction did exactly that and all 26 rulings on this case
// — including 6 human includes — silently became pointers to nodes that no longer
// existed. Nothing errored, because a pointer to a dead node is not a type error.
//
// The anchor is the fix, and its shape follows from the failure: it records the
// things that DID NOT change across the re-extraction. The document is still the
// same document; page 14 is still page 14; the witness still said those words.
// Those describe the RECORD. The node id describes the graph's current encoding
// of the record, which is ephemeral by law (§12.1). So rulings key to anchors,
// and the node id is kept only as provenance and as the matcher's fast path.
//
// This module owns the vocabulary and the normalization rule. It owns no I/O and
// no policy about WHEN a ruling may be written — that is the write path's job
// (`services::scenario_ruling`), which consults the per-kind rules below:
// `requires_document`, `requires_quote`, `requires_reason` and
// `tolerates_missing_node`. Those four encode the whole refusal law.

use serde::{Deserialize, Serialize};

/// The version of the ruling-anchor vocabulary THIS build defines.
///
/// Bumped whenever a [`RulingKind`] or [`FieldPresence`] token is added, removed,
/// or changes meaning — the same build-time coupling invariant as
/// `FACT_STATUS_LOOKUP_V` and `CONNECTION_TIER_LOOKUP_V`. A stored anchor row
/// carries no version today; when task 2.5 begins comparing anchors written by
/// different builds, this is the constant it will compare against.
pub const RULING_ANCHOR_LOOKUP_V: u32 = 1;

/// Which ruling an anchor row records.
///
/// ## Domain note: why this is NOT `FactStatus` and NOT `FactAction`
///
/// Three vocabularies meet at the ruling write path, and collapsing any two of
/// them would lose information:
///
/// * [`crate::dto::FactAction`] is what the HTTP client asks for (a verb).
/// * [`crate::domain::fact_status::FactStatus`] is the state the live row lands
///   in (`undecided` / `included` / `dropped`).
/// * `RulingKind` is what the LEDGER records — and it is the only one of the
///   three that can distinguish a *deliberate defer* from an untouched
///   candidate, because both of those are `FactStatus::Undecided`.
///
/// That last distinction is the entire reason defer is a first-class ruling: "I
/// looked at this and parked it, because …" and "nobody has looked at this" are
/// operationally different states, and the state column alone collapses them.
///
/// ## Rust Learning: `#[serde(rename_all = "snake_case")]` on a closed enum
///
/// Each variant maps to its snake_case token (`include`, `exclude`, `defer`,
/// `undrop`, `remove`). The enum is closed — no catch-all — so a token this build does not
/// define fails to deserialize rather than defaulting to something plausible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RulingKind {
    /// The human accepted this candidate as a fact of the scenario.
    Include,
    /// The human excluded it from THIS scenario (the graph node is untouched and
    /// still visible to other scenarios).
    Exclude,
    /// The human looked at it and parked it, with a stated reason. The live row
    /// stays `undecided`; the reason is what makes this distinguishable from a
    /// candidate nobody has touched.
    Defer,
    /// The human recovered a previously-excluded candidate back to the pool.
    ///
    /// RATIFIED 2026-08-01: §12.1's list of rulings is NON-EXHAUSTIVE — any verb
    /// that changes a candidate's ruling state writes a ledger row. Undrop
    /// qualifies: it moves a candidate OUT of excluded, and leaving it unrecorded
    /// would let a human reverse an exclusion with no trace, in the one table
    /// built to prevent silent state changes.
    Undrop,
    /// The human un-saved the reference entirely — `DELETE …/facts/:id`.
    ///
    /// ## Domain note: why a removal is a ruling, and why it can never be refused
    ///
    /// Under the same ratification, un-saving an include is a change to a human
    /// decision and therefore a ruling. It was the last traceless path in the
    /// system: the state row simply disappeared, leaving the include anchor in the
    /// ledger with nothing to say it had been withdrawn.
    ///
    /// It is also the one ruling that must never be blocked. A removal is an UNDO,
    /// and refusing it because the underlying evidence has since vanished would
    /// trap the human with a reference they can neither use nor discard. So
    /// [`RulingKind::requires_document`] and [`RulingKind::tolerates_missing_node`]
    /// both make an exception for it: a removal records whatever can still be read
    /// and marks the rest absent, rather than failing.
    ///
    /// Task 2.5 then needs no special case — reading the ledger forward
    /// distinguishes "included, still live" from "included, later removed".
    Remove,
}

impl RulingKind {
    /// The full, ordered vocabulary — the "extensible list in code."
    ///
    /// Adding a kind means adding a variant AND an entry here AND bumping
    /// [`RULING_ANCHOR_LOOKUP_V`]. Any caller enumerating the vocabulary reads it
    /// from ONE place rather than re-listing the variants.
    pub const ALL: &'static [RulingKind] = &[
        RulingKind::Include,
        RulingKind::Exclude,
        RulingKind::Defer,
        RulingKind::Undrop,
        RulingKind::Remove,
    ];

    /// The stable wire token, bound into `scenario_ruling_anchors.ruled_status`.
    pub fn code(self) -> &'static str {
        match self {
            RulingKind::Include => "include",
            RulingKind::Exclude => "exclude",
            RulingKind::Defer => "defer",
            RulingKind::Undrop => "undrop",
            RulingKind::Remove => "remove",
        }
    }

    /// Whether this ruling may only be made on an item with a source document.
    ///
    /// Every ruling except a removal: an item with no document is not record
    /// evidence, and a ruling on it could never be cited or re-anchored. A removal
    /// is the exception because it is an UNDO — see [`RulingKind::Remove`].
    ///
    /// Enforced here AND at the database: a column-level NOT NULL cannot express
    /// "except for removals", but the table-level CHECK added by migration
    /// `20260801130419` can, because it sees `ruled_status` beside `document_id`.
    /// This function refuses first, with a message; the CHECK catches anything
    /// that ever reaches the table without passing through here.
    pub fn requires_document(self) -> bool {
        match self {
            RulingKind::Include | RulingKind::Exclude | RulingKind::Defer | RulingKind::Undrop => {
                true
            }
            RulingKind::Remove => false,
        }
    }

    /// Whether this ruling may still be recorded when the graph no longer holds
    /// the item at all.
    ///
    /// Only a removal. For every other ruling a missing node is a refusal: writing
    /// a ruling whose anchor cannot be read is exactly the 2026-07-24 failure, and
    /// returning success would recreate it. For a removal the opposite is true —
    /// the vanished node is often WHY the human is removing the reference, so
    /// refusing would strand them. The removal is recorded with every anchor field
    /// marked absent, which is a complete and honest record of what happened.
    pub fn tolerates_missing_node(self) -> bool {
        matches!(self, RulingKind::Remove)
    }

    /// Whether this ruling may only be made on a candidate that carries a
    /// verbatim quote.
    ///
    /// ## Domain note: the citability law (v2 §9/§17)
    ///
    /// An item that cannot be quoted cannot be cited, and cannot be durably
    /// re-anchored — the quote is the primary thing task 2.5 will match on. So an
    /// unquotable INCLUDE is a rehearsal liability by law: it would sit in the
    /// scenario looking curated while being unusable in the room and unrecoverable
    /// after a re-extraction. Exclude is held to the same bar because an exclusion
    /// is a decision the case may later have to defend.
    ///
    /// Defer and undrop are deliberately exempt. Deferring is precisely how a
    /// human parks an item they cannot cite — refusing the defer too would leave
    /// them with no legal move at all, and the refusal message says so.
    ///
    /// ## Rust Learning: an exhaustive `match` as the compiler's checklist
    ///
    /// No `_ =>` arm, on purpose: adding a fifth `RulingKind` stops THIS function
    /// compiling until someone decides whether the new kind needs a quote. A
    /// catch-all would silently answer that question for them, and the wrong
    /// answer here is either a blocked human or an uncitable include.
    pub fn requires_quote(self) -> bool {
        match self {
            RulingKind::Include | RulingKind::Exclude => true,
            RulingKind::Defer | RulingKind::Undrop | RulingKind::Remove => false,
        }
    }

    /// Whether this ruling requires a stated reason.
    ///
    /// Only defer does. A defer with no reason is not a defer — it is an
    /// unexplained non-decision, indistinguishable on read from a candidate nobody
    /// touched, which defeats the point of making defer a ruling at all. The
    /// database backstops this with a CHECK tying `defer_reason IS NOT NULL` to
    /// `ruled_status = 'defer'` in both directions.
    pub fn requires_reason(self) -> bool {
        matches!(self, RulingKind::Defer)
    }
}

/// Whether an optional anchor field was present in the record at ruling time.
///
/// ## Domain note: why a bare NULL is not good enough
///
/// Page and speaker are legitimately absent for real evidence — documentary
/// evidence has no speaker, and some extractions carry no page. But a NULL column
/// cannot say WHY it is empty, and "this evidence genuinely has none" is
/// operationally different from "our capture missed it". Standing Rule 1 wants
/// those distinguishable, so absence is stored as a RECORDED state beside the
/// value rather than inferred from its emptiness.
///
/// Note there are exactly two states, not three. A "not captured" variant would be
/// unreachable: the anchor is read server-side from the graph at ruling time
/// (never supplied by the client), so if the field is absent, the record does not
/// have it. Adding a third state would be modelling a failure mode the write path
/// makes impossible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldPresence {
    /// The record carried this field; the value column holds it.
    Present,
    /// The record genuinely does not carry this field; the value column is NULL.
    None,
}

impl FieldPresence {
    /// The full vocabulary — see [`RulingKind::ALL`] for why this exists.
    pub const ALL: &'static [FieldPresence] = &[FieldPresence::Present, FieldPresence::None];

    /// The stable wire token, bound into the `*_state` columns.
    pub fn code(self) -> &'static str {
        match self {
            FieldPresence::Present => "present",
            FieldPresence::None => "none",
        }
    }

    /// Classify an optional value into its presence state.
    ///
    /// ## Rust Learning: a generic over `Option<T>` with no trait bounds
    ///
    /// `T` is unconstrained because this function only inspects the Option's
    /// SHAPE, never the value inside — it needs no `Display`, no `Clone`, nothing.
    /// An unbounded generic is the type-level statement "I do not care what is in
    /// here", which is exactly true and lets one function serve the page
    /// (`Option<i64>`), the speaker (`Option<String>`) and the quote.
    pub fn of<T>(value: &Option<T>) -> Self {
        match value {
            Some(_) => FieldPresence::Present,
            None => FieldPresence::None,
        }
    }
}

/// Normalize a verbatim quote for durable matching: casefold, then collapse
/// whitespace.
///
/// Returns the form stored in `scenario_ruling_anchors.quote_normalized`, ALONGSIDE
/// the verbatim text — never instead of it. The verbatim quote is what gets read
/// aloud in a courtroom; this is only a comparison surface.
///
/// ## Domain note: exactly two operations, and why not more
///
/// §12.1 names casefolding and whitespace collapse. It names nothing else, and
/// this function deliberately does nothing else — notably it does NOT fold curly
/// quotes to straight ones, strip punctuation, or normalize Unicode forms, even
/// though each would plausibly help the task-2.5 matcher.
///
/// The reason is that normalization is LAW, not a tuning knob (v2 §2b: this task
/// introduces no parameters). Every operation added here changes which historical
/// anchors match — silently, and retroactively, since stored normalized text was
/// written under the old rule. Widening the rule is therefore a deliberate,
/// versioned change with a `RULING_ANCHOR_LOOKUP_V` bump and a re-normalization of
/// stored rows, not a quiet edit to this function. The unicode-quote test below
/// pins the current behaviour so that widening cannot happen by accident.
///
/// ## Rust Learning: `split_whitespace` does the collapsing for free
///
/// `split_whitespace()` splits on any run of Unicode whitespace and yields no
/// empty pieces, so joining its output with a single space collapses internal runs
/// AND trims both ends in one pass — no `trim()`, no regex, no manual state
/// machine. It understands non-breaking spaces and other Unicode whitespace that a
/// naive `split(' ')` would miss, which matters for text that came out of a PDF.
///
/// `to_lowercase()` (not `to_ascii_lowercase()`) is Unicode-aware, so accented
/// capitals in a name fold correctly. It is a true casefold for the Latin text
/// this corpus contains.
pub fn normalize_quote(raw: &str) -> String {
    raw.split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ruling_tokens_match_serde() {
        for &kind in RulingKind::ALL {
            assert_eq!(json!(kind), json!(kind.code()));
        }
        for &state in FieldPresence::ALL {
            assert_eq!(json!(state), json!(state.code()));
        }
    }

    #[test]
    fn all_lists_every_ruling_kind_once() {
        assert_eq!(RulingKind::ALL.len(), 5);
        for &kind in RulingKind::ALL {
            assert_eq!(
                RulingKind::ALL.iter().filter(|k| **k == kind).count(),
                1,
                "{} is listed more than once",
                kind.code()
            );
        }
    }

    #[test]
    fn rejects_unknown_ruling_token() {
        // Standing Rule 1: a ruling this build does not define is a LOUD parse
        // error, never a silent default to the most permissive kind.
        let result: Result<RulingKind, _> = serde_json::from_value(json!("archive"));
        assert!(result.is_err(), "an unknown ruling token must not parse");
    }

    #[test]
    fn only_include_and_exclude_require_a_quote() {
        // The citability law, pinned. If someone exempts include from the quote
        // requirement, an uncitable fact can enter a scenario and this fails.
        assert!(RulingKind::Include.requires_quote());
        assert!(RulingKind::Exclude.requires_quote());
        // Defer must stay exempt: it is the human's legal move for an item that
        // cannot be quoted. Requiring a quote here would leave them stuck.
        assert!(!RulingKind::Defer.requires_quote());
        assert!(!RulingKind::Undrop.requires_quote());
        // A removal is exempt for the same reason it is exempt from the document
        // requirement: it is an undo, and an item that cannot be quoted is often
        // precisely the one a human wants to discard.
        assert!(!RulingKind::Remove.requires_quote());
    }

    #[test]
    fn only_a_removal_may_lack_a_document() {
        // Hardcoded per variant, deliberately NOT driven off `requires_document`
        // itself: a test that asks the production function what to expect agrees
        // with it even when it is wrong. These are the expected values written out
        // by hand, so a flipped arm fails here.
        assert!(RulingKind::Include.requires_document());
        assert!(RulingKind::Exclude.requires_document());
        assert!(RulingKind::Defer.requires_document());
        assert!(RulingKind::Undrop.requires_document());
        // The one exception, and the reason it exists: a removal is an undo, and a
        // human must always be able to discard a reference to something that has
        // gone — including something that never had a document.
        assert!(!RulingKind::Remove.requires_document());
    }

    #[test]
    fn only_a_removal_tolerates_a_vanished_node() {
        // Same discipline as above: written out, not derived. Getting this wrong in
        // the permissive direction would let an include be written with an anchor
        // that was never read — the 2026-07-24 failure exactly.
        assert!(!RulingKind::Include.tolerates_missing_node());
        assert!(!RulingKind::Exclude.tolerates_missing_node());
        assert!(!RulingKind::Defer.tolerates_missing_node());
        assert!(!RulingKind::Undrop.tolerates_missing_node());
        assert!(RulingKind::Remove.tolerates_missing_node());
    }

    #[test]
    fn only_defer_requires_a_reason() {
        assert!(RulingKind::Defer.requires_reason());
        assert!(!RulingKind::Include.requires_reason());
        assert!(!RulingKind::Exclude.requires_reason());
        assert!(!RulingKind::Undrop.requires_reason());
        assert!(!RulingKind::Remove.requires_reason());
    }

    #[test]
    fn presence_classifies_by_shape_not_by_value() {
        assert_eq!(FieldPresence::of(&Some(14_i64)), FieldPresence::Present);
        assert_eq!(FieldPresence::of(&None::<i64>), FieldPresence::None);
        // An EMPTY string is still Present: the record carried a value, and
        // "carried an empty speaker" is a data-quality fact the anchor should
        // record honestly rather than silently rewrite to "no speaker".
        assert_eq!(
            FieldPresence::of(&Some(String::new())),
            FieldPresence::Present
        );
    }

    // ── normalize_quote ───────────────────────────────────────────────────────

    #[test]
    fn normalization_casefolds() {
        assert_eq!(normalize_quote("I Do Not Recall"), "i do not recall");
    }

    #[test]
    fn normalization_collapses_and_trims_whitespace() {
        // Internal runs collapse to one space; leading/trailing vanish. This is the
        // case that matters most in practice: PDF extraction routinely yields
        // double spaces and line-wrap newlines inside a single quoted sentence, and
        // without this the same sentence would fail to match itself.
        assert_eq!(
            normalize_quote("  the   witness\n\tsaid   so  "),
            "the witness said so"
        );
    }

    #[test]
    fn normalization_handles_unicode_whitespace() {
        // A non-breaking space is whitespace to `split_whitespace` but not to a
        // naive `split(' ')`. PDFs emit these; a quote containing one must
        // normalize identically to the same quote with an ordinary space.
        assert_eq!(
            normalize_quote("page\u{00A0}14"),
            normalize_quote("page 14")
        );
    }

    #[test]
    fn normalization_preserves_unicode_quotes_verbatim() {
        // PINNED BEHAVIOUR, not an oversight: curly quotes and apostrophes survive
        // normalization unchanged, because the law names casefolding and whitespace
        // collapse and nothing else.
        //
        // The consequence is real and belongs on the record: a quote stored with a
        // curly apostrophe will NOT match the same sentence typed with a straight
        // one. If task 2.5 needs that equivalence, widening this function is a
        // LAW change — a deliberate edit here plus a RULING_ANCHOR_LOOKUP_V bump
        // plus re-normalizing stored rows — never a quiet fix inside the matcher.
        assert_eq!(
            normalize_quote("I don\u{2019}t recall"),
            "i don\u{2019}t recall"
        );
        assert_ne!(
            normalize_quote("I don\u{2019}t recall"),
            normalize_quote("I don't recall"),
            "curly and straight apostrophes are deliberately NOT unified; if this \
             ever becomes desirable it is a versioned law change, and this \
             assertion is the tripwire that forces the conversation"
        );
    }

    #[test]
    fn normalization_of_whitespace_only_input_is_empty() {
        // A quote that is nothing but whitespace normalizes to "" — which the write
        // path treats as no usable quote at all, so an include carrying it is
        // refused rather than anchored to an empty comparison surface.
        assert_eq!(normalize_quote("   \n\t "), "");
    }

    #[test]
    fn normalization_is_idempotent() {
        // Normalizing an already-normalized quote must change nothing. Task 2.5
        // will normalize incoming candidate text and compare it against stored
        // normalized text; if the function were not idempotent those two would
        // drift apart after any re-processing.
        let once = normalize_quote("  The  WITNESS said\u{00A0}so ");
        assert_eq!(normalize_quote(&once), once);
    }
}
