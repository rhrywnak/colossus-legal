//! Every string the case-timeline surfaces speak (CASE_CHRONOLOGY_DESIGN_v2, Phase B).
//!
//! A block of its own, for the reason each sibling has one: these are the words
//! ONE surface family speaks — the timeline list, the event page, and the home
//! band — and they move independently of every other screen's language.
//!
//! ## ⚑ THE THREE PARTIES, AND WHY BOTH HALVES ARE HERE
//!
//! A key is only real when all three agree: the DATABASE holds a row, this
//! module DECLARES the key, and the FRONTEND asks for it. Boot refuses if a
//! declared key has no row; `dto::chronology_wording_reach_tests` refuses if the
//! frontend asks for a name no field carries. That third edge is the .407 lesson
//! — seven rows seeded, declared in no block, and a page that rendered blank
//! against a database that was correct the whole time.
//!
//! ## ⚑ TWO STRINGS THIS BLOCK DELIBERATELY DOES NOT CARRY
//!
//! A loading line and a load-failure line. Both were drafted, and both were
//! withdrawn on the same reasoning: the wording store is DELIVERED BY the
//! request whose failure they would describe. A key that can only be read after
//! the read succeeded cannot speak about the read failing, and a key nothing can
//! reach is a row seeded, mirrored and paid for that no screen ever says.
//!
//! The bootstrap text lives in ONE named place in the frontend service instead,
//! marked as the exception it is. See the report's NEEDS A RULING.
//!
//! ## Why the glyphs are IN the stored strings
//!
//! `⚠`, `💬`, `⤢`, `⇲` and `◌` are part of the words, not decoration a component
//! adds. Putting them in the row means Roman can drop one from a label without a
//! rebuild, and it keeps the component free of any user-visible character at all
//! — which is the whole point of the no-wording-in-code law.

/// Every string the chronology surfaces speak.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChronologyWording {
    /// The page's own title.
    pub page_title: String,
    /// The unfiltered subtitle. `{events}` and `{phases}`.
    pub count_template: String,
    /// The FILTERED subtitle. `{phase}`, `{shown}`, `{total}`.
    ///
    /// Design R16: the subtitle is how a filtered state stays visible. A page
    /// that quietly showed six of twenty-two events with nothing saying so is
    /// the failure this template exists to prevent.
    pub filtered_count_template: String,

    /// The search box's placeholder.
    pub search_placeholder: String,
    /// The chip that clears the tag filter.
    pub all_tags_label: String,
    /// The date-range control's label.
    pub dates_label: String,
    /// The earliest-date field's label.
    pub date_from_label: String,
    /// The latest-date field's label.
    pub date_to_label: String,

    /// The always-visible control on every phase header (R16, R17).
    pub expand_label: String,
    /// How a reader leaves an expanded phase.
    pub show_all_phases_label: String,
    /// The line above a scroll window. `{count}`.
    pub scroll_hint_template: String,
    /// The phase header's right-hand meta line. `{range}` and `{count}`.
    pub phase_count_template: String,

    /// The amber mark on an event whose document is not in the system (R12).
    pub no_document_label: String,
    /// A link this build has no resolver for.
    ///
    /// ## Domain note: this is NOT "no document yet"
    ///
    /// `missing` means looked for and not there. `unchecked` means nobody
    /// looked, because this build cannot see that target's store. Rendering the
    /// second as the first would tell a reader a document is absent when the
    /// truth is that nothing checked — which is the claim the three-state
    /// resolution was introduced to stop making.
    pub link_unchecked_label: String,
    /// The note badge, plural. `{count}`.
    pub note_count_template: String,
    /// The note badge when there is exactly one.
    pub note_count_one: String,
    /// Shown beside a link that carries no pinpoint (R9) — the absence is the
    /// to-scan to-do list, so it is marked rather than left blank.
    pub no_pinpoint_label: String,

    /// The case genuinely holds no events.
    pub empty_label: String,
    /// The case holds events but the filters match none of them.
    ///
    /// A DIFFERENT string from `empty_label` on purpose: "there is nothing here"
    /// and "your filters hid everything" send a reader to two different places.
    pub no_matches_label: String,
    /// An event whose phase slug has no phase row. `{id}` and `{phase}`.
    ///
    /// ## Domain note: it renders, loudly, instead of vanishing
    ///
    /// The home band used to count such an event nowhere and show nothing —
    /// silently. An event nobody can see is an event nobody can fix.
    pub unknown_phase_template: String,

    /// The event page's breadcrumb back to the list.
    pub back_label: String,
    /// The event page's documents panel.
    pub documents_heading: String,
    /// The event page's notes panel.
    pub notes_heading: String,
    /// The event page's history panel.
    pub history_heading: String,
    /// Shown in the history panel when there is nothing in it.
    ///
    /// Rendered HONESTLY rather than hiding the panel: history is empty for
    /// every event until Phase C writes the first row, and a missing panel would
    /// read as a feature that does not exist rather than one with nothing in it.
    pub no_history_label: String,
    /// Shown in the notes panel when there is nothing in it.
    pub no_notes_label: String,

    /// The home band's count-mismatch marker. `{shown}` and `{total}`.
    ///
    /// Design B6: the band groups events by phase, so an event whose phase has
    /// no pill was previously counted nowhere and dropped without a word. This
    /// line is what it says instead.
    pub band_mismatch_template: String,
}

