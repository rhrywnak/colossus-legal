# Code Inventory — 2026-05-25

Factual inventory of what the codebase implements today. No recommendations, no
speculation. Every claim is backed by a file path + line numbers; key snippets
are quoted. Read-only — no code was changed.

Template/schema roots:
- Schemas: `backend/extraction_schemas/`
- Templates: `backend/extraction_templates/`
- Pipeline migrations: `backend/pipeline_migrations/`
- String constants (entity/relationship/status names): `backend/src/models/document_status.rs`

---

## Section A — Entity/relationship types the templates produce

### A1. Schema YAML files and their `entity_types`

| Schema file (`backend/extraction_schemas/`) | `entity_types` | relationship_types (if present) |
|---|---|---|
| `discovery_response_schema_v5_1.yaml` | `Party`, `Evidence` | STATED_BY, ABOUT, CORROBORATES, CONTRADICTS, REBUTS |
| `complaint_schema_v5_1.yaml` | `Party`, `LegalCount`, `Element`, `Allegation`, `Harm` | HAS_ELEMENT, ANCHORED_IN, PROVES_ELEMENT, ABOUT, CAUSED_BY, DAMAGES_FOR, SUFFERED_BY, EVIDENCED_BY |
| `affidavit_schema_v5_1.yaml` | `Party`, `Evidence` | STATED_BY, ABOUT, CORROBORATES, CONTRADICTS, REBUTS |
| `complaint_v5.yaml` | `Party`, `LegalCount`, `Element`, `Allegation`, `ThematicAllegation`, `Harm` | — |
| `complaint_v4.yaml` | `Party`, `ComplaintAllegation`, `LegalCount`, `Harm` | — |
| `affidavit_v4.yaml` | `Party`, `Evidence` | STATED_BY, ABOUT, CONTAINED_IN, CORROBORATES, CONTRADICTS, REBUTS |
| `discovery_response_v4.yaml` | `Party`, `Evidence` | STATED_BY, ABOUT, CORROBORATES, CONTRADICTS, REBUTS |
| `brief_v4.yaml` | `Party`, `MotionClaim`, `Evidence` | — |
| `motion_v4.yaml` | `Party`, `MotionClaim`, `Evidence` | — |
| `court_ruling_v4.yaml` | `Party`, `Evidence` | — |
| `general_legal.yaml` | `Evidence` | — |

`discovery_response_schema_v5_1.yaml` verified verbatim: `entity_types` at lines 43–121 (`Party` L44, `Evidence` L67); `relationship_types` at lines 123–133 (STATED_BY, ABOUT, CORROBORATES, CONTRADICTS, REBUTS). It does **not** list `CHARACTERIZES`.

### A2. Pass-1 templates — entity types each instructs

| Pass-1 template (`backend/extraction_templates/`) | Entity types extracted |
|---|---|
| `discovery_response_pass1_v5_1.md` | `Party`, `Evidence` (L14: "extract ENTITIES ONLY — the people, organizations, and sworn Q&A pairs") |
| `complaint_pass1_v5_1.md` | `Party`, `LegalCount`, `Element`, `Allegation`, `Harm` |
| `affidavit_pass1_v5_1.md` | `Party`, `Evidence` |
| `pass1_complaint_v4.md` | `Party`, `LegalCount`, `Element`, `ComplaintAllegation`, `Harm` |
| `pass1_complaint_v5.md` | `Party`, `LegalCount`, `Element`, `Allegation`, `ThematicAllegation`, `Harm` |
| `pass1_affidavit_v4.md` | `Party`, `Evidence` |
| `pass1_brief_v4.md` | `Party`, `MotionClaim`, `Evidence` |
| `pass1_court_ruling_v4.md` | `Party`, `Evidence` |
| `pass1_discovery_response_v4.md` | `Party`, `Evidence` |

### A3. Pass-2 templates — relationship types each instructs

