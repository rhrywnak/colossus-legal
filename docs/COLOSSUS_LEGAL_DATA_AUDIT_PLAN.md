# COLOSSUS_LEGAL_DATA_AUDIT_PLAN.md

**Purpose:** Systematic end-to-end verification of data integrity across the entire pipeline — from source PDFs through Neo4j and Qdrant to web display.

**Why now:** We're about to add 3 new documents. Before adding more data to a system whose integrity is unverified, we need to know what we have and where the gaps are.

**Time estimate:** 2-3 hours (mostly query execution and comparison)

---

## Audit Scope

```
Source PDFs on disk
  ↓ verified against ↓
Document nodes in Neo4j (file_path, title, date)
  ↓ verified against ↓
Evidence nodes in Neo4j (verbatim_quote, page_number, STATED_BY, CONTAINED_IN)
  ↓ verified against ↓
Claims JSON files (did every extracted claim get imported?)
  ↓ verified against ↓
Qdrant vectors (does every Evidence node have a vector?)
  ↓ verified against ↓
Web app display (Documents page, Evidence Explorer, Chat answers)
```

---

## Phase 1: PDF ↔ Document Node Verification

**Goal:** Every PDF on disk has a Document node. Every Document node has a PDF on disk.

### 1A. List all PDFs on DEV host

```bash
ssh root@pve-2 "ls -1 /dev-zfs/legal-docs/*.pdf" > /tmp/pdfs_on_disk.txt
cat /tmp/pdfs_on_disk.txt
```

### 1B. List all Document nodes in Neo4j

Run in Neo4j Browser (DEV):

```cypher
MATCH (d:Document)
RETURN d.id, d.title, d.file_path, d.document_type, d.date
ORDER BY d.id
```

### 1C. Cross-reference

For each Document node:
- [ ] Does `file_path` point to a file that exists on disk?
- [ ] Is the file_path property set (not null)?
- [ ] Does the `/documents/{id}/file` URL serve the PDF?

For each PDF on disk:
- [ ] Does a Document node exist for it?
- [ ] If not, is it a new document that needs intake, or an orphaned file?

### Expected findings template

```
| Document ID | file_path | On disk? | URL works? | Issue |
|-------------|-----------|----------|------------|-------|
| doc-awad-complaint | Awad_v_... | ✅/❌ | ✅/❌ | |
```

---

## Phase 2: Evidence Node Completeness

**Goal:** Every Evidence node has the minimum required properties and relationships.

### 2A. Find Evidence nodes missing required properties

```cypher
// Evidence without verbatim_quote
MATCH (e:Evidence) WHERE e.verbatim_quote IS NULL OR e.verbatim_quote = ""
MATCH (e)-[:CONTAINED_IN]->(d:Document)
RETURN e.id, e.title, d.title AS document, e.page_number
ORDER BY d.title, e.id
```

```cypher
// Evidence without page_number
MATCH (e:Evidence) WHERE e.page_number IS NULL
MATCH (e)-[:CONTAINED_IN]->(d:Document)
RETURN e.id, e.title, d.title AS document
ORDER BY d.title, e.id
```

### 2B. Find Evidence nodes missing required relationships

```cypher
// Evidence without CONTAINED_IN (orphaned — no source document)
MATCH (e:Evidence) WHERE NOT (e)-[:CONTAINED_IN]->(:Document)
RETURN e.id, e.title
```

```cypher
// Evidence without STATED_BY (no speaker attribution)
MATCH (e:Evidence) WHERE NOT (e)-[:STATED_BY]->()
RETURN e.id, e.title
ORDER BY e.id
```

### 2C. Evidence counts by document

```cypher
// How many evidence nodes per document?
MATCH (e:Evidence)-[:CONTAINED_IN]->(d:Document)
RETURN d.id, d.title, count(e) AS evidence_count
ORDER BY evidence_count DESC
```

### 2D. Evidence counts by completeness status

```cypher
// Summary: how many are fully grounded?
MATCH (e:Evidence)
OPTIONAL MATCH (e)-[:CONTAINED_IN]->(d:Document)
OPTIONAL MATCH (e)-[:STATED_BY]->(s)
RETURN 
  count(e) AS total_evidence,
  count(CASE WHEN e.verbatim_quote IS NOT NULL AND e.verbatim_quote <> "" THEN 1 END) AS has_quote,
  count(CASE WHEN e.page_number IS NOT NULL THEN 1 END) AS has_page,
  count(CASE WHEN d IS NOT NULL THEN 1 END) AS has_document,
  count(CASE WHEN s IS NOT NULL THEN 1 END) AS has_speaker
```