pub(crate) const KEY_PAGE_TITLE: &str = "chronology_page_title";
pub(crate) const KEY_COUNT_TEMPLATE: &str = "chronology_count_template";
pub(crate) const KEY_FILTERED_COUNT_TEMPLATE: &str = "chronology_filtered_count_template";
pub(crate) const KEY_SEARCH_PLACEHOLDER: &str = "chronology_search_placeholder";
pub(crate) const KEY_ALL_TAGS_LABEL: &str = "chronology_all_tags_label";
pub(crate) const KEY_DATES_LABEL: &str = "chronology_dates_label";
pub(crate) const KEY_DATE_FROM_LABEL: &str = "chronology_date_from_label";
pub(crate) const KEY_DATE_TO_LABEL: &str = "chronology_date_to_label";
pub(crate) const KEY_EXPAND_LABEL: &str = "chronology_expand_label";
pub(crate) const KEY_SHOW_ALL_PHASES_LABEL: &str = "chronology_show_all_phases_label";
pub(crate) const KEY_SCROLL_HINT_TEMPLATE: &str = "chronology_scroll_hint_template";
pub(crate) const KEY_PHASE_COUNT_TEMPLATE: &str = "chronology_phase_count_template";
pub(crate) const KEY_NO_DOCUMENT_LABEL: &str = "chronology_no_document_label";
pub(crate) const KEY_LINK_UNCHECKED_LABEL: &str = "chronology_link_unchecked_label";
pub(crate) const KEY_NOTE_COUNT_TEMPLATE: &str = "chronology_note_count_template";
pub(crate) const KEY_NOTE_COUNT_ONE: &str = "chronology_note_count_one";
pub(crate) const KEY_NO_PINPOINT_LABEL: &str = "chronology_no_pinpoint_label";
pub(crate) const KEY_EMPTY_LABEL: &str = "chronology_empty_label";
pub(crate) const KEY_NO_MATCHES_LABEL: &str = "chronology_no_matches_label";
pub(crate) const KEY_UNKNOWN_PHASE_TEMPLATE: &str = "chronology_unknown_phase_template";
pub(crate) const KEY_BACK_LABEL: &str = "chronology_back_label";
pub(crate) const KEY_DOCUMENTS_HEADING: &str = "chronology_documents_heading";
pub(crate) const KEY_NOTES_HEADING: &str = "chronology_notes_heading";
pub(crate) const KEY_HISTORY_HEADING: &str = "chronology_history_heading";
pub(crate) const KEY_NO_HISTORY_LABEL: &str = "chronology_no_history_label";
pub(crate) const KEY_NO_NOTES_LABEL: &str = "chronology_no_notes_label";
pub(crate) const KEY_BAND_MISMATCH_TEMPLATE: &str = "chronology_band_mismatch_template";

