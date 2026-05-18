//! Pass-1 entity loading for the pass-2 prompt.
//!
//! Reads the latest COMPLETED pass-1 run's `extraction_items` rows and
//! converts each into a prompt-shaped [`Pass1Entity`]. The
//! `to_prompt_value` projection drops DB internals so the pass-2 LLM
//! never sees `extraction_items.id`-shaped handles.
//!
//! Split out of [`super::extraction_items`] purely to keep that module
//! under the 300-line module budget. The two files share the same
//! `extraction_items` table and the same `ExtractionItemRecord` shape;
//! all logical coupling lives at compile time through the `super::`
//! import path below.

use sqlx::PgPool;

use crate::models::document_status::RUN_STATUS_COMPLETED;

use super::extraction_items::{get_items_for_run, ExtractionItemRecord};
use super::PipelineRepoError;

// ── Pass-2 support: cross-pass entity loading + relationship store ──

/// A pass-1 entity loaded for re-injection into the pass-2 prompt.
///
/// Carries both the LLM-supplied `id` string (e.g. `"party-001"` — used
/// to resolve pass-2 relationship endpoints back to `extraction_items`
/// rows) and the DB primary key (`item_id`) so the caller can build an
/// id → item_id map without a second query. The `to_prompt_value()`
/// helper emits only the prompt-facing subset so DB internals never
/// leak into the LLM input.
#[derive(Debug, Clone)]
pub struct Pass1Entity {
    /// `extraction_items.id` — the DB primary key. Used by the pass-2
    /// relationship writer to target the right FK.
    pub item_id: i32,
    /// The LLM's entity id as authored in pass 1 (e.g. `"party-001"`).
    /// Round-trips into pass 2's prompt so pass 2 can reference it, and
    /// then back out via the relationship payload.
    pub id: String,
    /// Effective entity type after Ingest resolution (falls back to the
    /// LLM's original type when no resolution has occurred).
    pub entity_type: String,
    /// Short human-readable label, if the pass-1 output supplied one.
    pub label: Option<String>,
    /// The `properties` object verbatim from the pass-1 entity JSON.
    /// Returned as `serde_json::Value::Object(Default::default())` when
    /// the pass-1 output omitted it, so the pass-2 prompt always sees a
    /// JSON object in this position.
    pub properties: serde_json::Value,
}

impl Pass1Entity {
    /// Build a `Pass1Entity` from a stored `extraction_items` row.
    ///
    /// Pass 1 stores the full entity JSON in `item_data`, so we parse
    /// `id` / `label` / `properties` back out of the JSONB. `entity_type`
    /// comes from the column (already COALESCE'd with
    /// `resolved_entity_type`), so the prompt sees the effective label
    /// — important when pass 2 is re-run after Ingest has resolved a
    /// Party into a Person/Organization.
    ///
    /// Visibility note: `pub(super)` so the test module in this file's
    /// sibling can construct a `Pass1Entity` from a synthetic record
    /// without going through the DB. The `to_prompt_value` projection
    /// stays fully `pub` — that one IS the public API.
    pub(super) fn from_item_record(rec: &ExtractionItemRecord) -> Self {
        let id = rec
            .item_data
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let label = rec
            .item_data
            .get("label")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let properties = rec
            .item_data
            .get("properties")
            .cloned()
            .unwrap_or_else(|| serde_json::Value::Object(Default::default()));
        Self {
            item_id: rec.id,
            id,
            entity_type: rec.entity_type.clone(),
            label,
            properties,
        }
    }

    /// Render the prompt-facing subset: `{id, entity_type, label?, properties}`.
    ///
    /// The DB `item_id` is intentionally omitted — it's a repo-internal
    /// handle that has no meaning to the LLM.
    pub fn to_prompt_value(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        obj.insert("id".into(), serde_json::Value::String(self.id.clone()));
        obj.insert(
            "entity_type".into(),
            serde_json::Value::String(self.entity_type.clone()),
        );
        if let Some(label) = &self.label {
            obj.insert("label".into(), serde_json::Value::String(label.clone()));
        }
        obj.insert("properties".into(), self.properties.clone());
        serde_json::Value::Object(obj)
    }
}

