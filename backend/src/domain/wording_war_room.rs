// =============================================================================
// backend/src/domain/wording_war_room.rs — the words the WAR ROOM speaks
// =============================================================================
//
// The eleventh stored-string block (task 396, P3b). It carries the Trial Prep
// dashboard's own sentences — the subtitle under the heading and the three metric
// tiles — and nothing else.
//
// ## Why this block is arriving late, and what it is fixing
//
// Ruling R2 of 2026-08-10 (task R2 §3) renamed the dashboard's subtitle, renamed
// the "Drafted / in review" tile to "Draft", and killed the "pattern analysis
// pending" chip, on the stated grounds that "every one of these is a row value,
// so Roman can retune any of them later with zero builds". The R2 batch shipped
// nine `scenario_identity_*` rows and the scan model row; NONE of §3 was
// migrated. Measured on DEV 2026-08-13: no war-room row exists in `app_settings`
// at all, and both sentences are still compiled-in literals in
// `TrialPrepDashboardPage.tsx` and `trialPrepHelpers.ts`. So the rows were never
// created, not created-and-bypassed — and this block is them.
//
// ## Domain note: the subtitle's rename is a correction of a claim, not a style
//
// "System-generated cross-examination scenarios" said the machine produced them.
// It did not: a human writes the attack, the scan gathers candidates, a human
// rules every one. A subtitle that credits the system for a human's judgment is
// the same honesty defect as an unlabelled placeholder number, one line higher up
// the page.

/// The stored strings the Trial Prep dashboard renders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarRoomWording {
    /// The sentence under the "Trial Prep — War Room" heading, saying who built
    /// what.
    pub subtitle: String,
    /// The label on the total-scenarios tile.
    pub metric_scenarios_label: String,
    /// The label on the ready-scenarios tile.
    pub metric_ready_label: String,
    /// The label on the not-yet-ready tile.
    ///
    /// Domain note: R2 shortened this from "Drafted / in review" because the tile
    /// counts ONE thing — scenarios that are not Ready — and a slashed pair of
    /// words invites a reader to look for two numbers in one figure.
    pub metric_draft_label: String,

    // ── The status card (CC_TASK_WAR_ROOM_v1, 2026-09-16) ──────────────────────
    //
    // Domain note: the card reports where a scenario stands — evidence ruled →
    // linked into the Matrix → prep written → deck built → answered — instead of
    // the "Ready" pill all eleven cards used to wear.
    /// The status card's middle-pane header.
    pub card_evidence_heading: String,
    /// The status card's right-pane header.
    pub card_prep_heading: String,
    /// Label: facts a human ruled Included.
    pub card_facts_included_label: String,
    /// Label: cards the latest scan proposed that nobody has ruled.
    pub card_candidates_label: String,
    /// Label: unlinked statements a human has since linked.
    pub card_matrix_linked_label: String,
    /// Template with `{linked}` and `{total}`.
    pub card_matrix_linked_template: String,
    /// The Matrix value when nothing was left unlinked (ruling Q4: never "0 of 0").
    pub card_matrix_linked_none: String,
    /// Template with `{date}` only — the model and relevant counts live on the
    /// scenario page (CC_TASK_REVIEW_LOOP_v1 §5).
    pub card_scan_template: String,
    /// The scan line for a scenario no scan has touched.
    pub card_scan_never: String,
    /// The prep headline when there are no visible questions.
    pub card_deck_none: String,
    /// The bold half of the prep headline: `{answered}` and `{total}`.
    pub card_answered_count_template: String,
    /// The regular-weight word after it: "answered".
    pub card_answered_word: String,
    /// The one muted line under the headline: the side split, the deck size and
    /// the deck's date (`{chuck_answered}`, `{chuck_total}`, `{defense_answered}`,
    /// `{defense_total}`, `{count}`, `{date}`).
    pub card_prep_meta_template: String,
    /// The amber pill: `{count}` questions new or changed since Marie last answered.
    pub card_changed_template: String,
    /// The amber pill for the review queue on THIS scenario, the same for every
    /// viewer: `{count}` answers awaiting `{reviewer}`'s review
    /// (CC_TASK_SIMPLE_COUNTS_v1). `{reviewer}` is the display-name settings row.
    pub card_review_template: String,
    /// Its singular, read when the count is exactly 1.
    pub card_review_one: String,
    /// The singular of `card_changed_template`, read when the count is exactly 1.
    pub card_changed_one: String,
    /// The gray pill: nobody has answered anything on this deck.
    pub card_not_started: String,
    /// The green pill.
    pub card_up_to_date: String,
    /// Action: open the practice deck.
    pub card_practice_action: String,
    /// Action: open the timeline window.
    pub card_timeline_action: String,
    /// Action: ask to delete (opens the confirm dialog).
    pub card_delete_action: String,

    /// The summary card's words (CC_TASK_SIMPLE_COUNTS_v1) — a nested block, see
    /// `wording_war_room_summary`.
    pub summary: super::wording_war_room_summary::WarRoomSummaryWording,
}