| Pass-2 template | Relationship types |
|---|---|
| `discovery_response_pass2_v5_1.md` | STATED_BY, ABOUT, CORROBORATES, CONTRADICTS, REBUTS, **CHARACTERIZES** |
| `complaint_pass2_v5_1.md` | HAS_ELEMENT, ANCHORED_IN, PROVES_ELEMENT, ABOUT, CAUSED_BY, DAMAGES_FOR, SUFFERED_BY, EVIDENCED_BY |
| `affidavit_pass2_v5_1.md` | STATED_BY, ABOUT, CORROBORATES, CONTRADICTS, REBUTS |
| `pass2_complaint_v4.md` | SUPPORTS, ABOUT, CAUSED_BY, DAMAGES_FOR, SUFFERED_BY |
| `pass2_complaint_v5.md` | HAS_ELEMENT, ANCHORED_IN, PROVES_ELEMENT, ABOUT, CAUSED_BY, DAMAGES_FOR, SUFFERED_BY, EVIDENCED_BY, PART_OF, THEME_SUPPORTS |
| `pass2_affidavit_v4.md` | STATED_BY, ABOUT, CORROBORATES, CONTRADICTS, REBUTS |
| `pass2_discovery_response_v4.md` | STATED_BY, ABOUT, CORROBORATES, CONTRADICTS, REBUTS |

### A4. Direct answers

**Does ANY template produce `Assertion` nodes?** **No.** No schema defines an `Assertion` entity type, and no pass-1 template instructs extracting one. `Assertion` appears only inside the *descriptions* of REBUTS/CONTRADICTS as a possible **cross-document target type**:
- `discovery_response_schema_v5_1.yaml:133` — "REBUTS targets Evidence or **Assertion** nodes from other documents"
- `affidavit_schema_v5_1.yaml` REBUTS description (same wording)
- `discovery_response_pass2_v5_1.md:127, 138, 185, 187` — REBUTS/CONTRADICTS reference "Evidence or Assertion entities from other documents"

(`ENTITY_ASSERTION = "Assertion"` is defined as a constant at `backend/src/models/document_status.rs:130`, but no schema/template extracts it.)

**Does ANY template produce `CHARACTERIZES` relationships?** **Yes — exactly one template:** `discovery_response_pass2_v5_1.md` (verified verbatim):
- L150 `### 6. CHARACTERIZES (Evidence → Party)`; rule L152; test L154; examples L156–159; "Do NOT" L161–163
- L175 `### Step 3: Create all CHARACTERIZES relationships`
- L223 lists it in the output enum; L235 example relationship; L255–258 checklist

Note: `CHARACTERIZES` is **not** listed in `discovery_response_schema_v5_1.yaml`'s `relationship_types` (L123–133) — the template instructs a relationship the schema does not declare. No other pass-2 template (affidavit, complaint) mentions CHARACTERIZES.

**`MADE_BY`** — zero occurrences anywhere in `extraction_schemas/` or `extraction_templates/`.

---

## Section B — What the ingest step writes to Neo4j

Files: `backend/src/pipeline/steps/ingest.rs`, `backend/src/api/pipeline/ingest_helpers.rs`, `backend/src/repositories/pipeline_repository/extraction_relationships.rs`.

### B1. Entity types — no whitelist
The ingest code writes a Neo4j node for **any** `entity_type`; the `entity_type` string is interpolated **directly as the node label**. The only gate is a Cypher-injection guard (alphanumeric + `_`). `ingest_helpers.rs:380–391` (`create_entity_node`):
```cypher
MERGE (n:{entity_type} {id: $id})
 ON CREATE SET n.title=$title, n.source_document=$doc_id, n.verbatim_quote=$verbatim_quote,
               n.grounding_status=$grounding_status, n.created_at=datetime()
 ON MATCH  SET n.title=$title, n.verbatim_quote=$verbatim_quote,
               n.grounding_status=$grounding_status, n.updated_at=datetime()
```
Injection guard at the top of `create_entity_node`: `if !entity_type.chars().all(|c| c.is_alphanumeric() || c == '_')` → `BadRequest`. No business-logic allowlist of entity types. Scalar `item_data["properties"]` (string/number/bool) are then `SET` individually (`ingest_helpers.rs:421–469`), each property name re-validated `[A-Za-z0-9_]`.

