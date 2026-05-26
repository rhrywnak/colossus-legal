# Research — Four Specific Unknowns (2026-05-25)

Read-only investigation; no code changed. Each answer is backed by file:line and
verbatim snippets.

Key files:
- `backend/src/pipeline/steps/ingest.rs`
- `backend/src/api/pipeline/ingest_helpers.rs`
- `backend/src/pipeline/steps/llm_extract_pass2.rs`
- `backend/src/repositories/pipeline_repository/extraction_relationships.rs`
- `backend/src/repositories/pipeline_repository/extraction_items_pass1.rs`

---

## Question 1 — How does ingest resolve relationship endpoints?

**Answer: neither (a) nor (b) literally.** The `extraction_relationships` row stores the **integer primary keys** `from_item_id` / `to_item_id` (FK → `extraction_items.id`). At ingest time those integer PKs are resolved to the **Neo4j string node id** — the content-derived `stable_entity_id` (e.g. `"{slug}:element:abcd1234"`, `"{slug}:para:47"`) — and the Cypher MATCHes on that derived string id. It is **not** the raw `item_data["id"]` string (e.g. `"allegation-awad-para-47"`), and it is **not** the integer PK in the Cypher itself.

### Code path

1. When each node is created, ingest records `integer item_id → derived Neo4j string id` in `pg_to_neo4j`:
   - Non-Party — `ingest.rs:391–398`:
     ```rust
     let neo4j_id = create_entity_node(&mut txn, item, doc_id, *seq).await…?;
     pg_to_neo4j.insert(item.id, neo4j_id.clone());
     ```
   - Party — `create_party_nodes(&mut txn, &items, doc_id, &mut pg_to_neo4j, …)` (`ingest.rs:363–370`) populates the same map.
   - `create_entity_node` returns the `stable_entity_id` (`ingest_helpers.rs:369`, `380–391`): `MERGE (n:{entity_type} {id: $id})` where `$id = stable_entity_id(item, doc_id)`.

2. For each relationship, the integer PK is resolved to the string node id — local via `pg_to_neo4j`, cross-document via the stored `extraction_items.neo4j_node_id` column (`lookup_neo4j_node_ids`). `ingest.rs:495–519`:
   ```rust
   let from_neo = pg_to_neo4j
       .get(&rel.from_item_id)
       .or_else(|| cross_doc_neo4j_ids.get(&rel.from_item_id))
       .ok_or_else(|| … "No Neo4j ID for from_item_id {…}" …)?;
   let to_neo = pg_to_neo4j
       .get(&rel.to_item_id)
       .or_else(|| cross_doc_neo4j_ids.get(&rel.to_item_id))
       .ok_or_else(|| … )?;
   ```
   (`cross_doc_neo4j_ids` is built at `ingest.rs:438–456` from `lookup_neo4j_node_ids`; unresolved endpoints fail loudly with an enriched message, `ingest.rs:462–519`.)

3. The Cypher MATCH uses those **string** ids — `ingest_helpers.rs:554–565` (`build_relationship_with_provenance_cypher`):
   ```cypher
   MATCH (a {id: $from_id}), (b {id: $to_id})
    MERGE (a)-[r:{rel_type}]->(b)
    ON CREATE SET r.source_document_id=$source_document_id, r.extraction_run_id=$extraction_run_id, r.created_at=datetime()
    …
   ```
   `create_ingest_relationship(&mut txn, from_neo, to_neo, &rel.relationship_type, doc_id, &extraction_run_id)` (`ingest.rs:526–533`) binds `$from_id = from_neo`, `$to_id = to_neo`.

**Summary:** row PK (int) → `pg_to_neo4j` / `neo4j_node_id` → derived stable string id → Cypher `MATCH … {id: $from_id/$to_id}`.

---

## Question 2 — How does Pass 2 reference entities, and how is its output mapped back to FK rows?

**Answer: Pass 2 outputs the LLM-authored STRING ids** from `entities_json` (e.g. `"allegation-008"`, `"element-001"`; cross-doc entities are prefixed `"ctx:…"`). The pipeline maps those strings → `extraction_items` **integer** `item_id` via an `id_map`, then inserts `extraction_relationships` rows whose `from_item_id`/`to_item_id` are those integers (the FK references).

### The id_map (string → integer item_id) — `llm_extract_pass2.rs:545–552`
```rust
let mut id_map: std::collections::HashMap<String, i32> = entities
    .iter()
    .filter(|e| !e.id.is_empty())
    .map(|e| (e.id.clone(), e.item_id))   // Pass-1 LLM id → extraction_items.id
    .collect();
for c in &cross_doc_entities {
    id_map.insert(c.prefixed_id.clone(), c.item_id);  // "ctx:…" → extraction_items.id
}
```
(`e.id` is `item_data["id"]`; `e.item_id` is the `extraction_items` PK — `extraction_items_pass1.rs:96–108`, `from_item_record`.)

### Parse + store — `llm_extract_pass2.rs:645–658`
```rust
let parsed = match parse_chunk_response(&response.text) { … };
let rel_count =
    extraction::store_pass2_relationships(db, run_id, document_id, &parsed, &id_map).await?;
```

