//! The accusations the link panel offers, and in what order (task 2.10).
//!
//! Pure — takes the graph's rows and the scenario's anchors, returns the panel
//! payload. No I/O, so the ordering rules (which are the whole point) are
//! testable with struct literals rather than a booted graph.
//!
//! ## The short list, and Roman's ruling of 2026-08-04
//!
//! The design said "the allegations this scenario already serves — three or four,
//! not thirty-eight". Measured on DEV there are two candidate sources and neither
//! is right alone: `scenarios.anchor_allegation_ids` holds ONE accusation for S-2,
//! and the accusations its pool already touches number 22. So the short list is
//! the UNION — the anchor first, then by how much of the pool touches each,
//! capped by the stored `link_short_list_max`. Everything else in the complaint
//! (120 accusations, measured) sits behind "Show all" with a filter box.
//!
//! The cap is a stored parameter and not a constant here for the ordinary reason:
//! it is a number that decides how a screen behaves, and Roman changes those
//! without a rebuild (v2 §2b).

use crate::domain::settings::Settings;
use crate::domain::wording::BackgroundDefaultState;
use crate::dto::evidence_links::{
    AllegationOptionDto, AllegationOptionsResponse, LinkPanelWording,
};
use crate::repositories::allegation_options_repository::AllegationOptionRow;

/// The accusation in complaint language, paragraph-numbered when known.
///
/// ## Why this duplicates `scenario_card::accusation_text` rather than calling it
///
/// It does not duplicate the RULE — it applies the same one to a different input.
/// That function takes an `ExtrasLink` (an Evidence→Allegation edge with its
/// bears-on chain); this takes a bare accusation with no edge, because the whole
/// point of the panel is offering accusations nothing links to yet. Widening the
/// other function's parameter to something both could satisfy would put a
/// half-empty struct on the card path to serve the picker.
///
/// The two are pinned to each other by a test, which is the honest way to hold a
/// shared rule that cannot be a shared function.
fn accusation_label(row: &AllegationOptionRow) -> String {
    let body = row
        .summary
        .as_deref()
        .or(row.title.as_deref())
        .unwrap_or(&row.allegation_id);

    match row.paragraph.as_deref() {
        Some(paragraph) if !paragraph.trim().is_empty() => format!("¶{paragraph} — {body}"),
        _ => body.to_string(),
    }
}

/// The count line for one accusation, or `None` when it sits under no count.
fn count_label(row: &AllegationOptionRow) -> Option<String> {
    match (row.count_number, row.count_name.as_deref()) {
        (Some(number), Some(name)) => Some(format!("Count {number} — {name}")),
        (Some(number), None) => Some(format!("Count {number}")),
        (None, Some(name)) => Some(name.to_string()),
        (None, None) => None,
    }
}

/// One row, ready to render.
fn to_option(row: &AllegationOptionRow) -> AllegationOptionDto {
    let label = accusation_label(row);
    let count = count_label(row);

    // The filter haystack: the label and the count together, lowercased once here
    // so the browser lowercases nothing and searches exactly what it is shown.
    let filter_text = match &count {
        Some(count) => format!("{label} {count}").to_lowercase(),
        None => label.to_lowercase(),
    };

    AllegationOptionDto {
        allegation_id: row.allegation_id.clone(),
        label,
        count_label: count,
        filter_text,
    }
}

/// Build the whole panel payload: its words, its short list, and everything else.
///
/// `anchor_ids` is the scenario's own `anchor_allegation_ids`. `rows` arrives in
/// paragraph order from the graph, which is the order `others` keeps.
///
/// ## Rust Learning: `sort_by_key` on a tuple, and why the key is negated
///
/// The short list sorts by "anchor first, then most-touched first". Rust's sort is
/// ASCENDING, so "most first" is expressed by sorting on the NEGATED count —
/// `-pool_items` — rather than by reversing the comparator, which would also
/// reverse the tie-breaks and put the highest paragraph number first among equals.
/// The sort is STABLE, so rows the key cannot separate stay in the graph's
/// paragraph order.
pub(crate) fn build_options(
    rows: Vec<AllegationOptionRow>,
    anchor_ids: &[String],
    settings: &Settings,
) -> AllegationOptionsResponse {
    let total = rows.len();

    // Which rows belong in the short list, and how they rank there. An anchor is
    // included whatever its pool count — it is the accusation this scenario was
    // created to answer, so it belongs at the top even on the day nothing has been
    // linked to it yet.
    let mut shortlisted: Vec<&AllegationOptionRow> = rows
        .iter()
        .filter(|row| anchor_ids.iter().any(|id| id == &row.allegation_id) || row.pool_items > 0)
        .collect();

    shortlisted.sort_by_key(|row| {
        let is_anchor = anchor_ids.iter().any(|id| id == &row.allegation_id);
        // `false < true` in Rust, so "not an anchor" sorts after "is an anchor"
        // when the key is `!is_anchor`.
        (!is_anchor, -row.pool_items)
    });
    shortlisted.truncate(settings.link_short_list_max);

    let serving: Vec<AllegationOptionDto> = shortlisted.iter().map(|row| to_option(row)).collect();

    // Everything the short list did not take, in the graph's paragraph order. An
    // accusation appears in exactly one of the two lists: a human scanning the
    // full list must not meet the same paragraph twice and wonder which is which.
    let others: Vec<AllegationOptionDto> = rows
        .iter()
        .filter(|row| !serving.iter().any(|o| o.allegation_id == row.allegation_id))
        .map(to_option)
        .collect();

    AllegationOptionsResponse {
        wording: panel_wording(settings, total),
        serving,
        others,
        total,
    }
}

