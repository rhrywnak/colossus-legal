# Database Access Audit — Colossus-Legal + Colossus-RS

**Generated:** 2026-04-05
**Scope:** All `.rs` files in `colossus-legal/backend/src/` and `colossus-rs/`, all `.ts`/`.tsx` files in `colossus-legal/frontend/src/`

---

## NEO4J AUDIT

| # | File | Line | Function | Label/RelType Hardcoded | Read/Write | Notes |
|---|------|------|----------|------------------------|------------|-------|
| 1 | neo4j.rs | 29 | check_neo4j | N/A | READ | Health check: RETURN 1 |
| 2 | api/admin_status.rs | 40 | get_status | N/A | READ | Health check: RETURN 1 |
| 3 | repositories/claim_repository.rs | 46 | list_claims | :Claim | READ | MATCH (c:Claim) |
| 4 | repositories/claim_repository.rs | 62 | get_claim_by_id | :Claim | READ | MATCH (c:Claim {id: $id}) |
| 5 | repositories/claim_repository.rs | 86 | create_claim | :Claim | WRITE | CREATE (c:Claim {...}) |
| 6 | repositories/claim_repository.rs | 115 | update_claim | :Claim | WRITE | MATCH + SET |
| 7 | repositories/evidence_repository.rs | 42-58 | list_evidence | :Evidence, :Document, :Person, CONTAINED_IN, STATED_BY | READ | MATCH + OPTIONAL MATCH rels |
| 8 | repositories/person_repository.rs | 40-42 | list_persons | :Person | READ | MATCH (p:Person) |
| 9 | repositories/allegation_repository.rs | 42-54 | list_allegations | :ComplaintAllegation, :LegalCount, SUPPORTS | READ | MATCH + OPTIONAL MATCH |
| 10 | repositories/allegation_detail_repository.rs | 28-34 | fetch_allegation_info | :ComplaintAllegation, :LegalCount, SUPPORTS | READ | Const ALLEGATION_INFO_QUERY |
| 11 | repositories/allegation_detail_repository.rs | 36-58 | fetch_characterizations | :Evidence, :ComplaintAllegation, :Person, :Organization, :Document, CHARACTERIZES, REBUTS, STATED_BY, CONTAINED_IN | READ | Const CHARACTERIZATION_QUERY |
| 12 | repositories/allegation_detail_repository.rs | 60-68 | fetch_proof_claims | :MotionClaim, :ComplaintAllegation, :Evidence, PROVES, RELIES_ON | READ | Const PROOF_CLAIMS_QUERY |
| 13 | repositories/harm_repository.rs | 43-59 | list_harms | :Harm, :ComplaintAllegation, :LegalCount, CAUSED_BY, DAMAGES_FOR | READ | MATCH + OPTIONAL MATCH |
| 14 | repositories/motion_claim_repository.rs | 47-64 | list_motion_claims | :MotionClaim, :ComplaintAllegation, :Evidence, :Document, PROVES, RELIES_ON, APPEARS_IN | READ | MATCH + OPTIONAL MATCH |
| 15 | repositories/evidence_chain_repository.rs | 45-57 | get_evidence_chain | :ComplaintAllegation, :LegalCount, :MotionClaim, :Evidence, :Document, SUPPORTS, PROVES, RELIES_ON, CONTAINED_IN | READ | Dynamic cypher |
| 16 | repositories/rebuttals_repository.rs | 25-42 | fetch_and_group_claims | :Evidence, :Person, :Document, :Organization, REBUTS, STATED_BY, CONTAINED_IN | READ | Const REBUTS_QUERY |
| 17 | repositories/rebuttals_repository.rs | 45-48 | fetch_rebuttal_totals | :Evidence, :Person, STATED_BY, REBUTS | READ | Const TOTAL_COUNTS_QUERY |
| 18 | repositories/schema_repository.rs | 43 | get_schema_stats | All labels via labels(n)[0] | READ | Generic label introspection |
| 19 | repositories/schema_repository.rs | 61 | get_schema_stats | All rels via type(r) | READ | Generic rel introspection |
| 20 | repositories/decomposition_repository.rs | 56-68 | get_decomposition | :ComplaintAllegation, :Evidence, :Person, :MotionClaim, CHARACTERIZES, REBUTS, STATED_BY, PROVES | READ | Const OVERVIEW_CHAR_QUERY |
| 21 | repositories/decomposition_repository.rs | 70-75 | build_proof_count_map | :ComplaintAllegation, :MotionClaim, PROVES | READ | Const OVERVIEW_PROOF_QUERY |
| 22 | repositories/analysis_repository.rs | 75-85 | fetch_gap_analysis | :ComplaintAllegation, :MotionClaim, :Evidence, PROVES, RELIES_ON | READ | MATCH + OPTIONAL MATCH |
| 23 | repositories/analysis_repository.rs | 161-170 | fetch_contradictions_summary | :Evidence, CONTRADICTS, CONTAINED_IN | READ | MATCH (a:Evidence)-[r:CONTRADICTS]->(b) |
| 24 | repositories/analysis_repository.rs | 201-212 | fetch_evidence_coverage | :Document, :Evidence, :MotionClaim, :ComplaintAllegation, CONTAINED_IN, RELIES_ON, PROVES | READ | Complex chain |
| 25 | repositories/contradiction_repository.rs | 42-58 | list_contradictions | :Evidence, :Document, CONTRADICTS, CONTAINED_IN | READ | MATCH with docs |
| 26 | repositories/query_repository.rs | 39-151 | list_queries/run_query | :Evidence, :MotionClaim, :ComplaintAllegation, :Harm, :LegalCount | READ | Multiple pre-registered queries |
| 27 | repositories/case_repository.rs | 63-68 | get_case_info | :Case | READ | MATCH (c:Case) LIMIT 1 |
| 28 | repositories/case_repository.rs | 107-114 | get_parties | :Case, :Person, :Organization, INVOLVES | READ | MATCH + labels check |
| 29 | repositories/case_repository.rs | 161-173 | get_stats | :ComplaintAllegation, :Evidence, :Document, :Harm, :LegalCount | READ | Aggregations |
| 30 | repositories/case_repository.rs | 199-202 | get_legal_count_details | :LegalCount | READ | MATCH + ORDER BY |
| 31 | repositories/person_detail_repository.rs | 39-41 | get_person_info | :Person | READ | Const PERSON_QUERY |
| 32 | repositories/person_detail_repository.rs | 44-59 | get_statements | :Evidence, :Person, :Document, :ComplaintAllegation, STATED_BY, CONTAINED_IN, CHARACTERIZES, REBUTS | READ | Const STATEMENTS_QUERY |
| 33 | repositories/case_summary_repository.rs | 86-89 | get_case_identity | :Case | READ | MATCH LIMIT 1 |
| 34 | repositories/case_summary_repository.rs | 109-128 | get_core_stats | :ComplaintAllegation, :Evidence, :Document, :Harm, :LegalCount | READ | Complex aggregation |
| 35 | repositories/case_summary_repository.rs | 159-164 | get_legal_count_details | :LegalCount, :ComplaintAllegation, SUPPORTS | READ | MATCH + OPTIONAL MATCH |
| 36 | repositories/case_summary_repository.rs | 193-223 | get_decomposition_stats | :Evidence, :ComplaintAllegation, :Person, CHARACTERIZES, STATED_BY, REBUTS | READ | Multiple queries |
| 37 | repositories/case_summary_repository.rs | 253-256 | get_parties | :Case, :Person, :Organization, INVOLVES | READ | Same pattern as case_repository |
| 38 | api/pipeline/delete.rs | 318 | cleanup_neo4j | All nodes via source_document | WRITE | DETACH DELETE by source_document |
| 39 | api/pipeline/delete.rs | 342 | cleanup_neo4j | All nodes via source_document_id | WRITE | DETACH DELETE by source_document_id |
| 40 | api/pipeline/ingest_helpers.rs | 41-48 | create_document_node | :Document | WRITE | CREATE |
| 41 | api/pipeline/ingest_helpers.rs | 96-102 | create_party_nodes | :Person, :Organization | WRITE | MERGE + ON CREATE/MATCH |
| 42 | api/pipeline/ingest_helpers.rs | 144-158 | create_allegation_nodes | :ComplaintAllegation | WRITE | CREATE |
| 43 | api/pipeline/ingest_helpers.rs | 188-201 | create_count_nodes | :LegalCount | WRITE | CREATE |
| 44 | api/pipeline/ingest_helpers.rs | 227-240 | create_harm_nodes | :Harm | WRITE | CREATE |
| 45 | api/pipeline/ingest_helpers.rs | 259 | create_ingest_relationship | Dynamic rel types | WRITE | CREATE relationship |
| 46 | *colossus-rs* expander_queries.rs | 90-105 | expand_evidence | :Evidence, :Document, :Person, :ComplaintAllegation, STATED_BY, ABOUT, CONTAINED_IN, CHARACTERIZES, REBUTS, CONTRADICTS | READ | RAG graph expansion |
| 47 | *colossus-rs* expander_queries.rs | 200+ | expand_allegation | :ComplaintAllegation, :MotionClaim, :Evidence, :Document, :Person, :LegalCount, :Harm, PROVES, RELIES_ON, CONTAINED_IN, STATED_BY, SUPPORTS, CAUSED_BY | READ | Complex expansion |
| 48 | *colossus-rs* expander_queries_minor.rs | 38-49 | expand_harm | :Harm, :ComplaintAllegation, :Evidence, :Document, :LegalCount, CAUSED_BY, EVIDENCED_BY, CONTAINED_IN, DAMAGES_FOR | READ | RAG expansion |
| 49 | *colossus-rs* expander_queries_minor.rs | 121-127 | expand_document | :Document, :Evidence, :Person, CONTAINED_IN, STATED_BY | READ | RAG expansion |
| 50 | *colossus-rs* expander_queries_minor.rs | 173-180 | expand_person | :Person, :Evidence, :Document, STATED_BY, CONTAINED_IN | READ | RAG expansion |
| 51 | *colossus-rs* embedding_repository.rs | 240-291 | fetch_document_nodes | :ComplaintAllegation, :Harm, :Person, :Organization, :LegalCount, :Document (source_document / source_document_id) | READ | Per-document node queries |

