// Allegation repository — lists allegations grouped by complaint section.
//
// ## Why paragraph-based grouping?
//
// The previous query traversed `Allegation -[:BEARS_ON]-> Element
// <-[:HAS_ELEMENT]- LegalCount` to determine which Count each allegation
// belonged to. Because the LLM non-deterministically assigned BEARS_ON
// links, Common Allegations (¶7-71) sometimes appeared under specific Counts
// and sometimes had no link at all, falling into a frontend "Jurisdictional
// & Procedural" fallback bucket.
//
// Paragraph number ranges are deterministic and match the Awad complaint's
// actual structure. The Cypher CASE expression maps each allegation to its
// complaint section by paragraph number, then joins against LegalCount nodes
// to get the real Count IDs and titles.
//
// v5.1 schema: label is `Allegation`, properties are `paragraph_number` and
// `summary`. The Cypher aliases preserve the wire field names (`paragraph`,
// `allegation`, `evidence_status`) so the frontend DTO is unchanged.

use neo4rs::{query, Graph};

use crate::dto::{AllegationDto, AllegationSummary, AllegationsResponse};

#[derive(Clone)]
pub struct AllegationRepository {
    graph: Graph,
}

#[derive(Debug)]
pub enum AllegationRepositoryError {
    Neo4j(neo4rs::Error),
    Value(neo4rs::DeError),
}

impl From<neo4rs::Error> for AllegationRepositoryError {
    fn from(value: neo4rs::Error) -> Self {
        AllegationRepositoryError::Neo4j(value)
    }
}

impl From<neo4rs::DeError> for AllegationRepositoryError {
    fn from(value: neo4rs::DeError) -> Self {
        AllegationRepositoryError::Value(value)
    }
}