/// Declared to the boot loader. A key here with no row in any migration makes
/// the backend REFUSE TO START — which is what the sibling test file exists to
/// catch before a deploy does.
pub const CHRONOLOGY_WORDING_KEYS: &[&str] = &[
    KEY_PAGE_TITLE,
    KEY_COUNT_TEMPLATE,
    KEY_FILTERED_COUNT_TEMPLATE,
    KEY_SEARCH_PLACEHOLDER,
    KEY_ALL_TAGS_LABEL,
    KEY_DATES_LABEL,
    KEY_DATE_FROM_LABEL,
    KEY_DATE_TO_LABEL,
    KEY_EXPAND_LABEL,
    KEY_SHOW_ALL_PHASES_LABEL,
    KEY_SCROLL_HINT_TEMPLATE,
    KEY_PHASE_COUNT_TEMPLATE,
    KEY_NO_DOCUMENT_LABEL,
    KEY_LINK_UNCHECKED_LABEL,
    KEY_NOTE_COUNT_TEMPLATE,
    KEY_NOTE_COUNT_ONE,
    KEY_NO_PINPOINT_LABEL,
    KEY_EMPTY_LABEL,
    KEY_NO_MATCHES_LABEL,
    KEY_UNKNOWN_PHASE_TEMPLATE,
    KEY_BACK_LABEL,
    KEY_DOCUMENTS_HEADING,
    KEY_NOTES_HEADING,
    KEY_HISTORY_HEADING,
    KEY_NO_HISTORY_LABEL,
    KEY_NO_NOTES_LABEL,
    KEY_BAND_MISMATCH_TEMPLATE,
];

/// Build a [`ChronologyWording`] from the stored rows, or say which key is wrong.
///
/// ## Rust Learning: one closure, every block, the same rule
///
/// `read` is taken by value as an `impl Fn`, matching the sibling blocks. The
/// caller in `settings_store` owns it; every block is judged by the identical
/// rule, so a key that is missing, blank or of the wrong declared kind fails the
/// same way here as anywhere else.
///
/// # Errors
/// Whatever `read` returns for the first key that is missing, blank, or of the
/// wrong declared kind.
pub fn build_chronology_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<ChronologyWording, E> {
    Ok(ChronologyWording {
        page_title: read(KEY_PAGE_TITLE)?,
        count_template: read(KEY_COUNT_TEMPLATE)?,
        filtered_count_template: read(KEY_FILTERED_COUNT_TEMPLATE)?,
        search_placeholder: read(KEY_SEARCH_PLACEHOLDER)?,
        all_tags_label: read(KEY_ALL_TAGS_LABEL)?,
        dates_label: read(KEY_DATES_LABEL)?,
        date_from_label: read(KEY_DATE_FROM_LABEL)?,
        date_to_label: read(KEY_DATE_TO_LABEL)?,
        expand_label: read(KEY_EXPAND_LABEL)?,
        show_all_phases_label: read(KEY_SHOW_ALL_PHASES_LABEL)?,
        scroll_hint_template: read(KEY_SCROLL_HINT_TEMPLATE)?,
        phase_count_template: read(KEY_PHASE_COUNT_TEMPLATE)?,
        no_document_label: read(KEY_NO_DOCUMENT_LABEL)?,
        link_unchecked_label: read(KEY_LINK_UNCHECKED_LABEL)?,
        note_count_template: read(KEY_NOTE_COUNT_TEMPLATE)?,
        note_count_one: read(KEY_NOTE_COUNT_ONE)?,
        no_pinpoint_label: read(KEY_NO_PINPOINT_LABEL)?,
        empty_label: read(KEY_EMPTY_LABEL)?,
        no_matches_label: read(KEY_NO_MATCHES_LABEL)?,
        unknown_phase_template: read(KEY_UNKNOWN_PHASE_TEMPLATE)?,
        back_label: read(KEY_BACK_LABEL)?,
        documents_heading: read(KEY_DOCUMENTS_HEADING)?,
        notes_heading: read(KEY_NOTES_HEADING)?,
        history_heading: read(KEY_HISTORY_HEADING)?,
        no_history_label: read(KEY_NO_HISTORY_LABEL)?,
        no_notes_label: read(KEY_NO_NOTES_LABEL)?,
        band_mismatch_template: read(KEY_BAND_MISMATCH_TEMPLATE)?,
    })
}

#[cfg(test)]
#[path = "wording_chronology_tests.rs"]
mod tests;