// KEYS: the stable identifiers. Renaming one is a migration, and until it runs
// the boot loader refuses to start.
pub(crate) const KEY_SUBTITLE: &str = "war_room_subtitle";
pub(crate) const KEY_METRIC_SCENARIOS: &str = "war_room_metric_scenarios_label";
pub(crate) const KEY_METRIC_READY: &str = "war_room_metric_ready_label";
pub(crate) const KEY_METRIC_DRAFT: &str = "war_room_metric_draft_label";
pub(crate) const KEY_CARD_EVIDENCE_HEADING: &str = "war_room_card_evidence_heading";
pub(crate) const KEY_CARD_PREP_HEADING: &str = "war_room_card_prep_heading";
pub(crate) const KEY_CARD_FACTS_INCLUDED_LABEL: &str = "war_room_card_facts_included_label";
pub(crate) const KEY_CARD_CANDIDATES_LABEL: &str = "war_room_card_candidates_label";
pub(crate) const KEY_CARD_MATRIX_LINKED_LABEL: &str = "war_room_card_matrix_linked_label";
pub(crate) const KEY_CARD_MATRIX_LINKED_TEMPLATE: &str = "war_room_card_matrix_linked_template";
pub(crate) const KEY_CARD_MATRIX_LINKED_NONE: &str = "war_room_card_matrix_linked_none";
pub(crate) const KEY_CARD_SCAN_TEMPLATE: &str = "war_room_card_scan_template";
pub(crate) const KEY_CARD_SCAN_NEVER: &str = "war_room_card_scan_never";
pub(crate) const KEY_CARD_DECK_NONE: &str = "war_room_card_deck_none";
pub(crate) const KEY_CARD_ANSWERED_COUNT_TEMPLATE: &str = "war_room_card_answered_count_template";
pub(crate) const KEY_CARD_ANSWERED_WORD: &str = "war_room_card_answered_word";
pub(crate) const KEY_CARD_PREP_META_TEMPLATE: &str = "war_room_card_prep_meta_template";
pub(crate) const KEY_CARD_CHANGED_TEMPLATE: &str = "war_room_card_changed_template";
pub(crate) const KEY_CARD_REVIEW_TEMPLATE: &str = "war_room_card_review_template";
pub(crate) const KEY_CARD_REVIEW_ONE: &str = "war_room_card_review_one";
pub(crate) const KEY_CARD_CHANGED_ONE: &str = "war_room_card_changed_one";
pub(crate) const KEY_CARD_NOT_STARTED: &str = "war_room_card_not_started";
pub(crate) const KEY_CARD_UP_TO_DATE: &str = "war_room_card_up_to_date";
pub(crate) const KEY_CARD_PRACTICE_ACTION: &str = "war_room_card_practice_action";
pub(crate) const KEY_CARD_TIMELINE_ACTION: &str = "war_room_card_timeline_action";
pub(crate) const KEY_CARD_DELETE_ACTION: &str = "war_room_card_delete_action";