### B2. Relationship types — no whitelist
SELECT of approved relationships — `extraction_relationships.rs:94–115` (`get_approved_relationships_for_document_all_passes`):
```sql
SELECT r.* FROM extraction_relationships r
 JOIN extraction_runs rn ON rn.id = r.run_id
 JOIN extraction_items fi ON fi.id = r.from_item_id
 JOIN extraction_items ti ON ti.id = r.to_item_id
 WHERE r.document_id = $1 AND rn.status = $2
   AND fi.review_status = $3 AND ti.review_status = $3
 ORDER BY r.id
```
($2 = `RUN_STATUS_COMPLETED`, $3 = `REVIEW_STATUS_APPROVED`.) No filter on `relationship_type`. The write (`ingest_helpers.rs:554–565`, `build_relationship_with_provenance_cypher`) interpolates `rel_type` directly:
```cypher
MATCH (a {id:$from_id}),(b {id:$to_id})
 MERGE (a)-[r:{rel_type}]->(b)
 ON CREATE SET r.source_document_id=$source_document_id, r.extraction_run_id=$extraction_run_id, r.created_at=datetime()
 ON MATCH  SET r.source_document_id=coalesce(r.source_document_id,$source_document_id), ...
```
`create_ingest_relationship` (`ingest_helpers.rs:584–603`) validates `rel_type` only with the same injection guard — no allowlist.

### B3. Special-casing
Yes — **Party** is special-cased. `ingest.rs` routes `PARTY_SUBTYPES` (`["Party","Person","Organization"]`, `document_status.rs:139`) to `create_party_nodes` and everything else (incl. `Element`) to `create_entity_node`. `create_party_nodes` (`ingest_helpers.rs:247–332`) resolves the node to a `:Person` or `:Organization` label from `properties.party_type`/`entity_kind` (`is_org = party_type=="organization" || contains "org"`), and runs cross-document Party resolution (`ingest.rs:324–337`, `ingest_resolver`). Non-Party nodes get the generic MERGE on a content-derived stable id.

### B4. Provenance — `authored` vs `extracted`
There is **no `provenance: authored` value and no authored/extracted discriminator in the ingest writes.** Every relationship is stamped with three properties — `source_document_id`, `extraction_run_id`, `created_at` (`build_relationship_with_provenance_cypher`, validated by `validate_relationship_provenance` at `ingest_helpers.rs:497–521`). `extraction_run_id` is always produced by the code as `format!("run-{}", run_id)` (`ingest.rs:525`, and DERIVED_FROM/CONTAINED_IN at `ingest_helpers.rs`), i.e. always the `run-{id}` form.

The string `provenance` in the code refers to (a) those three relationship properties and (b) the `item_data["provenance"]` array that drives `DERIVED_FROM` edges (`create_provenance_relationships`, `ingest_helpers.rs:639+`). The grep for `authored`/`extracted` finds only comments ("LLM-authored id", "extracted items"), not a provenance type field.

Note (documented-but-unused convention): `discovery_response_schema_v5_1.yaml:146` documents `extraction_run_id` format as `'run-{i32}' for pipeline-extracted, 'manual-{user_id}-{ts}' for hand-authored, 'repair-{...}'`. The ingest code never emits the `manual-`/`repair-` forms — it only writes `run-{id}`.

---

## Section C — What the frontend queries from Neo4j (backend Neo4j repositories)

All active read repositories use **v5 names** (`Allegation`, `Element`, `PROVES_ELEMENT`, `HAS_ELEMENT`, `CONTAINED_IN`, `STATED_BY`, `CHARACTERIZES`, `REBUTS`, `CONTRADICTS`). `ComplaintAllegation` and a stored `SUPPORTS` edge are v4 and are no longer matched (see notes). Inventory of repository Cypher (file paths under `backend/src/repositories/` unless noted):