/// Load the pass-1 entities for a document so pass 2 can be given them
/// as input.
///
/// Selects the latest COMPLETED `extraction_runs` row where
/// `pass_number = 1` and returns its `extraction_items` as
/// [`Pass1Entity`] values. Returns an empty `Vec` when no completed
/// pass-1 run exists — the caller decides whether that's a user error
/// ("run pass 1 first") or a no-op.
pub async fn load_pass1_entities(
    pool: &PgPool,
    document_id: &str,
) -> Result<Vec<Pass1Entity>, PipelineRepoError> {
    let run_id: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM extraction_runs \
         WHERE document_id = $1 AND pass_number = 1 AND status = $2 \
         ORDER BY id DESC LIMIT 1",
    )
    .bind(document_id)
    .bind(RUN_STATUS_COMPLETED)
    .fetch_optional(pool)
    .await?;

    let Some(run_id) = run_id else {
        return Ok(Vec::new());
    };

    let items = get_items_for_run(pool, run_id).await?;
    Ok(items.iter().map(Pass1Entity::from_item_record).collect())
}

#[cfg(test)]
mod tests {
    //! Pure-function tests for `Pass1Entity`. The DB-touching
    //! `load_pass1_entities` is exercised by integration tests; the
    //! `from_item_record` and `to_prompt_value` projections are
    //! purely in-memory and can be asserted directly.
    use super::*;

    /// Construct a synthetic `ExtractionItemRecord` for the projection
    /// tests below. Mirrors the shape `query_as` would deserialise from
    /// the `extraction_items` table — every nullable column starts as
    /// `None` so tests opt in to what they exercise.
    fn item_record_with(
        id: i32,
        entity_type: &str,
        item_data: serde_json::Value,
    ) -> ExtractionItemRecord {
        ExtractionItemRecord {
            id,
            run_id: 42,
            document_id: "doc-x".into(),
            entity_type: entity_type.into(),
            item_data,
            verbatim_quote: None,
            grounding_status: None,
            grounded_page: None,
            review_status: "pending".into(),
            reviewed_by: None,
            reviewed_at: None,
            review_notes: None,
            graph_status: "pending".into(),
            neo4j_node_id: None,
            resolved_entity_type: None,
        }
    }

    #[test]
    fn pass1_entity_extracts_id_label_properties_from_item_data() {
        let rec = item_record_with(
            101,
            "Party",
            serde_json::json!({
                "id": "party-001",
                "entity_type": "Party",
                "label": "Marie Awad",
                "properties": { "full_name": "Marie Awad", "role": "Plaintiff" },
                "verbatim_quote": "Plaintiff Marie Awad..."
            }),
        );
        let e = Pass1Entity::from_item_record(&rec);
        assert_eq!(e.item_id, 101);
        assert_eq!(e.id, "party-001");
        assert_eq!(e.entity_type, "Party");
        assert_eq!(e.label.as_deref(), Some("Marie Awad"));
        assert_eq!(e.properties["full_name"], "Marie Awad");
    }

    #[test]
    fn pass1_entity_tolerates_missing_label_and_properties() {
        let rec = item_record_with(
            7,
            "LegalCount",
            serde_json::json!({ "id": "count-001", "entity_type": "LegalCount" }),
        );
        let e = Pass1Entity::from_item_record(&rec);
        assert_eq!(e.id, "count-001");
        assert!(e.label.is_none());
        assert!(
            e.properties.is_object(),
            "missing properties must default to empty object, got {:?}",
            e.properties
        );
        assert_eq!(e.properties.as_object().unwrap().len(), 0);
    }

    #[test]
    fn pass1_entity_to_prompt_value_omits_item_id() {
        let e = Pass1Entity {
            item_id: 9,
            id: "harm-001".into(),
            entity_type: "Harm".into(),
            label: Some("Financial loss".into()),
            properties: serde_json::json!({ "amount_usd": 50000 }),
        };
        let v = e.to_prompt_value();
        let obj = v.as_object().expect("prompt value must be a JSON object");
        assert!(
            !obj.contains_key("item_id"),
            "DB item_id must not leak into the prompt payload"
        );
        assert_eq!(obj["id"], "harm-001");
        assert_eq!(obj["entity_type"], "Harm");
        assert_eq!(obj["label"], "Financial loss");
        assert_eq!(obj["properties"]["amount_usd"], 50000);
    }

    #[test]
    fn pass1_entity_to_prompt_value_omits_label_when_absent() {
        let e = Pass1Entity {
            item_id: 1,
            id: "count-001".into(),
            entity_type: "LegalCount".into(),
            label: None,
            properties: serde_json::json!({}),
        };
        let obj = e.to_prompt_value();
        assert!(
            !obj.as_object().unwrap().contains_key("label"),
            "absent label must not serialize as null"
        );
    }
}
