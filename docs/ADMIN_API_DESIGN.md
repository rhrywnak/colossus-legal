# Admin API & Tooling Design — Colossus-Legal v0.9.0

**Date:** 2026-03-23
**Author:** Roman + Opus
**Branch:** `feature/admin-api` (create from main at v0.8.9)
**Repos affected:** colossus-legal (primary), colossus-rag (none), colossus-ansible (minor)

---

## Overview

Four improvements inspired by Cognee's automated ingestion pipeline, adapted to our
human-in-the-loop litigation support model:

| Item | What | Why |
|------|------|-----|
| A | Incremental Qdrant indexing | Stop full-rebuilding 203+ vectors when adding 3 new evidence nodes |
| B | Content hashing on Document nodes | Prevent duplicate document ingestion |
| C | Admin API endpoints | Replace manual Cypher with validated HTTP endpoints |
| D | Admin frontend page | UI for document intake, reindex, chat management — admin-group gated |

---

## A. Incremental Qdrant Indexing

### Current state
- `backend/src/services/embedding_pipeline.rs` fetches ALL Evidence + ComplaintAllegation nodes from Neo4j, embeds them, upserts to Qdrant
- CLI: `./colossus-legal-backend embed [--clean]`
- Ansible: `playbooks/reindex-qdrant.yml` runs the CLI
- `--clean` deletes the collection first; without it, upserts overwrite existing points
- Current count: 203 points in `colossus_evidence`

### Change
Add an `--incremental` mode (make it the default):

1. Query Qdrant for all existing point IDs via scroll API
2. Query Neo4j for all Evidence + ComplaintAllegation node IDs
3. Compute the delta: `neo4j_ids - qdrant_ids`
4. Embed and upsert only the delta
5. Report: `embedded: N new, skipped: M existing, total: N+M`

CLI flags become:
- `embed` (no flags) → incremental (default)
- `embed --clean` → full rebuild (existing behavior)
- `embed --dry-run` → show what would be indexed without doing it

### Implementation notes
- Qdrant scroll API: `POST /collections/{name}/points/scroll` with `limit: 1000`, `with_payload: false`, `with_vector: false` — returns just IDs
- Point IDs in Qdrant are the Neo4j node IDs (strings stored as Qdrant point IDs)
- Use `HashSet<String>` for the diff — O(1) lookup
- The embedding model + Qdrant client are already constructed in the pipeline; this is purely a filter step before the embed loop

### CC instruction file: `CC_INCREMENTAL_INDEX.md`

---

## B. Content Hashing

### Change
Add `content_hash: String` property (SHA-256 hex) to Document nodes in Neo4j.

### Implementation
This is a runbook/Cypher change, not a code change:

1. When creating a new Document node (Step 3 of DOCUMENT_INTAKE_RUNBOOK), compute SHA-256 of the PDF:
   ```bash
   sha256sum /path/to/DOCUMENT.pdf | awk '{print $1}'
   ```

2. Add to the CREATE Cypher:
   ```cypher
   CREATE (d:Document {
     id: "doc-CHANGEME",
     ...,
     content_hash: "abc123..."
   })
   ```

3. Before CREATE, check for duplicates:
   ```cypher
   MATCH (d:Document {content_hash: $hash})
   RETURN d.id, d.title
   // EXPECT: 0 rows (if >0, document already exists)
   ```

4. Backfill existing documents (one-time):
   ```bash
   # On each host, compute hashes for all PDFs
   for f in /dev-zfs/legal-docs/*.pdf; do
     echo "$(sha256sum "$f" | awk '{print $1}')  $(basename "$f")"
   done
   ```
   Then update existing Document nodes with the computed hashes.

### Admin API integration (Item C)
The `POST /api/admin/documents` endpoint will compute the hash server-side from the file on disk
and reject duplicates automatically.

---

## C. Admin API Endpoints

### Design principles
- All admin endpoints require `require_admin(&user)?` guard
- All endpoints under `/api/admin/` prefix
- JSON request/response with proper error types
- Neo4j operations via existing `neo4rs::Graph` in AppState
- Endpoints replace manual Cypher execution, NOT the human review step

### Endpoints

#### C.1 `POST /api/admin/documents` — Register a new document

Creates a Document node in Neo4j after validating the PDF exists on disk.

**Request:**
```json
{
  "id": "doc-ssa-form-1724",
  "title": "SSA Form 1724 — Representative Payee Report",
  "document_type": "form",
  "date": "2012-10-01",
  "author": "catholic-family-service",
  "case_number": "09-47102-DE",
  "page_count": 1,
  "file_path": "SSA_Form_1724.pdf"
}
```