impl AllegationRepository {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// Fetch all allegations from Neo4j, grouped by deterministic complaint
    /// section (paragraph number ranges) instead of relationship traversal.
    ///
    /// ## Rust Learning: Cypher CASE expression
    ///
    /// The `CASE … WHEN … THEN … ELSE … END` in Cypher works like a match
    /// expression in Rust. Here it maps paragraph number ranges to section
    /// numbers (0 = common, 1-4 = legal counts, -1 = unassigned). The
    /// OPTIONAL MATCH then joins LegalCount nodes for sections 1-4 to get
    /// real Count IDs and titles. Sections 0 and -1 get synthetic values.
    ///
    /// # Ordering
    /// Rows come back in document order — primary key is `toInteger(a.paragraph_number)`
    /// so paragraph "2" precedes paragraph "10" (lexicographic sorts put "10"
    /// first). Paragraphs with non-numeric suffixes (e.g. "15(a)") return
    /// `null` from `toInteger`, which Cypher sorts last; the secondary
    /// `a.paragraph_number` key keeps those grouped in lexicographic order
    /// among themselves, and `a.id` is a stable tiebreaker.
    ///
    /// # Boilerplate filtering
    /// Incorporation paragraphs (¶7, 72, 86, 101, 115) that say
    /// "Incorporates paragraphs X through Y by reference" are filtered out.
    /// These are procedural boilerplate, not substantive allegations.
    pub async fn list_allegations(&self) -> Result<AllegationsResponse, AllegationRepositoryError> {
        // TODO: Paragraph ranges are Awad-case-specific (Awad v. CFS / Phillips).
        // For multi-case support, section boundaries should come from the
        // LegalCount nodes or a case configuration table, not hardcoded constants.
        //
        // Awad complaint section boundaries:
        // | Section          | ¶ start | ¶ end | Maps to Count # |
        // |------------------|---------|-------|-----------------|
        // | Common           |       7 |    71 |  0 (synthetic)  |
        // | Count I  (BFD)   |      72 |    85 |  1              |
        // | Count II (Fraud) |      86 |   100 |  2              |
        // | Count III (DR)   |     101 |   114 |  3              |
        // | Count IV (AoP)   |     115 |   126 |  4              |
        let common_start: i64 = 7;
        let common_end: i64 = 71;
        let count_1_start: i64 = 72;
        let count_1_end: i64 = 85;
        let count_2_start: i64 = 86;
        let count_2_end: i64 = 100;
        let count_3_start: i64 = 101;
        let count_3_end: i64 = 114;
        let count_4_start: i64 = 115;
        let count_4_end: i64 = 126;

        let mut allegations: Vec<AllegationDto> = Vec::new();

        // Why: coalesce(a.summary, '') guards against null — without it,
        // `null STARTS WITH 'X'` returns null in Cypher, which a WHERE clause
        // treats as false, silently dropping allegations with no body text.
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (a) WHERE labels(a)[0] = $allegation_label
                       AND NOT coalesce(a.summary, '') STARTS WITH 'Incorporates paragraphs'
                     WITH a, toInteger(a.paragraph_number) AS p
                     WITH a, p,
                       CASE
                         WHEN p >= $common_start AND p <= $common_end THEN 0
                         WHEN p >= $count_1_start AND p <= $count_1_end THEN 1
                         WHEN p >= $count_2_start AND p <= $count_2_end THEN 2
                         WHEN p >= $count_3_start AND p <= $count_3_end THEN 3
                         WHEN p >= $count_4_start AND p <= $count_4_end THEN 4
                         ELSE -1
                       END AS section_num
                     OPTIONAL MATCH (lc) WHERE labels(lc)[0] = $count_label
                       AND lc.count_number = section_num
                       AND section_num > 0
                     RETURN a.id AS id,
                            a.paragraph_number AS paragraph,
                            a.title AS title,
                            a.summary AS allegation,
                            NULL AS evidence_status,
                            a.category AS category,
                            a.severity AS severity,
                            CASE
                              WHEN section_num = 0 THEN ['common-allegations']
                              WHEN section_num > 0 AND lc IS NOT NULL THEN [lc.id]
                              ELSE ['unassigned']
                            END AS legal_count_ids,
                            CASE
                              WHEN section_num = 0 THEN ['Common Allegations']
                              WHEN section_num > 0 AND lc IS NOT NULL THEN [coalesce(lc.title, lc.name)]
                              ELSE ['Unassigned']
                            END AS legal_counts
                     ORDER BY toInteger(a.paragraph_number),
                              a.paragraph_number,
                              a.id",
                )
                .param("allegation_label", "Allegation")
                .param("count_label", "LegalCount")
                .param("common_start", common_start)
                .param("common_end", common_end)
                .param("count_1_start", count_1_start)
                .param("count_1_end", count_1_end)
                .param("count_2_start", count_2_start)
                .param("count_2_end", count_2_end)
                .param("count_3_start", count_3_start)
                .param("count_3_end", count_3_end)
                .param("count_4_start", count_4_start)
                .param("count_4_end", count_4_end),
            )
            .await?;

        while let Some(row) = result.next().await? {
            let id: String = row.get("id").unwrap_or_default();
            let title: String = row.get("title").unwrap_or_default();
            let paragraph: Option<String> = row.get("paragraph").ok();
            let allegation: Option<String> = row.get("allegation").ok();
            let evidence_status: Option<String> = row.get("evidence_status").ok();
            let category: Option<String> = row.get("category").ok();
            let severity: Option<i64> = row.get("severity").ok();
            let legal_count_ids: Vec<String> = row
                .get::<Vec<Option<String>>>("legal_count_ids")
                .unwrap_or_default()
                .into_iter()
                .flatten()
                .collect();
            let legal_counts: Vec<String> = row
                .get::<Vec<Option<String>>>("legal_counts")
                .unwrap_or_default()
                .into_iter()
                .flatten()
                .collect();

            allegations.push(AllegationDto {
                id,
                paragraph,
                title,
                allegation,
                evidence_status,
                category,
                severity,
                legal_count_ids,
                legal_counts,
            });
        }

        // Domain note: allegations outside all paragraph ranges should not
        // exist in the Awad complaint, but if they do, we log a warning
        // rather than silently dropping them. They get "unassigned" as their
        // section so the frontend can display them in a fallback bucket.
        let unassigned: Vec<&AllegationDto> = allegations
            .iter()
            .filter(|a| a.legal_count_ids.first().map(|id| id.as_str()) == Some("unassigned"))
            .collect();
        if !unassigned.is_empty() {
            let paragraphs: Vec<String> = unassigned
                .iter()
                .map(|a| a.paragraph.clone().unwrap_or_else(|| "null".to_string()))
                .collect();
            tracing::warn!(
                count = unassigned.len(),
                paragraphs = ?paragraphs,
                "Allegations fell outside all complaint section paragraph ranges"
            );
        }

        let proven = allegations
            .iter()
            .filter(|a| a.evidence_status.as_deref() == Some("PROVEN"))
            .count();
        let partial = allegations
            .iter()
            .filter(|a| a.evidence_status.as_deref() == Some("PARTIAL"))
            .count();
        let unproven = allegations
            .iter()
            .filter(|a| a.evidence_status.as_deref() == Some("UNPROVEN"))
            .count();

        let total = allegations.len();
        let summary = AllegationSummary {
            proven,
            partial,
            unproven,
        };

        Ok(AllegationsResponse {
            allegations,
            total,
            summary,
        })
    }
}