### Expected findings template

```
| Metric | Count | Percentage | Target |
|--------|-------|------------|--------|
| Total Evidence nodes | ? | 100% | — |
| Has verbatim_quote | ? | ?% | 100% |
| Has page_number | ? | ?% | 100% |
| Has CONTAINED_IN | ? | ?% | 100% |
| Has STATED_BY | ? | ?% | 100% |
```

---

## Phase 3: Claims JSON ↔ Neo4j Reconciliation

**Goal:** Every claim extracted in the JSON files was imported into Neo4j. No claims were lost during import.

### 3A. Inventory of claims JSON files

List all claims JSON files in the project:

| JSON File | Source Document | Claims in JSON | Expected node type |
|-----------|---------------|----------------|-------------------|
| Awad_Complaint_claims.json | Awad Complaint | 18 | ComplaintAllegation |
| Phillips_Discovery_Response_claims.json | Phillips Discovery | 52 | Evidence |
| CFS_Interrogatory_Response_claims.json | CFS Interrogatory | 45 | Evidence |
| Phillips_Motion_for_Default_claims.json | Phillips Motion Default | 58 | MotionClaim |
| CFS_Motion_for_Default_claims.json | CFS Motion Default | 52 | MotionClaim |
| Phillips_COA_Response_claims.json | Phillips CoA Response | 12 | Evidence |

### 3B. Count nodes per document and compare

For each JSON file, compare the claim count to the actual Neo4j node count:

```cypher
// Count ComplaintAllegation nodes
MATCH (a:ComplaintAllegation) RETURN count(a) AS complaint_allegations
// EXPECT: 18 (from Awad_Complaint_claims.json)
```

```cypher
// Count Evidence nodes from Phillips Discovery
MATCH (e:Evidence)-[:CONTAINED_IN]->(d:Document {id: "doc-phillips-discovery-response"})
RETURN count(e) AS phillips_discovery_evidence
// EXPECT: Should match claims extracted (not all 52 may have been imported as Evidence)
```

```cypher
// Count MotionClaim nodes per document
MATCH (m:MotionClaim)-[:APPEARS_IN]->(d:Document)
RETURN d.id, d.title, count(m) AS claim_count
ORDER BY d.id
```

### 3C. Reconciliation template

```
| JSON File | Claims in JSON | Nodes in Neo4j | Delta | Notes |
|-----------|---------------|----------------|-------|-------|
| Awad_Complaint_claims.json | 18 | ? | | |
| Phillips_Discovery_Response_claims.json | 52 | ? | | Not all claims become Evidence |
| ... | | | | |
```

**Important:** Not every claim in a JSON necessarily becomes a Neo4j node. Some claims may have been filtered during review (Step 5 of the intake process). The audit identifies gaps — Roman decides which gaps are intentional vs missed.

---

## Phase 4: Neo4j ↔ Qdrant Reconciliation

**Goal:** Every Evidence node in Neo4j has a corresponding vector in Qdrant.

### 4A. Count Evidence nodes in Neo4j

```cypher
MATCH (e:Evidence) RETURN count(e) AS neo4j_evidence_count
```

### 4B. Count points in Qdrant

```bash
# DEV
curl -s http://10.10.100.200:6333/collections/colossus_evidence | jq '.result.points_count'

# PROD
curl -s http://10.10.100.110:6333/collections/colossus_evidence | jq '.result.points_count'
```

### 4C. Compare

```
| Source | Count | Match? |
|--------|-------|--------|
| Neo4j Evidence nodes | ? | |
| Qdrant colossus_evidence points | ? | |
| Delta | ? | Should be 0 |
```

If the delta is non-zero, evidence was added to Neo4j without re-running the Qdrant sync. Fix: run the Qdrant sync playbook via Semaphore.

### 4D. Spot-check vector content

Pick 3 random Evidence nodes and verify their content is in Qdrant:

```bash
# Search Qdrant for a known evidence title
curl -s -X POST http://10.10.100.200:6333/collections/colossus_evidence/points/scroll \
  -H "Content-Type: application/json" \
  -d '{"filter":{"must":[{"key":"node_id","match":{"value":"evidence-phillips-q74"}}]},"limit":1}' | jq '.result.points[0].payload.title'
```

---

## Phase 5: Relationship Integrity