**Validation:**
1. Check PDF exists at `{document_storage_path}/{file_path}` — 400 if not found
2. Compute SHA-256 of the PDF file
3. Check for existing Document node with same `id` — 409 Conflict if exists
4. Check for existing Document node with same `content_hash` — 409 if duplicate
5. Create Document node with all properties + `content_hash`

**Response (201):**
```json
{
  "id": "doc-ssa-form-1724",
  "title": "SSA Form 1724 — Representative Payee Report",
  "content_hash": "a1b2c3...",
  "pdf_url": "/documents/doc-ssa-form-1724/file"
}
```

**File:** `backend/src/api/admin_documents.rs` (~150 lines)

#### C.2 `GET /api/admin/documents` — List all documents with status

Returns all Document nodes with evidence count and indexing status.

**Response:**
```json
{
  "documents": [
    {
      "id": "doc-awad-complaint",
      "title": "Awad Complaint",
      "document_type": "complaint",
      "date": "2009-11-01",
      "evidence_count": 18,
      "has_pdf": true,
      "content_hash": "abc..."
    }
  ],
  "total": 15
}
```

**Cypher:**
```cypher
MATCH (d:Document)
OPTIONAL MATCH (e:Evidence)-[:CONTAINED_IN]->(d)
RETURN d, count(e) AS evidence_count
ORDER BY d.date
```

#### C.3 `POST /api/admin/evidence` — Import reviewed evidence

Accepts a reviewed JSON extraction (the output of Step 4/5 from the runbook) and creates
Evidence nodes + relationships in a single Neo4j transaction.

**Request:**
```json
{
  "document_id": "doc-ssa-form-1724",
  "evidence": [
    {
      "id": "evidence-ssa1724-no-children",
      "title": "CFS claimed no living children on SSA Form",
      "content": "CFS reported to Social Security Administration that the ward had no living children...",
      "verbatim_quote": "Does the beneficiary have a living spouse or child? No",
      "page_number": 1,
      "date": "2012-10-01",
      "topic": "misrepresentation",
      "stated_by": "catholic-family-service",
      "about": ["marie-awad"],
      "supports_counts": ["count-fraud", "count-breach-fiduciary-duty"],
      "contradicts": [
        {
          "evidence_id": "evidence-cfs-q14",
          "topic": "surviving children",
          "value": "none vs three"
        }
      ],
      "proves_allegations": ["allegation-005"]
    }
  ]
}
```

**Validation:**
1. Verify `document_id` exists as a Document node — 404 if not
2. Verify each `stated_by` exists as a Person or Organization node — 400 if not
3. Verify each `about` person exists — 400 if not
4. Verify each `supports_counts` exists as a LegalCount — 400 if not
5. Verify each `contradicts.evidence_id` exists — 400 if not
6. Verify each `proves_allegations` exists — 400 if not
7. Check for duplicate evidence IDs — 409 if any exist

**Transaction:** All nodes and relationships created in a single Neo4j transaction.
If any step fails, entire transaction rolls back.

**Response (201):**
```json
{
  "created": 1,
  "relationships": {
    "contained_in": 1,
    "stated_by": 1,
    "about": 1,
    "supports": 2,
    "contradicts": 1,
    "proves": 1
  }
}
```

**File:** `backend/src/api/admin_evidence.rs` (~250 lines)

#### C.4 `POST /api/admin/reindex` — Trigger incremental Qdrant reindex

Triggers the embedding pipeline from within the running backend (no CLI needed).

**Request:**
```json
{
  "mode": "incremental"  // or "full"
}
```

**Response (200):**
```json
{
  "mode": "incremental",
  "new_points": 3,
  "skipped": 203,
  "total": 206,
  "duration_ms": 4500
}
```

**Note:** This runs the same logic as the CLI `embed` subcommand but as an HTTP handler.
The embedding pipeline is already in-process (fastembed model is loaded at startup).

**File:** `backend/src/api/admin_reindex.rs` (~80 lines)

#### C.5 `GET /api/admin/qa-entries` — List all chat entries (admin view)

Returns all QA entries across all users, with filtering.

**Query params:** `?user=roman&limit=50&offset=0`

**Response:** Same as `/api/qa-history` but without user filtering, plus a `asked_by` field.

#### C.6 `DELETE /api/admin/qa-entries/:id` — Delete any chat entry

Already exists as `DELETE /api/qa/:id` with admin guard. No change needed — just wire it
into the admin frontend.

