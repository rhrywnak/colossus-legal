//! What the read is GIVEN: the payload, its keys, and its named absences.
//!
//! Pure. No provider, no database, no clock — so every rule below is a unit test
//! instead of a call that costs money to observe. The impure half, which reads
//! the store and decides whether a missing value is legitimate, is
//! [`super::practice_read_gather`]; the reply half is
//! [`super::practice_read_parse`].
//!
//! ## Why this module exists at all (the defect it closes)
//!
//! Until T1 the model was told to judge whether Marie had anchored her answer in
//! a document, and was sent no document. Prompt v2 ordered *"the short counter
//! PLUS ONE ANCHOR — the receipt, named"* while its own rules forbade inventing
//! one, and `read_for` dropped the `exhibit` column on the floor before the call.
//! The read could only score an anchor when Marie happened to type a date herself.
//! That is not a prompt bug — the prompt was asking for something true — it is a
//! PAYLOAD bug, and this is the payload.
//!
//! ## Domain note: what is NOT in here, and why it is a rule and not an omission
//!
//! No case summary, no document text, no other scenario, no graph node, no
//! corpus. The model sees one question, one answer, three sentences she wrote,
//! their receipts, one sworn pair and one watch-for. That is "nothing reads the
//! whole graph" (design §5) expressed as an LLM input rather than as a query, and
//! it is why [`ReadPayload`] is a CLOSED struct with no `HashMap` and no
//! `serde_json::Value` escape hatch: nothing can reach the message except through
//! a field somebody added on purpose.
//!
//! `stronger` and `stronger_lean` sit on the very record the gatherer holds and
//! are deliberately absent — the model answer must not be visible to the thing
//! judging her attempt at it. A test asserts the WHOLE BODY, so re-adding either
//! fails the build rather than passing unnoticed.

use std::collections::BTreeSet;

/// What one point or receipt is, and the key it is cited by.
///
/// ## Domain note: a key with no text is NOT citable
///
/// `R2` with nothing behind it is printed — a named absence, never a silent gap
/// — but it does not enter [`ReadPayload::citable_keys`]. A model that cites it
/// is claiming a receipt exists when none does, which is the exact failure this
/// task was written to stop, so the citation check must catch it rather than wave
/// it through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keyed {
    /// `P1`, `R3`, `S2` — the key as the model must write it.
    pub key: String,
    /// The text behind the key, or `None` when nobody has authored one.
    pub text: Option<String>,
}

impl Keyed {
    /// One keyed item, from a key and an optional text.
    pub fn new(key: impl Into<String>, text: Option<String>) -> Self {
        Keyed {
            key: key.into(),
            text,
        }
    }
}

/// Whether this question carries a tactic, said in the model's own terms.
///
/// ## Rust Learning: a two-variant enum instead of `Option<String>`
///
/// This WAS an `Option<String>`, and that is precisely how the live defect got
/// in. `tactic_name` returns `None` for two unrelated reasons — the question
/// genuinely has no tactic (every Chuck question), or the stored vocabulary is
/// too short to name the card this question DOES carry — and the old code turned
/// both into the sentence "none — this is a direct question". On a cross question
/// with a trimmed `practice_tactic_names` row, that sentence is a lie told to the
/// model about the question it is judging.
///
/// An enum makes the two cases un-collapsible: the type has no third state, and
/// the unresolvable case never reaches this module at all, because the gatherer
/// turns it into an abstain before a payload is built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tactic {
    /// The card's own name, from the stored vocabulary.
    Named(String),
    /// The question carries none — as Chuck's direct and redirect questions do.
    /// **[measured 2026-08-20: 20 of the 30 live deck rows.]**
    NoneByDesign,
}

/// What she said she would point to, as three distinguishable facts.
///
/// ## Domain note: "never opened" and "opened and picked nothing" are different
///
/// The column keeps them apart (`points_to` is `NULL` versus `[]`) because
/// collapsing them would tell Chuck she considered the exhibits and reached for
/// none, on an answer where she never saw the list. The payload keeps them apart
/// for the same reason: a model told "she picked nothing" reads a deliberate
/// choice into an absence that was never a choice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PointsTo {
    /// The control was never opened.
    NeverOpened,
    /// She opened it and picked nothing.
    OpenedAndPickedNothing,
    /// The receipts she ticked, as the phrases she was shown.
    Picked(Vec<String>),
}