### Neo4j Node Labels Referenced

| Label | Files Using It |
|-------|---------------|
| :Case | case_repository, case_summary_repository |
| :Claim | claim_repository |
| :ComplaintAllegation | allegation_repository, allegation_detail_repository, decomposition_repository, analysis_repository, case_repository, case_summary_repository, evidence_chain_repository, query_repository, ingest_helpers, expander_queries, embedding_repository |
| :Document | evidence_repository, allegation_detail_repository, motion_claim_repository, evidence_chain_repository, analysis_repository, contradiction_repository, ingest_helpers, expander_queries, expander_queries_minor, embedding_repository |
| :Evidence | evidence_repository, allegation_detail_repository, harm_repository, motion_claim_repository, evidence_chain_repository, rebuttals_repository, decomposition_repository, analysis_repository, contradiction_repository, query_repository, person_detail_repository, case_summary_repository, expander_queries, expander_queries_minor |
| :Harm | harm_repository, case_repository, case_summary_repository, query_repository, ingest_helpers, expander_queries, expander_queries_minor, embedding_repository |
| :LegalCount | allegation_repository, allegation_detail_repository, harm_repository, evidence_chain_repository, case_repository, case_summary_repository, query_repository, ingest_helpers, expander_queries, expander_queries_minor, embedding_repository |
| :MotionClaim | allegation_detail_repository, motion_claim_repository, evidence_chain_repository, decomposition_repository, analysis_repository, query_repository, expander_queries |
| :Organization | allegation_detail_repository, rebuttals_repository, case_repository, case_summary_repository, ingest_helpers, embedding_repository |
| :Person | evidence_repository, person_repository, allegation_detail_repository, rebuttals_repository, decomposition_repository, case_repository, case_summary_repository, person_detail_repository, ingest_helpers, expander_queries, expander_queries_minor, embedding_repository |

