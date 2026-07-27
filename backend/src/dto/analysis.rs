//! Response DTOs for `GET /api/analysis` — the per-Allegation evidence tally
//! behind the Case Evidence & Analysis browser (`/explorer`).
//!
//! ## What this payload no longer carries, and why
//!
//! Until the 2026-07-27 honesty batch this endpoint also emitted a **strength
//! percentage and category** per Allegation, plus corpus-wide `strong` /
//! `moderate` / `weak` / `gap` bucket counts. Those were retired, not moved: the
//! percentage came from a five-row lookup table keyed on the evidence count
//! (0 → 25%, 1 → 60%, 2 → 80%, 3 → 90%, 4+ → 95%), so it carried no information
//! the count itself does not, while presenting as a measurement — and the
//! browser published the invented scale to the user as though it were method.
//! The honest figure is [`AllegationStrength::supporting_evidence_count`], which
//! survives unchanged. A real coverage verdict belongs to the Case State work,
//! not to a substitute invented here.
//!
//! Two whole sections went with the retirement, because deleting the page that
//! rendered them left them with no reader:
//!
//! - `contradictions_summary` — duplicated `GET /api/contradictions`, which the
//!   Contradictions page still serves.
//! - `evidence_coverage` — a third per-document "evidence produced vs. linked"
//!   table whose definition of *linked* (CORROBORATES ∪ the legacy MotionClaim
//!   path) matched neither of the other two in the product. Case Health Pane 1
//!   is the surviving per-document connection surface, and it labels its two
//!   tiers explicitly.
//!
//! The type name `AllegationStrength` is kept despite no longer carrying a
//! strength: renaming it would churn the frontend service, its tests and this
//! endpoint's only consumer for no reader benefit, and the retirement is
//! recorded here where the next reader will look.
//!
//! ## Why no `deny_unknown_fields` in this module
//!
//! `deny_unknown_fields` earns its keep on REQUEST bodies, where a client's typo
//! must fail loudly rather than be silently ignored. Nothing here is a request:
//! these are response shapes, and the only deserializer is a test. Making them
//! strict would buy no safety and would cost forward compatibility if a field is
//! ever added. Same posture as `dto::case_health`.

use serde::{Deserialize, Serialize};

/// One Allegation's evidence tally.
// serde: allows unknown fields because this is a response shape whose only deserializer is a test — see the module note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllegationStrength {
    pub id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allegation: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub paragraph: Option<String>,

    /// Number of DISTINCT evidence items reaching this Allegation, via the
    /// legacy `MotionClaim -[:PROVES]->` path or a direct `CORROBORATES` edge.
    /// A measured count, not a score.
    pub supporting_evidence_count: i32,

    /// Brief descriptions (up to 5) of those evidence items.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub supporting_evidence: Vec<String>,
}

/// The per-Allegation tally across the case.
///
/// `total_allegations` is a plain count of the rows below it — deliberately the
/// only summary figure left, now that the strength buckets are gone.
// serde: allows unknown fields because this is a response shape whose only deserializer is a test — see the module note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapAnalysis {
    pub total_allegations: i32,
    pub allegations: Vec<AllegationStrength>,
}

// ============================================================================
// Main Response DTO
// ============================================================================

/// Complete analysis response for `GET /analysis`.
// serde: allows unknown fields because this is a response shape whose only deserializer is a test — see the module note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResponse {
    pub gap_analysis: GapAnalysis,
}
