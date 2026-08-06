//! Wire shapes for authoring the accusation and its answers (task 2.11 B1).
//!
//! One read payload and two request bodies. The read is what the working view's
//! accusation section renders; the requests are what its three write concerns
//! send.
//!
//! ## Every sentence arrives composed, and the templates deliberately do not
//!
//! The count line, the three gap messages and the no-instances notice are built
//! on THIS side of the wire and cross it as finished sentences. The templates
//! they were built from are not sent at all.
//!
//! That is not tidiness, it is the §10 exclusion made structural. A browser
//! holding `accusation_count_template` would need the instance list and the
//! document map to fill it, and would then be holding exactly the data the
//! rehearsal page is forbidden to render. A browser that receives "Said 3 times,
//! in 2 documents." can render that and nothing else.
//!
//! The one template that DOES cross is [`AccusationWordingDto::save_failed_template`],
//! because its `{detail}` is the failure's own text — the single value that exists
//! only in the browser.
//!
//! ## Domain note: no verdicts, no confidence, no notes, no internal ids
//!
//! Nothing here carries a tier, an ordinal, a confidence, a status, a human's
//! annotation, or a database id. The anchors are graph node ids (§12) because that
//! is what a judgment is keyed to and what a click sends back; everything else a
//! human reads about a fact they already have from the cards payload.

use serde::{Deserialize, Serialize};

/// One marked instance, and whatever a human paired to it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccusationInstanceDto {
    /// The statement that IS them making the accusation.
    pub graph_node_id: String,
    /// The handle a human calls it by — `"C-14"`, or `null` for a candidate
    /// nothing has numbered yet. `null` rather than the node id: a code that is
    /// really an id would read as a handle and be quoted as one out loud.
    pub code: Option<String>,
    /// The record item paired as our answer, or `null` when nobody has paired one.
    pub answers_graph_node_id: Option<String>,
    /// That answer's handle, on the same terms as `code`.
    pub answer_code: Option<String>,
    /// Whether the paired answer is still in the scenario.
    ///
    /// `false` with a non-null `answers_graph_node_id` is the Remove law's visible
    /// broken pairing — the row is kept, and the matching gap below says so. The
    /// client is TOLD this rather than asked to infer it by hunting the gap list,
    /// for the same reason `status` travels beside `status_label`.
    pub answer_present: bool,
}

/// One named absence, ready to render.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccusationGapDto {
    /// A stable token — `no_answer_prepared`, `accusation_removed`,
    /// `answer_removed`.
    ///
    /// ## Why a token travels beside the sentence
    ///
    /// So the client can treat the prep list differently from a broken pairing
    /// (louder, first) WITHOUT parsing prose. A client that decided by matching on
    /// "NO ANSWER" would silently stop distinguishing them the first time Roman
    /// edited that row — which he is invited to do.
    pub kind: String,
    /// The instance the gap is about.
    pub graph_node_id: String,
    /// The stored sentence with its `{code}` already filled in.
    pub message: String,
}

/// Every word the accusation section renders that is not already a sentence.
///
/// A wire mirror of `domain::wording_accusation::AccusationWording` MINUS the six
/// templates the server fills — see this module's header for why their absence is
/// the point.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccusationWordingDto {
    pub section_heading: String,
    pub section_hint: String,
    pub text_label: String,
    pub text_placeholder: String,
    pub text_missing_notice: String,
    pub text_edit_label: String,
    pub text_save_label: String,
    pub text_clear_label: String,
    pub text_cancel_label: String,
    pub mark_label: String,
    pub unmark_label: String,
    pub pair_label: String,
    pub repair_label: String,
    pub unpair_label: String,
    pub answer_label: String,
    pub picker_prompt: String,
    pub picker_cancel_label: String,
    /// Shown when the picker's filter leaves nothing.
    pub picker_no_match_notice: String,
    /// Shown when the picker had nothing to offer in the first place. A different
    /// state with a different remedy, so never the same sentence.
    pub picker_empty_notice: String,
    pub gaps_heading: String,
    pub no_gaps_notice: String,
    /// Carries `{detail}` — the one placeholder only the browser can fill.
    pub save_failed_template: String,
}

/// The accusation section in one read.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccusationPanelDto {
    /// The standing accusation in a human's plain words, or `null`.
    ///
    /// `null` is the honest-gap law: nobody has written it, so the section shows
    /// `wording.text_missing_notice` and never a quote from the record standing in
    /// for it. It is never derived from `attack_text`.
    pub accusation_text: Option<String>,
    /// "Said 3 times, in 2 documents." — `null` when nothing is marked, because a
    /// count of zero is not a count, it is the notice below.
    pub count_line: Option<String>,
    /// "No instances marked yet. 46 included facts are waiting here." — `null`
    /// once anything is marked. Exactly one of this and `count_line` is present,
    /// which is what stops the section rendering both or neither.
    pub no_instances_notice: Option<String>,
    /// The marked instances, in the derivation's total order.
    pub instances: Vec<AccusationInstanceDto>,
    /// Every named absence, prep list first.
    pub gaps: Vec<AccusationGapDto>,
    pub wording: AccusationWordingDto,
}

/// Body of the accusation-sentence write.
///
/// ## Why `Option<String>` and not two routes
///
/// `null` CLEARS the sentence, and clearing is a real act — a human withdrawing
/// words they no longer stand behind, which returns the rehearsal block to its
/// named gap. Modelling it as an absent field on the same route keeps one place
/// where the sentence is decided; a separate DELETE would be a second write path
/// to fence.
///
/// An empty or whitespace-only string is REFUSED rather than treated as a clear.
/// They are different intentions — one is "withdraw this", the other is a slip —
/// and the column's CHECK refuses a blank anyway, so accepting it here would only
/// change which layer said no.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetAccusationTextRequest {
    /// The sentence, or `null` to withdraw it.
    ///
    /// ## Rust Learning: `#[serde(default)]` on an `Option` field
    ///
    /// Without it, `{}` is a deserialization ERROR ("missing field text") while
    /// `{"text": null}` succeeds. With it, both mean `None`. Both spellings mean
    /// "no sentence" to a human, so both are accepted — and `deny_unknown_fields`
    /// still catches the typo (`{"txet": "…"}`) that would otherwise clear the
    /// accusation silently.
    #[serde(default)]
    pub text: Option<String>,
}

/// Body of the answer pairing.
///
/// One field, required: a pairing with no answer in it is not a pairing, and the
/// database says so too (`scenario_human_facts_answer_needs_anchor`). Unpairing
/// has its own DELETE rather than being a `null` here, because "this no longer
/// answers it" and "this answers it instead" are different acts and the log has to
/// tell them apart.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairAnswerRequest {
    /// The record item that answers the anchored instance.
    pub answers_graph_node_id: String,
}
