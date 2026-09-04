//! The card as this audit sees it, and the bucket flags it carries.
//!
//! One `Card` per `Evidence` node, widened with the two Postgres joins the graph
//! cannot answer on its own: the document's page count (B9) and the extraction
//! run's template and model (B11).

/// Every field the audit reads, from both stores.
///
/// ## The property names are the ones that EXIST, not the ones the task named
///
/// STOP 0 measured the corpus rather than trusting the brief. Four of the names
/// in the instruction are not on any node — `evidence_id` is `id`, `quote` is
/// `verbatim_quote` (no node carries both), `page` is `page_number`,
/// `document_id` is `source_document` — and three (`template_file`,
/// `extraction_model`, `extraction_engine`) are on no node at all; they live in
/// Postgres. This struct is named for what is there.
#[derive(Debug, Clone)]
pub struct Card {
    /// The graph property `id`. The instruction called it `evidence_id`.
    pub id: String,
    /// The graph property `source_document`, which equals `documents.id`.
    pub source_document: String,
    pub page_number: Option<i64>,
    pub quote: String,
    pub title: String,
    pub question: Option<String>,
    pub statement_type: Option<String>,
    pub grounding_status: String,
    /// `ABOUT` edges reaching a `Person` or `Organization` — the party sense.
    pub party_count: i64,
    /// Parties reached by `ABOUT` whose `name` is null or blank.
    pub unnamed_party_count: i64,
    /// Distinct `Allegation` nodes reached by ANY of the four allegation-bearing
    /// relationship types. See `buckets::ALLEGATION_RELS`.
    pub allegation_count: i64,
    /// `CONTAINED_IN` edges reaching a `Document` node.
    pub document_node_count: i64,

    // ── joined from Postgres ────────────────────────────────────────────────
    /// `documents.page_count` for `source_document`; `None` when no row matches.
    pub doc_page_count: Option<i64>,
    /// Whether a `documents` row exists for `source_document`.
    pub doc_row_exists: bool,
    /// `extraction_runs.template_name`, via `extraction_items.neo4j_node_id`.
    pub template_name: Option<String>,
    /// `extraction_runs.model_name`, same join.
    pub model_name: Option<String>,
}

impl Card {
    /// The normalised quote — computed once per card and reused by B2 and B3.
    pub fn normalised(&self) -> String {
        crate::norm::normalise_quote(&self.quote)
    }
}

/// The twelve buckets, as a fixed list so the CSV header, the summary table and
/// the overlap matrix cannot drift apart.
///
/// ## Rust Learning: a `const` array of tuples as the single source of truth
///
/// Three separate renderers need "the buckets, in order, with their labels". A
/// `const` array iterated by all three means adding B13 is one edit, and the
/// compiler catches a renderer that indexes past the end — where three
/// hand-maintained lists would silently disagree.
pub const BUCKETS: [(&str, &str); 12] = [
    ("B1", "No retrievable text (blank, or a bare answer token)"),
    (
        "B2",
        "Exact duplicate quote (normalised) shared with another card",
    ),
    (
        "B3",
        "Near duplicate — one quote contains the other end-to-end",
    ),
    ("B4", "Grounding suspect (unverified or derived)"),
    ("B5", "No party — no ABOUT edge to a Person or Organization"),
    ("B6", "No allegation — coverage, not damage"),
    (
        "B7",
        "Bare cross-reference (prefilter's dropped statement_type)",
    ),
    ("B8", "OCR damage (§C1 signatures)"),
    (
        "B9",
        "Page unresolvable (null, zero, or past the document's end)",
    ),
    ("B10", "Orphan — source_document matches no document"),
    (
        "B11",
        "Extraction provenance missing (no template or model)",
    ),
    ("B12", "Mirror row missing or blank (evidence_search)"),
];

/// The per-card bucket flags, in `BUCKETS` order.
#[derive(Debug, Clone, Copy, Default)]
pub struct Flags(pub [bool; 12]);

impl Flags {
    /// Is this card in no bucket at all — "clean by these rules"?
    pub fn clean(&self) -> bool {
        !self.0.iter().any(|b| *b)
    }

    /// `0`/`1` for the CSV.
    pub fn digit(&self, index: usize) -> u8 {
        u8::from(self.0.get(index).copied().unwrap_or(false))
    }
}