| API endpoint | Repository fn (file:line) | Node labels MATCHed | Relationship types traversed | Naming |
|---|---|---|---|---|
| `GET /api/cases/:slug/causes-of-action` | `causes_of_action_repository.rs` (`fetch_counts` ~L69; `fetch_elements` ~L85) | `LegalCount`, `Element`, `Allegation` | `HAS_ELEMENT`, `PROVES_ELEMENT` | v5 |
| `GET /api/case-summary` | `case_summary_repository.rs:~232`; `case_summary_elements.rs:~51` | `LegalCount`, `Allegation`, `Element` | `HAS_ELEMENT`, `PROVES_ELEMENT` | v5 |
| `GET /api/graph/legal-proof` | `graph_repository.rs:37` | `LegalCount`, `Allegation`, `Element`, `MotionClaim`, `Evidence`, `Document` | `PROVES_ELEMENT`, `HAS_ELEMENT`, `PROVES`, `RELIES_ON`, `CONTAINED_IN` | v5 (renders synthetic `SUPPORTS` — see note) |
| `GET /api/decomposition` | `decomposition_repository.rs:~66` | `Allegation`, `Evidence`, `Person`, `MotionClaim` | `CHARACTERIZES`, `STATED_BY`, `REBUTS`, `PROVES` | v5 |
| `GET /api/allegations/:id/detail` | `allegation_detail_repository.rs:~36` | `Allegation`, `Element`, `LegalCount`, `Evidence`, `Document`, `MotionClaim`, `Person` | `PROVES_ELEMENT`, `HAS_ELEMENT`, `CHARACTERIZES`, `REBUTS`, `CONTAINED_IN`, `STATED_BY`, `PROVES`, `RELIES_ON` | v5 |
| `GET /api/allegations/:id/evidence-chain` | `evidence_chain_repository.rs:~55` | `Allegation`, `Element`, `LegalCount`, `MotionClaim`, `Evidence`, `Document` | `PROVES_ELEMENT`, `HAS_ELEMENT`, `PROVES`, `RELIES_ON`, `CONTAINED_IN` | v5 |
| `GET /api/rebuttals` | `rebuttals_repository.rs:~23` | `Evidence`, `Person`, `Organization`, `Document` | `REBUTS`, `STATED_BY`, `CONTAINED_IN` | v5 |
| `GET /api/evidence` | `evidence_repository.rs:~48` | `Evidence`, `Document`, `Person` | `CONTAINED_IN`, `STATED_BY` | v5 |
| `GET /api/contradictions` | `contradiction_repository.rs:~42` | `Evidence`, `Document` | `CONTRADICTS`, `CONTAINED_IN` | v5 |
| `GET /api/allegations` | `allegation_repository.rs:~109` | `Allegation`, `LegalCount` | (join via `count_number`/paragraph ranges; no edge traversal) | v5 |
| `GET /api/persons` | `person_repository.rs:~47` | `Person` | — | v5 |
| `GET /api/persons/:id/detail` | `person_detail_repository.rs:~55` | `Person`, `Evidence`, `Document`, `Allegation` | `STATED_BY`, `CONTAINED_IN`, `CHARACTERIZES`, `REBUTS` | v5 |
| `GET /api/harms` | `harm_repository.rs:~51` | `Harm` | — (label fetch only) | v5 |

Notes (verified):
- **`graph_repository.rs:44–66, 132`** — code comment documents the v5.1 migration (`:ComplaintAllegation`→`:Allegation`; direct `:SUPPORTS`→ two-hop via Element). The query MATCHes `(a:Allegation)-[:PROVES_ELEMENT]->(el)<-[:HAS_ELEMENT]-(c:LegalCount)` (L69–70), and the edge it returns to the frontend is **built in code** as `relationship: "SUPPORTS"` (L132) — a synthetic display label for the Allegation→LegalCount link, **not** a stored Neo4j edge.
- `ComplaintAllegation` (v4) is no longer MATCHed by active read queries; migration comments noting the rename appear in `allegation_detail_repository.rs`, `evidence_chain_repository.rs`, `decomposition_repository.rs`, `case_summary_repository.rs`.
- `Assertion` (v5 constant) is not MATCHed by any frontend read query.

(Other repositories exist — e.g. `claim_repository.rs` MATCHing `Claim`, and an admin import path `api/.../admin_evidence.rs` that writes `STATED_BY`/`ABOUT`/`CONTRADICTS`/etc. — but those are not the primary frontend read surface. Endpoint paths above are as registered in `backend/src/api/mod.rs`.)