#### C.7 `DELETE /api/admin/qa-entries` — Bulk delete chat entries

**Request:**
```json
{
  "ids": ["uuid-1", "uuid-2", "uuid-3"]
}
```

Or: `{"all": true}` to clear all entries (with confirmation).

**File:** `backend/src/api/admin_qa.rs` (~100 lines)

### Route registration

In `main.rs`, add an admin route group:

```rust
// Admin routes — require admin group
.route("/api/admin/documents", post(admin_documents::create_document))
.route("/api/admin/documents", get(admin_documents::list_documents))
.route("/api/admin/evidence", post(admin_evidence::import_evidence))
.route("/api/admin/reindex", post(admin_reindex::trigger_reindex))
.route("/api/admin/qa-entries", get(admin_qa::list_all_entries))
.route("/api/admin/qa-entries", delete(admin_qa::bulk_delete_entries))
```

---

## D. Admin Frontend Page

### Access control
- Route: `/admin`
- Gated by `is_admin` from `GET /api/me` response
- Non-admin users see no "Admin" link in navigation and get redirected if they hit `/admin` directly
- Nav bar shows "Admin" link only when `permissions.is_admin === true`

### Page layout — three panels

#### D.1 Document Management Panel
- Table of all documents (from `GET /api/admin/documents`)
- Columns: Title, Type, Date, Evidence Count, PDF Status, Hash
- "Register Document" button → form matching `POST /api/admin/documents` fields
- "Import Evidence" button → JSON textarea or file upload matching `POST /api/admin/evidence` format
- Status indicators: ✅ has PDF, ⚠️ no evidence, 🔍 not indexed

#### D.2 Index Management Panel  
- Current Qdrant stats: point count, collection name
- "Reindex (Incremental)" button → calls `POST /api/admin/reindex`
- "Reindex (Full)" button → calls with `mode: "full"`, confirm dialog
- Shows last reindex result (points added, duration)

#### D.3 Chat Management Panel
- Table of all QA entries (from `GET /api/admin/qa-entries`)
- Columns: Question (truncated), User, Date, Rating, Model
- Checkbox selection for bulk delete
- "Delete Selected" button with confirmation
- "Clear All" button with double confirmation
- Filter by user dropdown

### File: `frontend/src/pages/Admin.tsx` (~280 lines, near the 300-line limit)
May need to extract sub-components if it gets large:
- `AdminDocuments.tsx`
- `AdminIndex.tsx`  
- `AdminChats.tsx`

---

## Implementation Order

| Step | Item | CC Instruction | Est. effort |
|------|------|---------------|-------------|
| 1 | Incremental indexing | CC_INCREMENTAL_INDEX.md | 1 CC session |
| 2 | Admin document endpoint | CC_ADMIN_DOCUMENTS.md | 1 CC session |
| 3 | Admin evidence endpoint | CC_ADMIN_EVIDENCE.md | 1 CC session |
| 4 | Admin reindex endpoint | CC_ADMIN_REINDEX.md | 0.5 CC session |
| 5 | Admin QA endpoints | CC_ADMIN_QA.md | 0.5 CC session |
| 6 | Admin frontend page | CC_ADMIN_FRONTEND.md | 1-2 CC sessions |
| 7 | Content hash backfill | Manual (runbook update + Cypher) | 15 min |

Steps 1-5 are backend, step 6 is frontend, step 7 is operational.

---

## Version plan

- v0.9.0 — incremental indexing + admin API endpoints + admin frontend page
- Create branch `feature/admin-api` from main
- Bump version early, build + deploy after each major milestone

---

## Deployment impact

### New env vars: None
- Document storage path already in config (`DOCUMENT_STORAGE_PATH`)
- Neo4j, Qdrant, PostgreSQL connections already in AppState
- Embedding model already loaded at startup

### Ansible changes: None for this work
- Existing reindex playbook still works (CLI unchanged, just gains default incremental)

### Traefik/Auth changes: None
- `/api/admin/*` routes go through the same Authentik ForwardAuth as all other `/api/*` routes
- Admin guard is application-level, not infrastructure-level

---

## Rust Learning Opportunities

- **SHA-256 hashing**: Using the `sha2` crate with `Digest` trait — similar to how fastembed uses trait-based APIs
- **Neo4j transactions**: `graph.run()` vs `graph.execute()` vs explicit transactions with `graph.start_txn()`
- **Axum route groups**: How to organize related routes without middleware nesting
- **Serde flatten**: Using `#[serde(flatten)]` for request structs that share common fields
- **HashSet operations**: Set difference for the incremental indexing diff

---

*End of design document*
