//! Wire DTOs for linking a statement to an accusation (task 2.10).
//!
//! Three shapes: the panel's own payload (its words and the accusations it
//! offers), the request that saves a link, and what a save or an unlink reports
//! back.
//!
//! ## Everything user-visible arrives composed
//!
//! Every string below was built on this side of the wire — the accusation's
//! label, the count line, the panel's twenty words, the "Show all 120" with its
//! number already in it. The browser renders them and concatenates nothing (v2 §7
//! item 2, the language law).
//!
//! ## The panel's words come from the STORE, not from this file
//!
//! [`LinkPanelWording`] is a wire mirror of `domain::wording::Wording`, which is
//! read from `app_settings` at boot. There is no default anywhere in this module:
//! a literal here would be the compiled-in string Roman's 2026-08-04 ruling
//! deletes, wearing a DTO costume.

use serde::{Deserialize, Serialize};

use crate::domain::link_cut::LinkCut;

/// One accusation a human may tick.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllegationOptionDto {
    /// The graph id, which is what a save sends back.
    pub allegation_id: String,
    /// The accusation in complaint language, paragraph-numbered when known —
    /// "¶41 — Defendants attempted to justify the auction…". Composed by the same
    /// rule the card's own bears-on line uses, so the two never read differently.
    pub label: String,
    /// "Count 2 — Breach of fiduciary duty", or `null` when the accusation sits
    /// under no count. Absent rather than empty: a chip with no words is a chip
    /// that should not be there.
    pub count_label: Option<String>,
    /// The lowercased haystack the filter box matches against.
    ///
    /// ## Why the backend decides what is searchable
    ///
    /// Otherwise the browser picks — label only? label and count? — and that
    /// choice is a statement about what a human is looking for when they type
    /// "fiduciary". Shipping the haystack keeps the browser doing a substring test
    /// and nothing else, which is presentation rather than logic.
    pub filter_text: String,
}

/// Every word the link panel puts on screen, read from the settings store.
///
/// A flat mirror of `domain::wording::Wording` minus the fields other surfaces
/// use, plus the two that arrive already rendered. Serialized once per scenario
/// load rather than on every card, because the cards payload is re-read after
/// every ruling and twenty strings times 148 cards times every keystroke is a lot
/// of bytes to say the same thing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkPanelWording {
    pub intro: String,
    pub scope_notice: String,
    pub allegations_heading: String,
    /// "Show all 120" — the count already filled in from what the graph holds.
    pub show_all_label: String,
    pub filter_placeholder: String,
    pub no_match_notice: String,
    pub empty_options_notice: String,
    pub cut_heading: String,
    pub cut_supports_label: String,
    pub cut_against_label: String,
    pub save_label: String,
    pub cancel_label: String,
    pub unlink_label: String,
    pub missing_cut_refusal: String,
    pub missing_allegation_refusal: String,
    /// The 1.7F fold-in: the control that restores a machine-written question.
    pub question_revert_label: String,
    /// Said when Unlink removed nothing because the pair was not linked. The
    /// client cannot compose this: "I removed your link" and "there was nothing
    /// there" must not be the same observable (Standing Rule 1).
    pub unlink_found_nothing: String,
    /// Said when a link or unlink could not be written. Carries `{detail}`, which
    /// the browser fills with the failure's own text — the one value that exists
    /// only on that side of the wire.
    pub save_failed_template: String,
    /// Why Include and Exclude are greyed while the panel holds unsaved choices
    /// (task 2.12, item B). A correct refusal with no explanation reads as a
    /// broken button — which is exactly how it was reported.
    pub save_blocks_ruling: String,
    /// The control that takes a fact back out of a scenario (item G).
    pub fact_remove_label: String,
    /// The question asked before a removal. Carries `{code}`.
    pub fact_remove_confirm_template: String,
    /// The button that confirms a removal.
    pub fact_remove_confirm_yes: String,
    /// The button that abandons it.
    pub fact_remove_confirm_cancel: String,
    /// Said when a removal could not be written. Carries `{detail}`.
    pub fact_remove_failed_template: String,

    // ── Task 2.13 slice 1 ────────────────────────────────────────────────────
    /// Marks the interrogatory question above a discovery answer.
    pub fact_question_label: String,
    /// Introduces the kind of statement a fact is.
    pub fact_statement_kind_label: String,
    /// The heaviest weight's name.
    pub fact_tier_carries_label: String,
    /// The middle weight's name — where a newly included fact lands.
    pub fact_tier_backup_label: String,
    /// The lightest weight's name.
    pub fact_tier_background_label: String,
    /// The prompt on a row's weight control.
    pub fact_tier_prompt: String,
    /// Whether the background pile starts folded.
    ///
    /// ## Why this crosses the wire as a BOOLEAN and not as the stored token
    ///
    /// The store holds `collapsed` / `expanded` because `ValueKind` has no
    /// boolean. Parsing that token is a decision with a wrong answer available —
    /// `value == "collapsed"` silently treats every typo as `expanded` — so it
    /// happens ONCE, on the server, where a bad token is a named refusal. The
    /// browser receives the decided answer and has nothing left to get wrong.
    /// Same reason `status` travels beside `status_label`: the client is told the
    /// state, never asked to infer it from prose.
    pub fact_background_starts_collapsed: bool,
    /// The folded pile's line. `{count}` is left IN the template — only the
    /// client knows how many facts are in the tier after filtering, so it is the
    /// one placeholder the server cannot fill.
    pub fact_background_count_template: String,
    /// The control that folds the pile away again.
    pub fact_background_hide_label: String,
    /// The drag handle's accessible label.
    pub fact_order_drag_hint: String,
    /// Said when a weight could not be stored. Carries `{code}` and `{reason}`.
    pub fact_tier_save_failed_template: String,
    /// Said when a drag could not be stored. Carries `{code}` and `{reason}`.
    pub fact_order_save_failed_template: String,
    /// The queue's summary when there is no pool at all — including before it has
    /// loaded, which is the state that used to claim the work was finished.
    /// The opt-in that opens the raw evidence pool on a never-scanned scenario.
    /// Carries `{count}` (task 2.15, piece 3a).
    pub queue_raw_pool_toggle_template: String,
    pub queue_empty_pool_summary: String,
    /// The queue's summary when a real pool exists and none of it is outstanding.
    pub queue_all_ruled_summary: String,
    /// The queue's summary while the counts are not known yet.
    pub queue_counting_summary: String,
    /// Marks the answer beneath its question, the question label's partner.
    pub fact_answer_label: String,
    /// Said when a fact moves into the background pile. Carries `{code}`.
    pub fact_background_move_notice: String,
    /// The line under the section heading naming the weights and the drag.
    pub fact_weights_hint: String,
    /// The count beneath the list. Carries `{shown}` and `{background}`.
    pub fact_footer_template: String,
    /// The control that forgets where a human dragged one fact.
    pub fact_unplace_label: String,
    /// The queue's heading when a completed scan is proposing candidates. Carries
    /// `{count}` and `{when}` (2026-08-08).
    pub queue_proposed_heading_template: String,
    /// The attribution line on a proposed card. Carries `{when}`.
    pub card_proposed_attribution_template: String,
    /// The proposed-role chip. Carries `{verb}`.
    pub card_proposed_role_template: String,
    /// The badge on a card whose one ruling settles a twin. Carries `{count}` and
    /// `{codes}`.
    pub card_proposed_covers_template: String,
}