/// Everything the model is told about one answer.
///
/// Owned rather than borrowed (this was `ReadInputs<'a>`): the gatherer that
/// fills it and the composer that reads it now live in different modules, and a
/// lifetime spanning that seam would buy one avoided clone per answer at the cost
/// of threading `'a` through an async call. One read per typed answer is not a
/// place to spend a lifetime parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadPayload {
    pub question: String,
    pub side: String,
    /// `cross`, `direct` or `redirect`. Sent as itself rather than inferred from
    /// `side`, which cannot tell Chuck's two apart — and the three are judged by
    /// different rules.
    pub kind: String,
    pub tactic: Tactic,
    pub answer: String,
    /// Her points, keyed `P1`…`Pn`. Empty is legitimate (Roman, A2).
    pub points: Vec<Keyed>,
    /// The receipts behind those points, keyed `R1`…`Rn`.
    pub receipts: Vec<Keyed>,
    /// `S1` — what they said. `None` on every question with no sworn pair.
    pub said: Option<String>,
    /// `S2` — what they admitted under oath.
    pub admitted: Option<String>,
    pub points_to: PointsTo,
    pub watch_for: Option<String>,
    pub always: String,
}

// The named absences. Literals rather than settings rows because NOBODY READS
// THEM ON A SCREEN — they are this build's own words about its own data, spoken
// to a model, and task §2.1 endorses the convention by name. The rule they serve
// is the honest-gap law: an absent value is SAID, never omitted, because a field
// that simply vanishes from the message is indistinguishable to the model from a
// field this build forgot to send.
const NO_TACTIC: &str = "none — this question carries no tactic";
const NO_WATCH_FOR: &str = "(no watch-for was written for this question)";
const NO_POINTS: &str = "(none recorded)";
const NO_RECEIPT: &str = "(none recorded)";
const NO_PAIR: &str = "(no sworn pair is recorded for this question)";
const POINTS_TO_NEVER_OPENED: &str = "(she did not open the exhibit list)";
const POINTS_TO_PICKED_NOTHING: &str = "(she opened the exhibit list and picked nothing)";

impl ReadPayload {
    /// The keys a citation may legitimately name.
    ///
    /// ## Why this is computed from the payload and never written by hand
    ///
    /// It is the same set twice: the line the prompt shows the model, and the set
    /// the parser validates its answer against. Deriving both from one function
    /// is what makes "the model may cite only what it was sent" a property of the
    /// code rather than a promise in prose — and it is why a key whose text is
    /// absent is excluded here rather than filtered at the far end, where the two
    /// lists could drift.
    ///
    /// ## Rust Learning: `BTreeSet` and not `HashSet`
    ///
    /// `BTreeSet` iterates in sorted order, so the prompt's key line is
    /// byte-identical for the same payload every time. A `HashSet` would reorder
    /// it per process, which would make the sent prompt unstable for no reason
    /// and any diff of two payloads unreadable.
    pub fn citable_keys(&self) -> BTreeSet<String> {
        // Derived from `citable_sources`, never assembled a second time. See
        // that function for why.
        self.citable_sources()
            .into_iter()
            .map(|(key, _)| key)
            .collect()
    }