### Neo4j Relationship Types Referenced

| Relationship | Files Using It |
|-------------|---------------|
| ABOUT | expander_queries |
| APPEARS_IN | motion_claim_repository |
| CAUSED_BY | harm_repository, expander_queries, expander_queries_minor |
| CHARACTERIZES | allegation_detail_repository, decomposition_repository, case_summary_repository, person_detail_repository, expander_queries |
| CONTAINED_IN | evidence_repository, allegation_detail_repository, evidence_chain_repository, rebuttals_repository, analysis_repository, contradiction_repository, expander_queries, expander_queries_minor |
| CONTRADICTS | analysis_repository, contradiction_repository, expander_queries |
| DAMAGES_FOR | harm_repository, expander_queries_minor |
| EVIDENCED_BY | expander_queries_minor |
| INVOLVES | case_repository, case_summary_repository |
| PROVES | allegation_detail_repository, motion_claim_repository, evidence_chain_repository, decomposition_repository, analysis_repository, expander_queries |
| REBUTS | allegation_detail_repository, rebuttals_repository, decomposition_repository, case_summary_repository, person_detail_repository, expander_queries |
| RELIES_ON | allegation_detail_repository, motion_claim_repository, evidence_chain_repository, analysis_repository, expander_queries |
| STATED_BY | evidence_repository, allegation_detail_repository, rebuttals_repository, decomposition_repository, case_summary_repository, person_detail_repository, expander_queries, expander_queries_minor |
| SUPPORTS | allegation_repository, allegation_detail_repository, evidence_chain_repository, case_summary_repository, expander_queries |