### `store_pass2_relationships` — `extraction_relationships.rs:342–380`
```rust
if let Some(rels) = parsed.get("relationships").and_then(|v| v.as_array()) {
    for rel in rels {
        let (from_key, to_key, relationship_type) = resolve_relationship_fields(rel);
        let (Some(&from_id), Some(&to_id)) = (id_map.get(from_key), id_map.get(to_key)) else {
            tracing::warn!(… "Pass 2: skipping relationship with unresolved endpoint(s)");
            continue;                                  // skip-and-log, not an error
        };
        insert_extraction_relationship(pool, run_id, document_id,
            from_id, to_id, relationship_type, rel.get("properties"), 1).await?;
        rel_count += 1;
    }
}
```
- `resolve_relationship_fields` (`extraction_relationships.rs:189–206`) accepts both the schema-compliant `{from_entity, to_entity, relationship_type}` and the short `{from, to, type}` forms; missing type defaults to `"UNKNOWN"`.
- `insert_extraction_relationship` (`extraction_relationships.rs:41–66`) writes the row with integer `from_item_id`/`to_item_id` and the `relationship_type` string.
- An endpoint string that is **not** in `id_map` (e.g. a hallucinated id) causes that relationship to be **skipped and logged**, not stored.

---

## Question 3 — A relationship type the schema YAML doesn't declare (e.g. `CHARACTERIZES`)?

**Answer: there is NO schema validation of relationship types. The type is stored in `extraction_relationships` regardless, and later written to Neo4j regardless.** The only gate is endpoint resolvability (Q2) at storage and a Cypher-injection guard at ingest.

- **Storage (Pass 2):** `store_pass2_relationships` (`extraction_relationships.rs:342–380`, quoted above) does not reference the schema, `relationship_types`, or `valid_patterns`. It inserts whatever `relationship_type` string `resolve_relationship_fields` returns (defaulting to `"UNKNOWN"` only when the field is entirely absent). A `CHARACTERIZES` relationship whose Evidence and Party endpoints both resolve via `id_map` is inserted like any other.
- **Ingest (to Neo4j):** the relationship type is interpolated directly as the edge label with only an alphanumeric/`_` injection guard — no allowlist. `create_ingest_relationship` (`ingest_helpers.rs:584–603`):
  ```rust
  if !rel_type.chars().all(|c| c.is_alphanumeric() || c == '_') {
      return Err(AppError::BadRequest { … "Invalid relationship type: '{rel_type}'" … });
  }
  validate_relationship_provenance(rel_type, source_document_id, extraction_run_id)?;
  let cypher = build_relationship_with_provenance_cypher(rel_type);  // MERGE (a)-[r:{rel_type}]->(b)
  ```
  `validate_relationship_provenance` (`ingest_helpers.rs:497–521`) only checks that `source_document_id` and `extraction_run_id` are non-empty — not the type.

**Net:** undeclared types (e.g. `CHARACTERIZES`, which `discovery_response_pass2_v5_1.md` instructs but `discovery_response_schema_v5_1.yaml` does not declare) are neither rejected nor warned-on for being undeclared; they are persisted and ingested. The schema's `relationship_types` / `valid_patterns` are LLM guidance in the prompt, not an enforced storage/ingest filter. (No code path was found that cross-checks emitted relationship types against the schema.)

---

## Question 4 — `stable_entity_id` for an `Element`

### Code — `ingest_helpers.rs:71–72` (prefix) and `130–158` (Element arm)
```rust
pub fn stable_entity_id(item: &ExtractionItemRecord, doc_id: &str) -> String {
    let doc_slug = slug(doc_id);
    match item.entity_type.as_str() {
        …
        ENTITY_ELEMENT => {
            let props = &item.item_data["properties"];
            let parent       = props["parent_count_id"].as_str().unwrap_or("");
            let anchors      = props["anchor_paragraph_numbers"].as_str().unwrap_or("");
            let element_name = props["element_name"].as_str().unwrap_or("");

            // Fallback if all three identifying props are empty:
            if parent.is_empty() && anchors.is_empty() && element_name.is_empty() {
                let data_str = serde_json::to_string(&item.item_data).unwrap_or_default();
                let hash = format!("{:x}", Sha256::digest(data_str.as_bytes()));
                return format!("{}:element:hash-{}", doc_slug, &hash[..8]);
            }

            // Normalize anchors: split on ',', trim, sort, rejoin
            let mut anchor_parts: Vec<&str> = anchors.split(',').map(str::trim).collect();
            anchor_parts.sort();
            let anchor_normalized = anchor_parts.join(",");

            let key_input = format!("{}|{}|{}", parent, anchor_normalized, element_name);
            let hash = format!("{:x}", Sha256::digest(key_input.as_bytes()));
            format!("{}:element:{}", doc_slug, &hash[..8])
        }
        …
    }
}
```

**Properties hashed:** `parent_count_id`, the **normalized** `anchor_paragraph_numbers` (comma-split, trimmed, sorted, rejoined), and `element_name`. `verbatim_text`/`order_in_count` are NOT part of the identity (comment L132–133). `doc_slug = slug(doc_id)`.

**Output format:** `{doc_slug}:element:{first 8 hex chars of sha256("{parent_count_id}|{anchor_normalized}|{element_name}")}`.

### The given example
- `parent_count_id` = `"count-001"`
- `anchor_paragraph_numbers` = `"74,75"` → normalized: split→`["74","75"]`, trim, sort→`["74","75"]`, rejoin → `"74,75"` (already sorted)
- `element_name` = `"Existence of fiduciary duty"`
- doc slug = `"awad-v-catholic-family-complaint-11-1-13"` (= `slug(doc_id)`)

Hash input (exact bytes): `count-001|74,75|Existence of fiduciary duty`

Generated Neo4j node id:
```
awad-v-catholic-family-complaint-11-1-13:element:<first 8 hex chars of sha256("count-001|74,75|Existence of fiduciary duty")>
```
The 8-character suffix is the first 8 hex characters of the SHA-256 digest of that exact string. It is deterministic, but computing the digest requires running SHA-256 (not done here, per the read-only/no-commands constraint) — so the literal 8 hex chars are not stated rather than guessed. Everything else in the id is fixed as shown.

---

*End of report. Read-only; no recommendations.*