    /// Every citable key WITH the words behind it, in the prompt's own order.
    ///
    /// ## ⚑ THE ONE AUTHORITY. `citable_keys` is derived from this.
    ///
    /// There used to be two: this set was computed here, and the critique's
    /// footnote list was assembled by hand from `points` and `receipts`. They
    /// disagreed — the hand-built one omitted the sworn pair — so a read could
    /// cite `S2` and the screen would show that key with NOTHING under it, on
    /// every question carrying a sworn pair. Silent, and reachable in normal use.
    ///
    /// One function now decides what a key means, so the next key type cannot
    /// be added to one list and forgotten in the other. A caller that wants only
    /// the names takes them from here; a caller that wants the words takes them
    /// from here.
    ///
    /// ## Domain note: a key with NO text is not citable
    ///
    /// Excluded at the source rather than filtered at the far end — the model is
    /// never told a key it cannot be shown the words for, so an empty footnote
    /// is unreachable by construction rather than by care.
    ///
    /// ## Rust Learning: `BTreeMap`-like ordering from a sorted `Vec`
    ///
    /// Returned sorted, for the reason `citable_keys` used a `BTreeSet`: the
    /// prompt's key line must be byte-identical for the same payload every time,
    /// or two payloads cannot be diffed and the sent prompt wobbles for no
    /// reason.
    pub fn citable_sources(&self) -> Vec<(String, String)> {
        let mut sources: Vec<(String, String)> = Vec::new();
        for item in self.points.iter().chain(self.receipts.iter()) {
            if let Some(text) = item.text.as_ref() {
                sources.push((item.key.clone(), text.clone()));
            }
        }
        if let Some(text) = self.said.as_ref() {
            sources.push(("S1".to_string(), text.clone()));
        }
        if let Some(text) = self.admitted.as_ref() {
            sources.push(("S2".to_string(), text.clone()));
        }
        sources.sort_by(|a, b| a.0.cmp(&b.0));
        sources
    }
}

/// Render one keyed block — `P1. …`, or the named absence.
fn keyed_block(items: &[Keyed], empty: &str, absent: &str) -> String {
    if items.is_empty() {
        return empty.to_string();
    }
    items
        .iter()
        .map(|item| match &item.text {
            Some(text) => format!("{}. {text}", item.key),
            None => format!("{}. {absent}", item.key),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Compose the user message.
///
/// One `format!` with named substitutions, deliberately: this is the ONLY place
/// the user half of the payload is assembled, and a reader auditing what a model
/// was told should be able to see all of it at once rather than reconstruct it
/// from a builder.
pub fn build_user_message(payload: &ReadPayload) -> String {
    let tactic = match &payload.tactic {
        Tactic::Named(name) => name.as_str(),
        Tactic::NoneByDesign => NO_TACTIC,
    };
    let watch = payload.watch_for.as_deref().unwrap_or(NO_WATCH_FOR);
    let points = keyed_block(&payload.points, NO_POINTS, NO_RECEIPT);
    let receipts = keyed_block(&payload.receipts, NO_POINTS, NO_RECEIPT);
    // Half a pair is a data defect, not a load failure (Roman, A3): the half that
    // exists is sent under its own key and the other is named absent, so a read
    // is still possible and the gap is visible rather than papered over.
    let said = payload.said.as_deref().unwrap_or(NO_PAIR);
    let admitted = payload.admitted.as_deref().unwrap_or(NO_PAIR);
    let points_to = match &payload.points_to {
        PointsTo::NeverOpened => POINTS_TO_NEVER_OPENED.to_string(),
        PointsTo::OpenedAndPickedNothing => POINTS_TO_PICKED_NOTHING.to_string(),
        PointsTo::Picked(picked) => picked.join(" · "),
    };
    let keys = payload
        .citable_keys()
        .into_iter()
        .collect::<Vec<_>>()
        .join(" ");

    format!(
        "THE QUESTION ({side}): {question}\n\
         THE KIND: {kind}\n\
         THE TACTIC: {tactic}\n\n\
         HER ANSWER, verbatim:\n{answer}\n\n\
         HER THREE POINTS:\n{points}\n\n\
         THE RECEIPTS BEHIND HER POINTS:\n{receipts}\n\n\
         WHAT THEY SAID:\nS1. {said}\n\n\
         WHAT THEY ADMITTED UNDER OATH:\nS2. {admitted}\n\n\
         WHAT SHE SAID SHE WOULD POINT TO: {points_to}\n\n\
         THE WATCH-FOR: {watch}\n\n\
         THE ALWAYS CARD: {always}\n\n\
         THE KEYS YOU MAY CITE: {keys}\n",
        side = payload.side,
        kind = payload.kind,
        question = payload.question,
        answer = payload.answer,
        always = payload.always,
    )
}

#[cfg(test)]
#[path = "practice_read_payload_tests.rs"]
mod tests;