/// The whole panel in one read.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllegationOptionsResponse {
    pub wording: LinkPanelWording,
    /// The accusations this scenario already serves — its anchor first, then the
    /// ones its pool touches most, capped by `link_short_list_max`.
    ///
    /// The short list exists because limiting the options makes review faster AND
    /// more accurate; the full list is one click behind it.
    pub serving: Vec<AllegationOptionDto>,
    /// Everything else in the complaint, in paragraph order.
    pub others: Vec<AllegationOptionDto>,
    /// How many accusations the case holds in total — `serving + others`.
    ///
    /// Sent as a number as well as being baked into `show_all_label` because the
    /// label is prose a human may edit and the count is a fact a client may need
    /// (to decide whether a "show all" control is worth rendering at all).
    pub total: usize,
}

/// What a human ticked, and which way they said it cuts.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveLinksRequest {
    /// One or more accusations. An empty list is refused with the stored sentence
    /// — saving nothing is not a link, and silently accepting it would leave the
    /// card exactly as stuck as it was with no explanation.
    pub allegation_ids: Vec<String>,
    /// Required. A link with no cut would feed a readiness verdict that cannot
    /// tell ammunition from hazard.
    ///
    /// ## Rust Learning: a typed enum on the REQUEST, not a `String`
    ///
    /// `serde` refuses a token outside the vocabulary before the handler runs, so
    /// there is no "what if it says 'maybe'" branch to write — the 400 names the
    /// field and the request never reaches the database.
    pub cut: LinkCut,
}

/// What a save did.
///
/// Deliberately carries no composed sentence: the card's summary is built from
/// the whole set of links plus the stored template, and the client re-reads the
/// pool to get it (which is also what unlocks Include and Exclude). Returning a
/// second, separately-composed copy here would be two places that can disagree.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveLinksResponse {
    pub graph_node_id: String,
    /// How many accusations were newly linked.
    pub linked: usize,
    /// How many were already linked and now cut the other way. The two are
    /// reported apart because they are different acts, and the ledger records
    /// them as different acts.
    pub recut: usize,
}

/// What an unlink did.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnlinkResponse {
    pub graph_node_id: String,
    pub allegation_id: String,
    /// Whether a link was actually removed. `false` means the client's view was
    /// stale — the pair was not linked. Not an error, but not the same thing as
    /// having removed something, and the client is told which happened.
    pub removed: bool,
}