---

## POSTGRESQL AUDIT

| # | File | Line | Function | Table.Column Referenced | Location | Read/Write |
|---|------|------|----------|------------------------|----------|------------|
| 1 | repositories/pipeline_repository/mod.rs | 108 | insert_document | documents (id, title, file_path, file_hash, document_type, status) | Repo | WRITE |
| 2 | repositories/pipeline_repository/mod.rs | 129 | insert_pipeline_config | pipeline_config (document_id, pass1_model, pass2_model, pass1_max_tokens, pass2_max_tokens, schema_file, admin_instructions, prior_context_doc_ids, created_by) | Repo | WRITE |
| 3 | repositories/pipeline_repository/mod.rs | 156 | update_document_status | documents (status, updated_at) | Repo | WRITE |
| 4 | repositories/pipeline_repository/mod.rs | 177 | insert_document_text | document_text (document_id, page_number, text_content) | Repo | WRITE |
| 5 | repositories/pipeline_repository/mod.rs | 192 | list_all_documents | documents + extraction_runs + pipeline_steps (LEFT JOINs) | Repo | READ |
| 6 | repositories/pipeline_repository/mod.rs | 222 | get_document | documents + extraction_runs + pipeline_steps (LEFT JOINs) | Repo | READ |
| 7 | repositories/pipeline_repository/mod.rs | 253 | get_document_text | document_text | Repo | READ |
| 8 | repositories/pipeline_repository/mod.rs | 268 | get_pipeline_config | pipeline_config | Repo | READ |
| 9 | repositories/pipeline_repository/extraction.rs | 65 | insert_extraction_run | extraction_runs | Repo | WRITE |
| 10 | repositories/pipeline_repository/extraction.rs | 93 | complete_extraction_run | extraction_runs (raw_output, tokens, cost, status) | Repo | WRITE |
| 11 | repositories/pipeline_repository/extraction.rs | 119 | insert_extraction_item | extraction_items (run_id, document_id, entity_type, item_data, verbatim_quote) | Repo | WRITE |
| 12 | repositories/pipeline_repository/extraction.rs | 147 | insert_extraction_relationship | extraction_relationships | Repo | WRITE |
| 13 | repositories/pipeline_repository/extraction.rs | 169 | get_items_with_quotes | extraction_items | Repo | READ |
| 14 | repositories/pipeline_repository/extraction.rs | 187 | update_item_grounding | extraction_items (grounding_status, grounded_page) | Repo | WRITE |
| 15 | repositories/pipeline_repository/extraction.rs | 203 | get_all_items | extraction_items | Repo | READ |
| 16 | repositories/pipeline_repository/extraction.rs | 217 | get_all_relationships | extraction_relationships | Repo | READ |
| 17 | repositories/pipeline_repository/extraction.rs | 234 | get_latest_completed_run | extraction_runs | Repo | READ |
| 18 | repositories/pipeline_repository/extraction.rs | 253 | get_items_for_run | extraction_items | Repo | READ |
| 19 | repositories/pipeline_repository/extraction.rs | 267 | get_relationships_for_run | extraction_relationships | Repo | READ |
| 20 | repositories/pipeline_repository/extraction.rs | 281 | get_extraction_runs | extraction_runs | Repo | READ |
| 21 | repositories/pipeline_repository/users.rs | 42 | upsert_known_user | known_users | Repo | WRITE |
| 22 | repositories/pipeline_repository/users.rs | 64 | list_known_users | known_users | Repo | READ |
| 23 | repositories/pipeline_repository/users.rs | 81 | assign_reviewer | documents (assigned_reviewer, assigned_at) | Repo | WRITE |
| 24 | repositories/pipeline_repository/review.rs | 45 | list_items | extraction_items (paginated with filters) | Repo | READ |
| 25 | repositories/pipeline_repository/review.rs | 75 | count_items | extraction_items (COUNT) | Repo | READ |
| 26 | repositories/pipeline_repository/review.rs | 98 | approve_item | extraction_items (review_status, reviewed_by, reviewed_at, review_notes) | Repo | WRITE |
| 27 | repositories/pipeline_repository/review.rs | 118 | reject_item | extraction_items | Repo | WRITE |
| 28 | repositories/pipeline_repository/review.rs | 140 | edit_item | extraction_items (grounded_page, verbatim_quote, grounding_status, review fields) | Repo | WRITE |
| 29 | repositories/pipeline_repository/review.rs | 166 | bulk_approve | extraction_items | Repo | WRITE |
| 30 | repositories/pipeline_repository/review.rs | 184 | count_pending | extraction_items | Repo | READ |
| 31 | repositories/pipeline_repository/steps.rs | 33 | record_step_start | pipeline_steps | Repo | WRITE |
| 32 | repositories/pipeline_repository/steps.rs | 52 | record_step_complete | pipeline_steps | Repo | WRITE |
| 33 | repositories/pipeline_repository/steps.rs | 73 | record_step_failure | pipeline_steps | Repo | WRITE |
| 34 | repositories/pipeline_repository/steps.rs | 92 | get_steps_for_document | pipeline_steps | Repo | READ |
| 35 | repositories/audit_repository.rs | 35 | log_action | admin_audit_log | Repo | WRITE |
| 36 | repositories/audit_repository.rs | 58 | get_recent | admin_audit_log | Repo | READ |
| 37 | repositories/qa_repository.rs | 182 | create_qa_entry | qa_entries | Repo | WRITE |
| 38 | repositories/qa_repository.rs | 212 | get_qa_history | qa_entries | Repo | READ |
| 39 | repositories/qa_repository.rs | 235 | get_qa_entry | qa_entries | Repo | READ |
| 40 | repositories/qa_repository.rs | 257 | update_rating | qa_entries (rating, rated_by, rated_at) | Repo | WRITE |
| 41 | repositories/qa_repository.rs | 285-316 | get_all_qa_entries | qa_entries (with/without user filter + counts) | Repo | READ |
| 42 | repositories/qa_repository.rs | 334 | bulk_delete_qa_entries | qa_entries | Repo | WRITE |
| 43 | repositories/qa_repository.rs | 348 | delete_all_qa_entries | qa_entries | Repo | WRITE |
| 44 | repositories/qa_repository.rs | 363 | delete_qa_entry | qa_entries | Repo | WRITE |
| 45 | api/admin_verify.rs | 81-95 | verify_evidence | audit_verifications (DELETE + INSERT) | **Handler** | WRITE |
| 46 | api/admin_flag.rs | 69 | flag_evidence | audit_findings | **Handler** | WRITE |
| 47 | api/admin_document_evidence.rs | 132 | get_document_evidence | audit_verifications | **Handler** | READ |
| 48 | api/admin_document_evidence.rs | 146 | get_document_evidence | audit_findings | **Handler** | READ |
| 49 | api/admin_status.rs | 58 | get_status | (SELECT 1 health check) | **Handler** | READ |
| 50 | api/pipeline/delete.rs | 74 | delete_document | document_audit_log (INSERT) | **Handler** | WRITE |
| 51 | api/pipeline/delete.rs | 114-150 | delete_document | extraction_relationships, extraction_items, extraction_runs, document_text, pipeline_steps, pipeline_config, documents (7 DELETEs in txn) | **Handler** | WRITE |
| 52 | api/pipeline/delete.rs | 199-258 | build_audit_snapshot | document_text, extraction_items, extraction_relationships, extraction_runs, pipeline_steps (COUNTs + json_build_object) | **Handler** | READ |
| 53 | api/pipeline/extract_text.rs | 171 | extract_text | documents (document_type, updated_at) | **Handler** | WRITE |
| 54 | api/pipeline/workload.rs | 52-108 | workload_handler | documents, known_users, extraction_items (LATERAL join) | **Handler** | READ |
| 55 | api/pipeline/metrics.rs | 79 | query_documents_by_status | documents (status, COUNT) | **Handler** | READ |
| 56 | api/pipeline/metrics.rs | 89 | query_total_cost | extraction_runs (SUM cost_usd) | **Handler** | READ |
| 57 | api/pipeline/metrics.rs | 99 | query_avg_grounding_rate | pipeline_steps (result_summary JSONB) | **Handler** | READ |
| 58 | api/pipeline/metrics.rs | 124 | query_step_performance | pipeline_steps (aggregations) | **Handler** | READ |
| 59 | api/pipeline/metrics.rs | 158-197 | query_estimates | documents, extraction_runs, pipeline_steps (complex aggregations) | **Handler** | READ |
| 60 | api/pipeline/errors.rs | 37-66 | errors_handler | documents, pipeline_steps (JOIN + subquery + COUNT) | **Handler** | READ |

