// =============================================================================
// backend/src/domain/case_phase.rs — which phase of the case a document belongs
// to (task DOCUMENT_PHASE)
// =============================================================================
//
// Roman: "the documents processed list does not include the phase." Four phases,
// and until now nothing in the schema recorded which one a document belonged to.
//
// ## The vocabulary is borrowed, not invented
//
// These four slugs already existed as the timeline's own data
// (`frontend/public/data/timeline.json`), and the Home page's timeline band has
// been rendering pills from them since long before this column. Defining a
// second vocabulary here — even one differing only in spelling — would mean a
// document tagged one way and a timeline phase named another never meeting.
//
// ## THIS MODULE HAS NO LABELS, DELIBERATELY
//
// Ruled 2026-08-17: the display names (PRE-PROBATE · PROBATE · COA · COMPLAINT)
// live in `timeline.json` and are read from there by every surface that renders
// one. The backend stores the slug, returns the slug, and never renders a label.
//
// That is not an oversight to be helpfully corrected later. If a `label()` method
// appeared here it would immediately be the SECOND place labels live, the two
// would drift the first time a phase was renamed, and the rename would then be a
// deploy instead of a data edit. The absence is the design. Renaming a phase for
// display is one line in one JSON file, and this module does not care.
//
// ## Why a code-owned lookup, not a Postgres enum
//
// The `actor_role` (D1) and `date_precision` (P4) precedent: a Rust enum plus a
// versioned list. A Postgres enum would make adding a phase a migration; bare
// strings compared in match arms would let a typo'd `"appeal"` fail silently.
// The migration carries a CHECK listing the same four tokens as a backstop.

use serde::{Deserialize, Serialize};

/// The version of the case-phase vocabulary THIS build defines.
///
/// Bumped whenever a phase is added or removed. Mirrors
/// `DATE_PRECISION_LOOKUP_V` and `ACTOR_ROLE_LOOKUP_V`.
///
/// Note what does NOT bump it: renaming a phase's display label. That is a
/// `timeline.json` edit and this vocabulary is unaffected — which is the whole
/// point of storing slugs.
pub const CASE_PHASE_LOOKUP_V: u32 = 1;

/// Which phase of the case a document belongs to.
///
/// ## Rust Learning: `#[serde(rename_all = "snake_case")]` on an enum
///
/// serde would otherwise render `CivilLawsuit` as `"CivilLawsuit"`. `rename_all`
/// maps every variant to its wire token in one line, so the JSON the frontend
/// sends, the text in the Postgres column and the slug in `timeline.json` stay
/// identical without per-variant attributes. An unknown token fails to
/// deserialize rather than defaulting — the loud boundary Standing Rule 1 asks
/// for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CasePhase {
    /// Before the probate case: the conversion, the guardianship petition, and
    /// Emil Awad's death. Displayed as PRE-PROBATE.
    Estate,
    /// The probate court proceedings. Displayed as PROBATE.
    Probate,
    /// The Court of Appeals. Displayed as COA.
    Appeals,
    /// The civil action this system exists to prepare. Displayed as COMPLAINT.
    CivilLawsuit,
}

/// Every phase this build defines, in the case's chronological order.
///
/// The order is the order a UI offers them and the order the timeline renders
/// them, which is the same order because it is the order the case happened in.
pub const ALL_CASE_PHASES: &[CasePhase] = &[
    CasePhase::Estate,
    CasePhase::Probate,
    CasePhase::Appeals,
    CasePhase::CivilLawsuit,
];

impl CasePhase {
    /// The stable wire token — the slug stored, transmitted, and matched against
    /// `timeline.json`'s phase `id`.
    ///
    /// Not a label. See the module header for why there is no `label()`.
    pub fn slug(self) -> &'static str {
        match self {
            CasePhase::Estate => "estate",
            CasePhase::Probate => "probate",
            CasePhase::Appeals => "appeals",
            CasePhase::CivilLawsuit => "civil_lawsuit",
        }
    }

    /// Parse a slug from the wire.
    ///
    /// ## Rust Learning: returning `Option` rather than defaulting
    ///
    /// A `_ => CasePhase::Estate` arm would make a typo'd phase silently become
    /// the first one, and a document would sit in the wrong phase with nothing
    /// to show for it. `None` forces the caller to decide, and the one caller
    /// turns it into a 400 naming the four valid slugs.
    pub fn from_slug(slug: &str) -> Option<Self> {
        ALL_CASE_PHASES.iter().copied().find(|p| p.slug() == slug)
    }
}

/// Why a phase could not be accepted.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CasePhaseError {
    #[error("'{supplied}' is not a phase of this case — expected one of: {valid}")]
    Unknown { supplied: String, valid: String },
}

/// Validate an optional phase from a request body.
///
/// `None` is VALID and means "no phase recorded" — the field is never required
/// (chronology design R4: absence tolerated). That is the difference from
/// `date_precision::validate`, which enforces mandatory-with-override: a
/// document with no date is a question nobody answered, whereas a document with
/// no phase is simply one nobody has filed yet, and there is no useful third
/// state between them.
///
/// An empty or whitespace-only string is treated as `None` rather than rejected:
/// a `<select>` with no selection posts `""`, and turning that into a 400 would
/// make "clear this field" impossible from the only UI that writes it.
pub fn validate(supplied: Option<&str>) -> Result<Option<CasePhase>, CasePhaseError> {
    let Some(raw) = supplied.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };

    CasePhase::from_slug(raw).map(Some).ok_or_else(|| {
        let valid = ALL_CASE_PHASES
            .iter()
            .map(|p| p.slug())
            .collect::<Vec<_>>()
            .join(", ");
        CasePhaseError::Unknown {
            supplied: raw.to_string(),
            valid,
        }
    })
}

#[cfg(test)]
#[path = "case_phase_tests.rs"]
mod tests;
