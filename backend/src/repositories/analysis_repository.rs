use neo4rs::{query, Graph};

use crate::dto::{AllegationStrength, AnalysisResponse, GapAnalysis};
use crate::neo4j::schema;

#[derive(Clone)]
pub struct AnalysisRepository {
    graph: Graph,
}

#[derive(Debug)]
pub enum AnalysisRepositoryError {
    Neo4j(neo4rs::Error),
    Value(neo4rs::DeError),
}

impl From<neo4rs::Error> for AnalysisRepositoryError {
    fn from(value: neo4rs::Error) -> Self {
        AnalysisRepositoryError::Neo4j(value)
    }
}

impl From<neo4rs::DeError> for AnalysisRepositoryError {
    fn from(value: neo4rs::DeError) -> Self {
        AnalysisRepositoryError::Value(value)
    }
}

/// Build the gap-analysis query.
///
/// ## Rust Learning: why a `fn -> String` and not a `const`
///
/// A Rust `const` must be a compile-time literal — it cannot call `format!`. To
/// keep the relationship-type names in exactly one place we interpolate the
/// `schema::*` constants, which forces an owned `String` returned from a
/// function. Cypher cannot parameterize a relationship type, so interpolating
/// the trusted, code-defined constants is the established pattern here (see
/// `causes_of_action_repository::elements_query`). The query uses square
/// brackets for its list comprehensions but no literal `{ }` braces, so no
/// `{{`/`}}` escaping is needed.
///
/// v5.1 migration notes carried forward from the previous literal:
///   - `:ComplaintAllegation` → `:Allegation`.
///   - `a.allegation` (v4 prose) → `a.summary`.
///   - `a.paragraph` → `a.paragraph_number`.
///   - The evidence collection paths (MotionClaim PROVES, CORROBORATES) connect
///     directly to Allegation in both v4 and v5.1 — no traversal change.
fn gap_analysis_query() -> String {
    format!(
        "MATCH (a:Allegation)
         OPTIONAL MATCH (a)<-[:{proves}]-(mc:MotionClaim)-[:{relies_on}]->(e1)
         OPTIONAL MATCH (a)<-[:{corroborates}]-(e2)
         WITH a,
              collect(DISTINCT e1) AS motion_ev,
              collect(DISTINCT e2) AS direct_ev
         WITH a,
              [x IN motion_ev WHERE x IS NOT NULL] AS mev,
              [x IN direct_ev WHERE x IS NOT NULL] AS dev
         WITH a,
              mev + [x IN dev WHERE NOT x IN mev] AS evidence_list
         RETURN a.id AS id,
                a.summary AS allegation,
                a.paragraph_number AS paragraph,
                size(evidence_list) AS evidence_count,
                [e IN evidence_list | coalesce(e.title, e.label, e.summary, e.id)][0..5] AS evidence_titles
         ORDER BY size(evidence_list) DESC, a.id",
        proves = schema::PROVES,
        relies_on = schema::RELIES_ON,
        corroborates = schema::CORROBORATES,
    )
}

impl AnalysisRepository {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// Fetch the per-Allegation evidence tally from Neo4j.
    ///
    /// One read. The `contradictions_summary` and `evidence_coverage` sections
    /// this method used to assemble were retired in the 2026-07-27 honesty batch
    /// along with the page that rendered them — see the note in
    /// `crate::dto::analysis` for which surface now owns each.
    pub async fn get_analysis(&self) -> Result<AnalysisResponse, AnalysisRepositoryError> {
        Ok(AnalysisResponse {
            gap_analysis: self.fetch_gap_analysis().await?,
        })
    }