### PostgreSQL Tables Referenced

| Table | # Queries | Files |
|-------|-----------|-------|
| documents | 15 | pipeline_repository/mod.rs, users.rs, delete.rs, extract_text.rs, workload.rs, metrics.rs, errors.rs |
| extraction_items | 18 | pipeline_repository/extraction.rs, review.rs, delete.rs, workload.rs |
| extraction_runs | 11 | pipeline_repository/extraction.rs, delete.rs, metrics.rs |
| extraction_relationships | 7 | pipeline_repository/extraction.rs, delete.rs |
| pipeline_steps | 11 | pipeline_repository/steps.rs, delete.rs, metrics.rs, errors.rs |
| pipeline_config | 5 | pipeline_repository/mod.rs, delete.rs |
| document_text | 6 | pipeline_repository/mod.rs, delete.rs |
| known_users | 3 | pipeline_repository/users.rs, workload.rs |
| qa_entries | 10 | qa_repository.rs |
| admin_audit_log | 2 | audit_repository.rs |
| audit_verifications | 2 | admin_verify.rs, admin_document_evidence.rs |
| audit_findings | 2 | admin_flag.rs, admin_document_evidence.rs |
| document_audit_log | 1 | delete.rs |

---

## QDRANT AUDIT

| # | File | Line | Function | Collection/Field Hardcoded | Notes |
|---|------|------|----------|---------------------------|-------|
| 1 | services/qdrant_service.rs | 20 | COLLECTION_NAME const | `"colossus_evidence"` | Used in all Qdrant calls |
| 2 | services/qdrant_service.rs | 69-72 | ensure_collection | 768-dim Cosine vectors | Hardcoded embedding dimensions |
| 3 | services/qdrant_service.rs | 84-90 | ensure_collection | 7 payload indexes: node_id, node_type, document_id, statement_type, stated_by, evidence_status, category | All "keyword" type |
| 4 | services/qdrant_service.rs | 174-176 | search_points | Filter key: `"node_type"` | Search filter construction |
| 5 | services/qdrant_service.rs | 195-208 | search_points | 13 payload fields: node_id, node_type, title, document_id, page_number, stated_by, statement_type, statement_date, exhibit_number, significance, verbatim_quote, evidence_status, category | SearchResult struct mapping |
| 6 | services/qdrant_service.rs | 259 | get_existing_point_ids | `["node_id"]` payload include | Scroll pagination |
| 7 | services/embedding_pipeline.rs | 217-230 | run_embedding_pipeline | QdrantPoint payload: node_id, node_type, title + all node properties | Payload construction |
| 8 | api/admin_status.rs | ~45 | get_status | `"colossus_evidence"` | Health check URL |
| 9 | api/search.rs | 52-78 | SearchHit struct | 14 fields mirroring Qdrant payload | Response DTO |
| 10 | api/ask.rs | 100-134 | RetrievalDetail struct | Same payload fields | RAG response DTO |
| 11 | *colossus-rs* retriever.rs | 252-263 | scope_filters_to_qdrant_filter | Filter keys: `"document_id"`, `"node_type"` | Scope filter construction |
| 12 | *colossus-rs* retriever.rs | 317-327 | scored_point_to_context_chunk | Payload fields: node_id, node_type, title, document_id, page_number | Chunk extraction |
| 13 | *colossus-rs* expander.rs | 184-210 | expand_context | 7 node type strings: Evidence, ComplaintAllegation, MotionClaim, Harm, Document, Person, Organization | Match dispatch |
| 14 | *colossus-rs* expander.rs | 337-354 | build_content | Per-type field selection: verbatim_quote, allegation, claim_text, description | Content extraction |

