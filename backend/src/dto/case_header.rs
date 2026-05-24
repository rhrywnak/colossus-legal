//! Response DTOs for `GET /api/cases/:slug` — the Home page case header.
//!
//! These types are **serialize-only** (the endpoint is read-only) and shape the
//! JSON exactly as `HOME_PAGE_REDESIGN_v2.md` §6 specifies.
//!
//! ## Why these DTOs deliberately diverge from the codebase's serde habit
//!
//! Most DTOs here use `#[serde(skip_serializing_if = "Option::is_none")]` to
//! omit absent fields. This endpoint does the opposite: a `None` field is
//! emitted as JSON `null` (the key is present). The Home page contract
//! distinguishes "absent" (`null`) from "explicitly empty" (`""`), and the
//! frontend (instruction 4) renders against fields that are always present —
//! so omitting them would break that contract. We therefore do **not** use
//! `skip_serializing_if` on these response types.

use chrono::NaiveDate;
use serde::Serialize;

/// The full case-header payload: caption, court strip, parties, counsel.
#[derive(Debug, Clone, Serialize)]
pub struct CaseHeaderResponse {
    pub case_id: String,
    pub case_slug: String,
    pub display_title: String,
    /// Full caption for tooltip/detail; `null` when the short title suffices.
    pub display_title_full: Option<String>,
    pub court: CourtInfo,
    pub status: String,
    pub complaint_document_id: Option<String>,
    pub parties: PartiesGroups,
    /// Always an array — `[]` when the case has no counsel rows (never omitted).
    pub counsel: Vec<CounselContact>,
}

/// The court / case-metadata strip beneath the title.
#[derive(Debug, Clone, Serialize)]
pub struct CourtInfo {
    pub name: Option<String>,
    pub jurisdiction: Option<String>,
    /// `null` when not yet assigned. Domain note: the seed stores "not yet
    /// populated" as an empty string, but a blank docket number carries no
    /// information for the header, so the builder collapses both `NULL` and
    /// `""` to `null` here (see `case_header_builder::build_case_header`).
    pub case_number: Option<String>,
    pub filed_date: Option<NaiveDate>,
    /// Originating court when venue was transferred; `null` if filed here.
    pub transferred_from: Option<String>,
    pub transfer_date: Option<NaiveDate>,
}

/// Parties grouped for the two-column Home page layout.
///
/// ## Why three buckets, grouped in Rust (not SQL)
///
/// The header shows Plaintiffs in one column and Defendants in the other, and
/// under Defendants a subtle "DROPPED" subheader lists no-longer-active
/// parties. Bucketing in Rust (rather than three status-filtered SQL queries)
/// keeps the rule in one readable place and lets us unit-test it without a
/// database. The split is **role-first, status-second**: a Plaintiff whose
/// status is "dropped" still appears under `plaintiffs` — status only triages
/// Defendants, because the design only renders the DROPPED subheader there.
#[derive(Debug, Clone, Serialize)]
pub struct PartiesGroups {
    pub plaintiffs: Vec<HeaderParty>,
    pub active_defendants: Vec<HeaderParty>,
    pub dropped_defendants: Vec<DroppedDefendant>,
}

/// A plaintiff or active defendant. (Dropped defendants use
/// [`DroppedDefendant`], which carries the extra dismissal fields the design
/// shows only for that group.)
#[derive(Debug, Clone, Serialize)]
pub struct HeaderParty {
    pub party_id: String,
    pub name: String,
    pub entity_type: Option<String>,
    pub notes: Option<String>,
    /// Internal display ordering. Not serialized: the backend already returns
    /// each group pre-sorted, so the frontend never needs the raw value. Kept
    /// as a field so the bucketing/sorting logic is unit-testable.
    #[serde(skip_serializing)]
    pub sort_order: i32,
}

/// A dropped / dismissed / settled defendant, with the dismissal detail the
/// Home page surfaces under the "DROPPED" subheader.
#[derive(Debug, Clone, Serialize)]
pub struct DroppedDefendant {
    pub party_id: String,
    pub name: String,
    pub entity_type: Option<String>,
    /// The specific non-active lifecycle state (`dropped` | `dismissed` |
    /// `settled`) — surfaced so the UI/operator can distinguish them.
    pub status: String,
    pub dismissal_date: Option<NaiveDate>,
    pub dismissal_basis: Option<String>,
    pub notes: Option<String>,
    /// See [`HeaderParty::sort_order`].
    #[serde(skip_serializing)]
    pub sort_order: i32,
}

/// One counsel-of-record row. Rendered one line per record on the header.
#[derive(Debug, Clone, Serialize)]
pub struct CounselContact {
    pub counsel_id: String,
    pub represents_role: String,
    pub firm_name: Option<String>,
    pub attorney_name: String,
    pub bar_number: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    /// See [`HeaderParty::sort_order`].
    #[serde(skip_serializing)]
    pub sort_order: i32,
}