/// The panel's words, with the one count that has to be filled in.
///
/// Every field is a clone of a stored string. There is not one literal here, and
/// the only computation is putting the real accusation count into "Show all
/// {count}" — which is why that row is a template rather than a sentence naming a
/// number that would be wrong the next time a document is processed.
fn panel_wording(settings: &Settings, total: usize) -> LinkPanelWording {
    let w = &settings.wording;
    LinkPanelWording {
        intro: w.link_panel_intro.clone(),
        scope_notice: w.link_scope_notice.clone(),
        allegations_heading: w.link_allegations_heading.clone(),
        show_all_label: crate::domain::wording_templates::render(
            &w.link_show_all_label,
            &[("count", total.to_string().as_str())],
        ),
        filter_placeholder: w.link_filter_placeholder.clone(),
        no_match_notice: w.link_no_match_notice.clone(),
        empty_options_notice: w.link_empty_options_notice.clone(),
        cut_heading: w.link_cut_heading.clone(),
        cut_supports_label: w.link_cut_supports_label.clone(),
        cut_against_label: w.link_cut_against_label.clone(),
        save_label: w.link_save_label.clone(),
        cancel_label: w.link_cancel_label.clone(),
        unlink_label: w.link_unlink_label.clone(),
        missing_cut_refusal: w.link_missing_cut_refusal.clone(),
        missing_allegation_refusal: w.link_missing_allegation_refusal.clone(),
        question_revert_label: w.question_revert_label.clone(),
        unlink_found_nothing: w.link_unlink_found_nothing.clone(),
        save_failed_template: w.link_save_failed_template.clone(),
        save_blocks_ruling: w.link_save_blocks_ruling.clone(),
        fact_remove_label: w.fact_remove_label.clone(),
        fact_remove_confirm_template: w.fact_remove_confirm_template.clone(),
        fact_remove_confirm_yes: w.fact_remove_confirm_yes.clone(),
        fact_remove_confirm_cancel: w.fact_remove_confirm_cancel.clone(),
        fact_remove_failed_template: w.fact_remove_failed_template.clone(),
        fact_question_label: w.fact_question_label.clone(),
        fact_statement_kind_label: w.fact_statement_kind_label.clone(),
        fact_tier_carries_label: w.fact_tier_carries_label.clone(),
        fact_tier_backup_label: w.fact_tier_backup_label.clone(),
        fact_tier_background_label: w.fact_tier_background_label.clone(),
        fact_tier_prompt: w.fact_tier_prompt.clone(),
        // Parsed HERE, once. `try_from` refuses an unrecognised token, and the
        // fallback is the seeded default rather than a guess — a store edited to
        // nonsense must not silently flip the pile open, and the refusal is
        // already named at boot by `build_wording`'s validation.
        fact_background_starts_collapsed: BackgroundDefaultState::try_from(
            w.fact_tier_background_default_state.as_str(),
        )
        .map(|state| state == BackgroundDefaultState::Collapsed)
        .unwrap_or_else(|e| {
            tracing::error!(
                error = %e,
                "the background-pile default state is not a token this build \
                 understands; falling back to collapsed, which is the seeded default"
            );
            true
        }),
        fact_background_count_template: w.fact_background_count_template.clone(),
        fact_background_hide_label: w.fact_background_hide_label.clone(),
        fact_order_drag_hint: w.fact_order_drag_hint.clone(),
        fact_tier_save_failed_template: w.fact_tier_save_failed_template.clone(),
        fact_order_save_failed_template: w.fact_order_save_failed_template.clone(),
        queue_raw_pool_toggle_template: w.queue_raw_pool_toggle_template.clone(),
        queue_empty_pool_summary: w.queue_empty_pool_summary.clone(),
        queue_all_ruled_summary: w.queue_all_ruled_summary.clone(),
        queue_counting_summary: w.queue_counting_summary.clone(),
        fact_answer_label: w.fact_answer_label.clone(),
        fact_background_move_notice: w.fact_background_move_notice.clone(),
        fact_weights_hint: w.fact_weights_hint.clone(),
        fact_footer_template: w.fact_footer_template.clone(),
        fact_unplace_label: w.fact_unplace_label.clone(),
        queue_proposed_heading_template: w.queue_proposed_heading_template.clone(),
        card_proposed_attribution_template: w.card_proposed_attribution_template.clone(),
        card_proposed_role_template: w.card_proposed_role_template.clone(),
        card_proposed_covers_template: w.card_proposed_covers_template.clone(),
        card_ruling_saved_template: w.card_ruling_saved_template.clone(),
        card_ruling_left_filter_template: w.card_ruling_left_filter_template.clone(),
        card_defer_recorded_template: w.card_defer_recorded_template.clone(),
        card_ruling_failed_template: w.card_ruling_failed_template.clone(),
        card_locked_condition_label: w.card_locked_condition_label.clone(),
    }
}

/// Every accusation's composed label, by id — what the CARD path needs.
///
/// The cards endpoint labels a human's links, and a link may point at an
/// accusation the extraction never touched, so the card path cannot get these
/// from its own extras query. Building the map here means both surfaces spell an
/// accusation the same way, from one rule.
pub(crate) fn label_index(
    rows: &[AllegationOptionRow],
) -> std::collections::HashMap<String, String> {
    rows.iter()
        .map(|row| (row.allegation_id.clone(), accusation_label(row)))
        .collect()
}

#[cfg(test)]
#[path = "scenario_link_options_tests.rs"]
mod tests;