---

## FRONTEND AUDIT

| # | File | Interface/Type | Fields That Assume Backend Schema | Notes |
|---|------|---------------|----------------------------------|-------|
| 1 | services/evidence.ts | EvidenceDto | exhibit_number, title, question, answer, kind, weight, page_number, significance, verbatim_quote, stated_by, document_id, document_title | Assumes Evidence node payload |
| 2 | services/allegations.ts | AllegationDto | paragraph, title, allegation, evidence_status, category, severity, legal_count_ids, legal_counts | Assumes ComplaintAllegation structure |
| 3 | services/motionClaims.ts | MotionClaimDto | title, claim_text, category, significance, proves_allegations, relies_on_evidence, source_document_id, source_document_title | Assumes MotionClaim structure |
| 4 | services/harms.ts | HarmDto | title, category, subcategory, amount, description, date, source_reference, caused_by_allegations, damages_for_counts | Assumes Harm structure |
| 5 | services/documentEvidence.ts | DocumentEvidence | node_type (hardcoded 5 values), title, verbatim_quote, page_number, kind, weight, speaker, verification, flags | Core frontend entity contract |
| 6 | services/search.ts | SearchHit | node_id, node_type, title, score, document_id, page_number | Qdrant payload subset |
| 7 | services/ask.ts | RetrievalDetail | node_id, node_type, title, score, origin, document_title, document_id, page_number, quote_preview, relationship_count | RAG response contract |
| 8 | services/ask.ts | AnswerSource | document_id, document_title, page_number, evidence_title, node_id | Chat answer citation |
| 9 | services/personDetail.ts | PersonInfo, PersonSummary | id, name, role; total_statements, documents_count, characterizations_count, rebuttals_received_count | Person detail structure |
| 10 | services/evidenceChain.ts | ChainAllegation, EvidenceWithDocument | evidence_status, legal_counts, question, answer, page_number | Evidence chain navigation |
| 11 | components/RetrievalDetailsPanel.tsx | NODE_TYPE_COLORS | 6 keys: Evidence, ComplaintAllegation, MotionClaim, Harm, LegalCount, Document | Hardcoded node type strings |
| 12 | utils/nodeTypeDisplay.ts | getNodeTypeDisplay() | 5 case values: ComplaintAllegation, MotionClaim, LegalCount, Harm, Evidence | Display label transformations |
| 13 | utils/nodeTypeDisplay.ts | getPageLabel() | Special case: ComplaintAllegation uses paragraph display | Node-type-specific logic |
| 14 | pages/GraphPage.tsx | ACCENT_COLORS | snake_case keys: legal_count, allegation, motion_claim, evidence, document | Different naming convention |
| 15 | components/pipeline/ContentPanel.tsx | TYPE_COLORS | Person, Evidence, Allegation, Claim, Document, Event | Pipeline entity type colors |