---

## Section D — Pipeline Postgres tables

### D1. Tables (pipeline DB; migrations in `backend/pipeline_migrations/`)
Created in `20260327_create_pipeline_tables.sql` (+ later ALTERs / additional migration files):
1. `documents` (L5–14) — id, title, file_path, file_hash, document_type, status (DEFAULT 'UPLOADED'), created_at, updated_at; many ALTER-added columns (reviewer, processing-progress, content/OCR, mime/format, restate_invocation_id).
2. `document_text` (L17–22) — document_id (FK→documents), page_number, text_content; PK (document_id, page_number).
3. `extraction_runs` (L25–38) — see D2-adjacent below; + F3 reproducibility columns (`20260410`), chunk counters (`20260412`), `processing_config JSONB` (`20260420`); UNIQUE (document_id, pass_number) (`20260422113610`).
4. `extraction_items` (L41–54) — see D2.
5. `extraction_relationships` (L57–69) — see D3.
6. `pipeline_config` (L72–83) — per-document config; heavily ALTERed (profile/template/chunking/model fields; legacy `pass1_model`/`pass2_model`/`*_max_tokens` dropped in `20260513`).
7. `pipeline_steps` (`20260401_create_pipeline_steps.sql`) — step audit (status lowercase).
8. `known_users` (`20260402_known_users_and_reviewer.sql`).
9. `document_audit_log` (`20260403_create_document_audit_log.sql`).
10. `review_edit_history` (`20260411_f5_review_edit_history.sql`) — item_id FK→extraction_items.
11. `extraction_chunks` (`20260412_fp7_chunk_extraction.sql`) — extraction_run_id FK→extraction_runs ON DELETE CASCADE.
12. `pipeline_jobs` (`20260417_create_pipeline_jobs_and_events.sql`).
13. `pipeline_events` (`20260417…`) — job_id FK→pipeline_jobs ON DELETE CASCADE.
14. `rag_config` (`20260418_create_rag_config.sql`).
15. `llm_models` (`20260420_config_system.sql`).

Separately, `backend/migrations/` (the main `colossus_legal` DB, not the pipeline DB) contains `20260524095049_case_metadata_tables.sql` → `cases`, `parties`, `counsel` (the relational source for the `GET /api/cases/:slug` case header). These are case-level metadata, not an entity/relationship store.

### D2. `extraction_items` FKs and NOT NULL (`20260327…:41–54`)
- FKs: `run_id INTEGER NOT NULL REFERENCES extraction_runs(id)`; `document_id TEXT NOT NULL REFERENCES documents(id)`. (No `ON DELETE` clause; no other FKs.)
- NOT NULL: `id` (PK), `run_id`, `document_id`, `entity_type`, `item_data` (JSONB), `review_status` (DEFAULT 'PENDING').
- Nullable / ALTER-added: `verbatim_quote`, `grounding_status`, `grounded_page`, `reviewed_by`, `reviewed_at`, `review_notes`; `graph_status TEXT DEFAULT 'pending'` (`20260413`), `neo4j_node_id VARCHAR(255)` (`20260421212806`), `resolved_entity_type VARCHAR(100)` (`20260421214108`), `verification_reason TEXT` (`20260509162937`).

### D3. `extraction_relationships` FKs (`20260327…:57–69`)
```sql
from_item_id INTEGER NOT NULL REFERENCES extraction_items(id),
to_item_id   INTEGER NOT NULL REFERENCES extraction_items(id),
```
Both endpoints **must** reference rows in `extraction_items(id)` — enforced by the FK. A relationship endpoint **cannot** reference an entity that is not a row in `extraction_items`. (Also FKs: `run_id`→extraction_runs, `document_id`→documents.)