/// Every war-room key this build reads, so a missing one is caught at boot BY
/// NAME rather than as an unlabelled tile.
pub const WAR_ROOM_WORDING_KEYS: &[&str] = &[
    KEY_SUBTITLE,
    KEY_METRIC_SCENARIOS,
    KEY_METRIC_READY,
    KEY_METRIC_DRAFT,
    KEY_CARD_EVIDENCE_HEADING,
    KEY_CARD_PREP_HEADING,
    KEY_CARD_FACTS_INCLUDED_LABEL,
    KEY_CARD_CANDIDATES_LABEL,
    KEY_CARD_MATRIX_LINKED_LABEL,
    KEY_CARD_MATRIX_LINKED_TEMPLATE,
    KEY_CARD_MATRIX_LINKED_NONE,
    KEY_CARD_SCAN_TEMPLATE,
    KEY_CARD_SCAN_NEVER,
    KEY_CARD_DECK_NONE,
    KEY_CARD_ANSWERED_COUNT_TEMPLATE,
    KEY_CARD_ANSWERED_WORD,
    KEY_CARD_PREP_META_TEMPLATE,
    KEY_CARD_CHANGED_TEMPLATE,
    KEY_CARD_REVIEW_TEMPLATE,
    KEY_CARD_REVIEW_ONE,
    KEY_CARD_CHANGED_ONE,
    KEY_CARD_NOT_STARTED,
    KEY_CARD_UP_TO_DATE,
    KEY_CARD_PRACTICE_ACTION,
    KEY_CARD_TIMELINE_ACTION,
    KEY_CARD_DELETE_ACTION,
];

/// Build a [`WarRoomWording`] from the stored rows, or say which key is wrong.
///
/// Same generic-closure shape as the ten sibling builders — see
/// [`crate::domain::wording_model_params::build_model_params_wording`].
///
/// # Errors
/// Returns whatever `read` returns for the first key that is missing, of the
/// wrong declared kind, or blank.
pub fn build_war_room_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<WarRoomWording, E> {
    Ok(WarRoomWording {
        subtitle: read(KEY_SUBTITLE)?,
        metric_scenarios_label: read(KEY_METRIC_SCENARIOS)?,
        metric_ready_label: read(KEY_METRIC_READY)?,
        metric_draft_label: read(KEY_METRIC_DRAFT)?,
        card_evidence_heading: read(KEY_CARD_EVIDENCE_HEADING)?,
        card_prep_heading: read(KEY_CARD_PREP_HEADING)?,
        card_facts_included_label: read(KEY_CARD_FACTS_INCLUDED_LABEL)?,
        card_candidates_label: read(KEY_CARD_CANDIDATES_LABEL)?,
        card_matrix_linked_label: read(KEY_CARD_MATRIX_LINKED_LABEL)?,
        card_matrix_linked_template: read(KEY_CARD_MATRIX_LINKED_TEMPLATE)?,
        card_matrix_linked_none: read(KEY_CARD_MATRIX_LINKED_NONE)?,
        card_scan_template: read(KEY_CARD_SCAN_TEMPLATE)?,
        card_scan_never: read(KEY_CARD_SCAN_NEVER)?,
        card_deck_none: read(KEY_CARD_DECK_NONE)?,
        card_answered_count_template: read(KEY_CARD_ANSWERED_COUNT_TEMPLATE)?,
        card_answered_word: read(KEY_CARD_ANSWERED_WORD)?,
        card_prep_meta_template: read(KEY_CARD_PREP_META_TEMPLATE)?,
        card_changed_template: read(KEY_CARD_CHANGED_TEMPLATE)?,
        card_review_template: read(KEY_CARD_REVIEW_TEMPLATE)?,
        card_review_one: read(KEY_CARD_REVIEW_ONE)?,
        card_changed_one: read(KEY_CARD_CHANGED_ONE)?,
        card_not_started: read(KEY_CARD_NOT_STARTED)?,
        card_up_to_date: read(KEY_CARD_UP_TO_DATE)?,
        card_practice_action: read(KEY_CARD_PRACTICE_ACTION)?,
        card_timeline_action: read(KEY_CARD_TIMELINE_ACTION)?,
        card_delete_action: read(KEY_CARD_DELETE_ACTION)?,
        summary: super::wording_war_room_summary::build_war_room_summary_wording(&read)?,
    })
}

#[cfg(test)]
#[path = "wording_war_room_tests.rs"]
pub(crate) mod seed_tests;

#[cfg(test)]
mod tests {
    use super::*;

    fn echo(key: &str) -> Result<String, std::convert::Infallible> {
        Ok(key.to_string())
    }