    /// Fetch gap analysis data
    ///
    /// Counts evidence for each `ComplaintAllegation` via two paths,
    /// union-deduped:
    ///
    ///   - Legacy path: `Evidence <-[:RELIES_ON]- MotionClaim -[:PROVES]-> ComplaintAllegation`
    ///     (motions that cite specific evidence to prove an allegation).
    ///   - Pass-2 cross-doc path: any node emitting a direct
    ///     `-[:CORROBORATES]->` edge into the allegation (e.g.,
    ///     `SwornStatement`, `Admission`, `CourtRuling`).
    ///
    /// `CONTRADICTS` edges are deliberately excluded — they're
    /// counter-evidence, not supporting evidence, and shouldn't move
    /// an allegation from "gap" to "strong." The leaf variables carry
    /// no label filter so pass-2 node types that aren't labeled
    /// `:Evidence` still count.
    async fn fetch_gap_analysis(&self) -> Result<GapAnalysis, AnalysisRepositoryError> {
        let mut allegations: Vec<AllegationStrength> = Vec::new();

        let mut result = self.graph.execute(query(&gap_analysis_query())).await?;

        while let Some(row) = result.next().await? {
            let id: String = row.get("id").unwrap_or_default();
            // best-effort: `summary` and `paragraph_number` are optional DISPLAY
            // fields — an Allegation legitimately carries neither, and both a
            // missing property and a decode miss render the same way (the row
            // shows no summary / no paragraph). Degrading to `None` therefore
            // loses nothing a reader could act on; the load-bearing columns
            // (`id`, `evidence_count`) are NOT treated this way.
            let allegation: Option<String> = row.get("allegation").ok();
            let paragraph: Option<String> = row.get("paragraph").ok();
            let evidence_count: i64 = row.get("evidence_count").unwrap_or(0);

            // Get evidence titles (up to 5)
            let evidence_titles: Vec<String> = row
                .get::<Vec<String>>("evidence_titles")
                .unwrap_or_default();

            allegations.push(AllegationStrength {
                id,
                allegation,
                paragraph,
                supporting_evidence_count: evidence_count as i32,
                supporting_evidence: evidence_titles,
            });
        }

        let total_allegations = allegations.len() as i32;

        Ok(GapAnalysis {
            total_allegations,
            allegations,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The gap-analysis query is built from the shared schema constants and keeps
    /// the aliases the row decoder reads. This guards two regressions a clean
    /// compile would miss: a relationship type drifting away from `neo4j::schema`
    /// (Rule 12), and a `RETURN` alias being renamed out from under
    /// `fetch_gap_analysis`, which would silently decode every row to its
    /// fallback (`unwrap_or_default` / `unwrap_or(0)`) and report an all-zero
    /// tally rather than an error.
    ///
    /// `fetch_gap_analysis` itself needs a live `neo4rs::Graph`, so it is
    /// exercised by DEV verification — the same convention
    /// `causes_of_action_repository` and `proof_matrix_repository` follow.
    #[test]
    fn gap_analysis_query_uses_schema_constants_and_its_decoded_aliases() {
        let q = gap_analysis_query();

        // Relationship types come from neo4j::schema, never inline literals.
        assert!(q.contains(&format!("<-[:{}]-", schema::PROVES)));
        assert!(q.contains(&format!("-[:{}]->", schema::RELIES_ON)));
        assert!(q.contains(&format!("<-[:{}]-", schema::CORROBORATES)));

        // Every alias `fetch_gap_analysis` decodes must be produced here.
        for alias in [
            "AS id",
            "AS allegation",
            "AS paragraph",
            "AS evidence_count",
            "AS evidence_titles",
        ] {
            assert!(q.contains(alias), "missing RETURN alias `{alias}`");
        }

        // Deterministic ordering, so two loads of the same graph render alike.
        assert!(q.contains("ORDER BY size(evidence_list) DESC, a.id"));
    }

    /// The union of the two evidence paths is deduped: an item reached BOTH by
    /// the legacy MotionClaim path and by a direct CORROBORATES edge must count
    /// once. That is what the `[x IN dev WHERE NOT x IN mev]` filter does, and
    /// losing it would inflate every allegation's evidence count — the exact
    /// class of quiet over-count this batch exists to remove.
    #[test]
    fn gap_analysis_query_dedupes_the_two_evidence_paths() {
        let q = gap_analysis_query();
        assert!(q.contains("mev + [x IN dev WHERE NOT x IN mev] AS evidence_list"));
    }

    /// The strength lookup table is gone and must not come back. Pinned as a
    /// query-level assertion too: a reinstated scale would need a bucket
    /// expression here, and there is none — the endpoint returns a raw count.
    #[test]
    fn gap_analysis_query_returns_a_count_not_a_score() {
        let q = gap_analysis_query();
        assert!(q.contains("size(evidence_list) AS evidence_count"));
        assert!(
            !q.to_lowercase().contains("strength"),
            "the fabricated strength scale was retired on 2026-07-27"
        );
    }
}