**Goal:** Key relationships are consistent and complete.

### 5A. CONTRADICTS pairs — both sides have STATED_BY to same speaker

```cypher
// CONTRADICTS where speakers don't match (invalid)
MATCH (e1:Evidence)-[:CONTRADICTS]->(e2:Evidence)
OPTIONAL MATCH (e1)-[:STATED_BY]->(s1)
OPTIONAL MATCH (e2)-[:STATED_BY]->(s2)
WHERE s1.id <> s2.id OR s1 IS NULL OR s2 IS NULL
RETURN e1.id, s1.id AS speaker1, e2.id, s2.id AS speaker2
```

Should return 0 rows. Any results are data quality issues.

### 5B. Proof chain completeness

```cypher
// Allegations without any PROVES relationship (no evidence proving them)
MATCH (a:ComplaintAllegation)
WHERE NOT ()-[:PROVES]->(a)
RETURN a.id, a.allegation
```

### 5C. Orphaned nodes (no relationships at all)

```cypher
MATCH (n) WHERE NOT (n)--() AND NOT n:Case
RETURN labels(n) AS type, n.id, n.title
```

### 5D. REBUTS integrity — different speakers

```cypher
// REBUTS where speakers are the same (should be CONTRADICTS instead)
MATCH (e1:Evidence)-[:REBUTS]->(e2:Evidence)
OPTIONAL MATCH (e1)-[:STATED_BY]->(s1)
OPTIONAL MATCH (e2)-[:STATED_BY]->(s2)
WHERE s1.id = s2.id
RETURN e1.id, e2.id, s1.id AS same_speaker
```

Should return 0 rows.

---

## Phase 6: Web App Display Verification

**Goal:** The web app correctly displays what's in the database.

### 6A. Documents page

- [ ] Count of documents on the page matches Document node count in Neo4j
- [ ] Every document with a `file_path` shows "View PDF" (not "Document not available")
- [ ] Every "View PDF" link actually opens the correct PDF

### 6B. Evidence Explorer

- [ ] Evidence count on the page matches Evidence node count in Neo4j
- [ ] Each evidence item shows source document and page number (where available)
- [ ] Clicking a source link opens the PDF at the correct page

### 6C. Contradictions page

- [ ] Contradiction count matches CONTRADICTS relationship count in Neo4j
- [ ] Each contradiction shows both statements with sources

### 6D. Chat/Ask

Test these questions and verify answers cite evidence correctly:

1. "What did Phillips state about the $50,000?" — should cite Phillips Discovery evidence
2. "What did CFS admit about the children?" — should cite CFS Admissions evidence (once imported)
3. "Where did Phillips contradict himself?" — should show CONTRADICTS pairs with sources

---

## Phase 7: DEV ↔ PROD Consistency

**Goal:** PROD has the same data as DEV (or documented differences).

### 7A. Node counts comparison

Run on both DEV and PROD:

```cypher
MATCH (n) RETURN labels(n)[0] AS type, count(n) AS count ORDER BY type
```

### 7B. Known differences

Document expected differences:

```
| Item | DEV | PROD | Reason |
|------|-----|------|--------|
| QAEntry nodes in Neo4j | 0 | 5 | PROD cleanup not yet done |
| ... | | | |
```

### 7C. Qdrant sync status

```
| Environment | Collection | Points | Last synced |
|-------------|-----------|--------|-------------|
| DEV | colossus_evidence | ? | ? |
| PROD | colossus_evidence | ? | ? |
```

---

## Audit Output

After running all phases, produce:

1. **AUDIT_FINDINGS.md** — a document listing every issue found, categorized as:
   - CRITICAL: Data is wrong or missing in a way that affects answers
   - HIGH: Data is incomplete but partially functional
   - LOW: Cosmetic or non-blocking issues

2. **Remediation plan** — for each finding, what needs to be done (fix data, re-sync, add missing quotes, etc.)

3. **Updated COLOSSUS_LEGAL_DOC_INDEX** — reflecting actual verified state, not assumed state

---

## Audit Principles

1. **Query, don't assume.** Every number in this audit comes from an actual query result, not from memory or transition docs.
2. **Record raw results.** Paste actual query output, not summaries. If the count is wrong later, we can trace back.
3. **Fix forward, don't cover up.** If a claim was missed during import, add it. Don't pretend it was intentional.
4. **Document intentional gaps.** If a claim was deliberately excluded (e.g., too vague, duplicate), note that explicitly so future audits don't re-flag it.