    /// Every field reads the key it claims to — the three tile labels especially,
    /// because they are three short strings of the same shape and a copy-paste
    /// between them would put one tile's word on another with nothing failing.
    #[test]
    fn every_field_reads_its_own_key() {
        let w = build_war_room_wording(echo).expect("infallible read");
        assert_eq!(w.subtitle, KEY_SUBTITLE);
        assert_eq!(w.metric_scenarios_label, KEY_METRIC_SCENARIOS);
        assert_eq!(w.metric_ready_label, KEY_METRIC_READY);
        assert_eq!(w.metric_draft_label, KEY_METRIC_DRAFT);
        assert_eq!(w.card_evidence_heading, KEY_CARD_EVIDENCE_HEADING);
        assert_eq!(w.card_prep_heading, KEY_CARD_PREP_HEADING);
        assert_eq!(w.card_facts_included_label, KEY_CARD_FACTS_INCLUDED_LABEL);
        assert_eq!(w.card_candidates_label, KEY_CARD_CANDIDATES_LABEL);
        assert_eq!(w.card_matrix_linked_label, KEY_CARD_MATRIX_LINKED_LABEL);
        assert_eq!(
            w.card_matrix_linked_template,
            KEY_CARD_MATRIX_LINKED_TEMPLATE
        );
        assert_eq!(w.card_matrix_linked_none, KEY_CARD_MATRIX_LINKED_NONE);
        assert_eq!(w.card_scan_template, KEY_CARD_SCAN_TEMPLATE);
        assert_eq!(w.card_scan_never, KEY_CARD_SCAN_NEVER);
        assert_eq!(w.card_deck_none, KEY_CARD_DECK_NONE);
        assert_eq!(
            w.card_answered_count_template,
            KEY_CARD_ANSWERED_COUNT_TEMPLATE
        );
        assert_eq!(w.card_answered_word, KEY_CARD_ANSWERED_WORD);
        assert_eq!(w.card_prep_meta_template, KEY_CARD_PREP_META_TEMPLATE);
        assert_eq!(w.card_changed_template, KEY_CARD_CHANGED_TEMPLATE);
        assert_eq!(w.card_review_template, KEY_CARD_REVIEW_TEMPLATE);
        assert_eq!(w.card_review_one, KEY_CARD_REVIEW_ONE);
        assert_eq!(w.card_changed_one, KEY_CARD_CHANGED_ONE);
        assert_eq!(w.card_not_started, KEY_CARD_NOT_STARTED);
        assert_eq!(w.card_up_to_date, KEY_CARD_UP_TO_DATE);
        assert_eq!(w.card_practice_action, KEY_CARD_PRACTICE_ACTION);
        assert_eq!(w.card_timeline_action, KEY_CARD_TIMELINE_ACTION);
        assert_eq!(w.card_delete_action, KEY_CARD_DELETE_ACTION);
    }

    /// A key the builder reads but the list omits would be missing from
    /// `REQUIRED_KEYS`, so a blank tile would reach the screen instead of a named
    /// boot refusal.
    #[test]
    fn the_key_list_covers_every_field() {
        let w = build_war_room_wording(echo).expect("infallible read");
        let read_keys = [
            w.subtitle,
            w.metric_scenarios_label,
            w.metric_ready_label,
            w.metric_draft_label,
            w.card_evidence_heading,
            w.card_prep_heading,
            w.card_facts_included_label,
            w.card_candidates_label,
            w.card_matrix_linked_label,
            w.card_matrix_linked_template,
            w.card_matrix_linked_none,
            w.card_scan_template,
            w.card_scan_never,
            w.card_deck_none,
            w.card_answered_count_template,
            w.card_answered_word,
            w.card_prep_meta_template,
            w.card_changed_template,
            w.card_review_template,
            w.card_review_one,
            w.card_changed_one,
            w.card_not_started,
            w.card_up_to_date,
            w.card_practice_action,
            w.card_timeline_action,
            w.card_delete_action,
        ];
        assert_eq!(read_keys.len(), WAR_ROOM_WORDING_KEYS.len());
        for key in read_keys {
            assert!(
                WAR_ROOM_WORDING_KEYS.contains(&key.as_str()),
                "{key} is read by the builder but missing from WAR_ROOM_WORDING_KEYS",
            );
        }
    }

    #[test]
    fn the_keys_are_distinct() {
        let mut sorted = WAR_ROOM_WORDING_KEYS.to_vec();
        sorted.sort_unstable();
        let count = sorted.len();
        sorted.dedup();
        assert_eq!(sorted.len(), count, "two war-room wording keys collide");
    }
}