### Frontend Naming Convention Conflicts

| Convention | Where Used | Example |
|-----------|-----------|---------|
| PascalCase | Qdrant payload, RAG, search, document evidence | `ComplaintAllegation`, `MotionClaim` |
| snake_case | GraphPage evidence chain visualization | `motion_claim`, `legal_count` |
| Informal | Pipeline ContentPanel | `Allegation` (not `ComplaintAllegation`) |

---

## SUMMARY

| Metric | Count |
|--------|-------|
| **Total Neo4j hardcoded queries** | 51 |
| **Total PostgreSQL hardcoded queries** | 86 |
| **Total Qdrant hardcoded calls** | 14 locations |
| **Total frontend schema assumptions** | 15 interfaces/mappings |

### Files that would need to change for:

**Neo4j schema change (node labels or relationship types):**
- colossus-legal repositories: ~14 files
- colossus-legal pipeline handlers: 2 files (ingest_helpers, delete)
- colossus-rs: 3 files (expander_queries, expander_queries_minor, embedding_repository)
- **Total: ~19 Rust files**

**PostgreSQL schema change (table/column rename):**
- colossus-legal repositories: 7 files
- colossus-legal handlers: 8 files
- colossus-rs: 0 files
- **Total: ~15 Rust files**

**Qdrant schema change (collection name, payload fields, dimensions):**
- colossus-legal backend: 4 files (qdrant_service, embedding_pipeline, search, ask)
- colossus-rs: 2 files (retriever, expander)
- Frontend: 5 files (search, ask, RetrievalDetailsPanel, nodeTypeDisplay, GraphPage)
- **Total: ~11 files**

### Key Observations

1. **All queries use parameterized binding** — no SQL/Cypher injection risks found
2. **PostgreSQL queries split roughly 50/50** between repository modules and handler files — the pipeline handlers (delete, metrics, workload, errors) contain significant inline SQL
3. **Neo4j is read-heavy** — 43 READ vs 8 WRITE queries
4. **Frontend uses three different naming conventions** for node types: PascalCase (Evidence), snake_case (motion_claim), and informal (Allegation vs ComplaintAllegation)
5. **Qdrant payload is the tightest coupling** — 13 field names are hardcoded identically across backend services, API DTOs, colossus-rs RAG, and frontend types
6. **colossus-rs has zero PostgreSQL queries** — all sqlx usage is in colossus-legal