### D4. Authored / human-created entity or relationship store?
**No.** No migration (in `backend/pipeline_migrations/` or `backend/migrations/`) creates a separate `manual_entities`/`authored_entities`/`authored_relationships`/`canonical_elements`-type table. All pipeline entities flow through `extraction_items`; all pipeline relationships through `extraction_relationships` (endpoints FK-constrained to `extraction_items`). Human edits are recorded in `review_edit_history` (field-level diffs on existing `extraction_items` rows), not as a separate entity store. The `cases`/`parties`/`counsel` tables are case metadata, not graph entities.

---

## Section E — The cross-document context loader

File: `backend/src/repositories/pipeline_repository/extraction_context.rs`.

### E1. Exact SQL (`load_cross_document_context`, L253–276)
```sql
SELECT i.id AS item_id, i.item_data, i.document_id AS source_document_id,
       docs.document_type AS source_document_type,
       COALESCE(i.resolved_entity_type, i.entity_type) AS effective_entity_type
FROM extraction_items i
JOIN extraction_runs runs ON runs.id = i.run_id
JOIN documents docs       ON docs.id = i.document_id
WHERE i.document_id <> $1
  AND docs.status     = $3              -- STATUS_PUBLISHED ('PUBLISHED')
  AND runs.pass_number = 1
  AND runs.status      = $4              -- RUN_STATUS_COMPLETED ('COMPLETED')
  AND i.review_status  = $5              -- REVIEW_STATUS_APPROVED ('approved')
  AND COALESCE(i.resolved_entity_type, i.entity_type) = ANY($2)
ORDER BY i.document_id, i.id
```

### E2. Entity-type whitelist ($2) — `CROSS_DOC_ENTITY_TYPES` (L80–90)
`Party`, `Person`, `Organization`, `LegalCount`, `ComplaintAllegation`, `Allegation`, `Evidence`, **`Element`**, `Harm`. (Locked by the regression test `cross_doc_entity_types_includes_v5_1_labels`, L408–443.) A per-type property allowlist trims payloads (`filter_properties_for_prompt`, L214–235; unknown types pass through whole).

### E3. Source: **Postgres only.** The loader reads `extraction_items` joined to `extraction_runs` and `documents`. There is **no Neo4j query** in this loader (or in the Pass-2 entity-context assembly that consumes it — `llm_extract_pass2.rs` passes `None` for the template's `{{context}}` in the complaint path and inlines cross-doc entities into `{{entities_json}}`).

### E4. Can it see a Neo4j-only entity? **No.** If an entity exists in Neo4j but has **no `extraction_items` row** (e.g. a canonical Element loaded directly into the graph), neither this loader nor `load_pass1_entities` can surface it — both read `extraction_items` exclusively. The `Element` entry in the whitelist only surfaces `Element` **rows that exist in `extraction_items`** (i.e. Elements extracted from a published document).

---

## Section F — discovery_response v5.1 templates (verified verbatim)

### F1. `discovery_response_schema_v5_1.yaml`
`entity_types`: `Party` (L44), `Evidence` (L67). `relationship_types` (L123–133): STATED_BY, ABOUT, CORROBORATES, CONTRADICTS, REBUTS. (No `CHARACTERIZES`; `Assertion` only in the REBUTS description, L133.)

### F2. `discovery_response_pass1_v5_1.md`
Instructs extracting `Party` and `Evidence` only (L14: "extract ENTITIES ONLY — the people, organizations, and sworn Q&A pairs"). **Does not mention `Assertion`.** (It discusses "characterizations" of parties as a quality of Evidence significance, L26 — but as a concept, not an entity or a relationship instruction.)

### F3. `discovery_response_pass2_v5_1.md`
Relationship types it tells the LLM to create: STATED_BY (L50), ABOUT (L61), CORROBORATES (L82), CONTRADICTS (L125), REBUTS (L138), and **CHARACTERIZES** (L150–163; created in "Step 3", L175; in the output enum, L223; example, L235; checklist, L255–258).
- **`CHARACTERIZES`**: **Yes**, present (Evidence → Party).
- **`MADE_BY`**: **No** — does not appear.
- `Assertion`: appears only as a possible cross-document target for CONTRADICTS/REBUTS (L127, L138, L185, L187), never as an extracted entity.

---

*End of inventory. Read-only; no recommendations.*
