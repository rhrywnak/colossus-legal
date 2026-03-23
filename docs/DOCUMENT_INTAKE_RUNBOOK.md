# DOCUMENT_INTAKE_RUNBOOK.md — Adding New Documents to Colossus-Legal

**Purpose:** Step-by-step process for adding a new PDF document to the system so it appears in the web app, is searchable via Chat, and is linked in the knowledge graph.

**Who:** Roman (with Claude Opus for extraction and Cypher generation)

**Time estimate:** 30-90 minutes per document depending on complexity

---

## Overview

```
PDF on workstation
  │
  ├─ Step 1: Name and register the document
  ├─ Step 2: Copy PDF to DEV and PROD hosts
  ├─ Step 3: Create Document node in Neo4j
  ├─ Step 4: Extract claims/evidence with Claude
  ├─ Step 5: Review and approve extraction
  ├─ Step 6: Import evidence into Neo4j
  ├─ Step 7: Index evidence in Qdrant
  ├─ Step 8: Add decomposer alias (if needed)
  └─ Step 9: Verify in web app
```

---

## Step 1: Name and Register the Document

### Choose a document ID

Follow the existing pattern: `doc-{author/source}-{type}-{identifier}`

Examples from existing documents:
- `doc-awad-complaint`
- `doc-phillips-discovery-response`
- `doc-penzien-coa-brief-300891`
- `doc-coa-ruling-011212`

### Choose a PDF filename

The filename on disk must be simple and consistent. Existing patterns:
- `GEORGE_PHILLIPS_RESPONSE_TO_DISCOVERY.pdf`
- `court_of_appeals_ruling_01122012.pdf`
- `Awad_v_Catholic_Family_Complaint_11113.pdf`

### Record in the doc index

Add the document to `COLOSSUS_LEGAL_DOC_INDEX_V030.md` (or the current version) with:
- Document number (next sequential)
- Title, date filed, page count
- Pipeline stage: STAGE 1 (Uploaded)

---

## Step 2: Copy PDF to DEV and PROD Hosts

The PDF must be in the virtiofs-mounted directory on both Proxmox hosts.

```bash
# Copy to DEV
scp /path/to/DOCUMENT.pdf root@pve-2:/dev-zfs/legal-docs/

# Copy to PROD
scp /path/to/DOCUMENT.pdf root@pve-1:/prod-zfs/legal-docs/

# Verify
ssh root@pve-2 "ls -la /dev-zfs/legal-docs/DOCUMENT.pdf"
ssh root@pve-1 "ls -la /prod-zfs/legal-docs/DOCUMENT.pdf"
```

The backend serves PDFs from `/data/documents/` inside the container (mapped to the ZFS dataset). The URL pattern is:

```
https://colossus-legal-api-dev.cogmai.com/documents/{document_id}/file
```

**NOTE:** This endpoint has NO `/api` prefix — it's `/documents/{id}/file`, not `/api/documents/{id}/file`. The backend uses the `file_path` property on the Document node to locate the PDF on disk.

---

## Step 3: Create Document Node in Neo4j

Run this Cypher in Neo4j Browser (DEV first, then PROD after verification):

```cypher
// Create Document node — adjust all values
CREATE (d:Document {
  id: "doc-CHANGEME",
  title: "CHANGEME — Human-readable title",
  document_type: "brief",          // brief | discovery_response | affidavit | court_order | motion | ruling | form
  date: "YYYY-MM-DD",              // filing date, ISO format
  author: "CHANGEME",              // person or organization who authored it
  case_number: "09-47102-DE",      // or "11-4113-NZ" for circuit court docs
  page_count: 0,                   // total pages
  file_path: "CHANGEME.pdf"        // exact PDF filename on disk (used by /documents/:id/file endpoint)
})
RETURN d.id, d.title
// EXPECT: 1 row
```

**Verify the document is accessible in the app:**
```
https://colossus-legal-api-dev.cogmai.com/documents/doc-CHANGEME/file
```

If this returns the PDF, you're good. If it returns `{"error":"not_found","message":"Document has no associated file"}`, check that the `file_path` property matches the exact filename on disk in `/dev-zfs/legal-docs/` (case-sensitive).

---

## Step 4: Extract Claims/Evidence with Claude

This is where you use Claude (this chat or a new conversation) to extract structured data from the document.

### For legal briefs, motions, and filings:

Provide the PDF to Claude and ask:

```
Extract the key factual claims from this document. For each claim, provide:
1. A unique ID (format: evidence-{source}-{topic}-{number})
2. The verbatim quote from the document
3. The page number
4. Who stated it (person ID, lowercase hyphenated, e.g., "george-phillips")
5. The topic/category
6. Any documents or exhibits referenced

Return as JSON array matching this structure:
{
  "document_id": "doc-CHANGEME",
  "evidence": [
    {
      "id": "evidence-CHANGEME-001",
      "title": "Short description",
      "content": "Longer description for search",
      "verbatim_quote": "Exact quote from document",
      "page_number": 1,
      "stated_by": "person-id",
      "topic": "category",
      "referenced_exhibits": ["Exhibit 1 — description"]
    }
  ]
}
```

### For government forms or simple documents:

These may not need full claim extraction. A single Evidence node with the key facts may suffice.

### Save the extraction

Save Claude's JSON output to a file:
```
{DocumentName}_claims.json
```

---

## Step 4b: Determine Relationships (Decision Framework)

For **every** evidence node you're about to create, walk through this checklist in order. The first two are always required. The rest depend on the content.

### Always Required

| # | Question | Relationship | Direction | Example |
|---|----------|-------------|-----------|---------|
| 1 | **What document contains this?** | CONTAINED_IN | Evidence → Document | Every evidence node must link to its source document |
| 2 | **Who said/wrote this?** | STATED_BY | Evidence → Person/Org | The author, witness, or party who made the statement |

### Conditional — Ask Each One

| # | Question | Relationship | Direction | When to create |
|---|----------|-------------|-----------|----------------|
| 3 | **Who is this about?** | ABOUT | Evidence → Person/Org | If the statement names or describes a specific person's actions, character, or situation |
| 4 | **Does anyone in the graph say the opposite?** | CONTRADICTS | Evidence ↔ Evidence | **Same speaker**, conflicting statements at different times. This is impeachment. Run the contradiction finder query below. |
| 5 | **Does a different person's evidence counter this?** | REBUTS | Evidence → Evidence | **Different speaker** refutes this statement. E.g., Morris's affidavit rebuts Phillips's claim about Marie. |
| 6 | **Does this directly support a Count?** | SUPPORTS | Evidence → LegalCount | If this evidence directly establishes an element of fraud, breach of fiduciary duty, abuse of process, or declaratory relief |
| 7 | **Does this prove a specific complaint allegation?** | PROVES | Evidence → ComplaintAllegation | If this evidence proves a specific numbered allegation from the complaint |
| 8 | **Does this evidence label/characterize an allegation?** | CHARACTERIZES | Evidence → ComplaintAllegation | If the statement makes a judgment about an allegation (e.g., calling it "frivolous") |
| 9 | **Does this rely on or cite other evidence?** | RELIES_ON | MotionClaim → Evidence | For motion claims that cite specific evidence to support an argument |

### Contradiction Finder Query

Before creating CONTRADICTS relationships, run this query to find existing statements by the same speaker that might conflict:

```cypher
// Find all statements by CFS or Phillips about a specific topic
// Replace the WHERE clause with your search terms
MATCH (e:Evidence)-[:STATED_BY]->(speaker)
WHERE speaker.id IN ["george-phillips", "catholic-family-service"]
  AND (e.content CONTAINS "children" OR e.content CONTAINS "child"
       OR e.title CONTAINS "children" OR e.verbatim_quote CONTAINS "children")
MATCH (e)-[:CONTAINED_IN]->(d:Document)
RETURN e.id, e.title, e.verbatim_quote, e.page_number,
       d.title AS document, speaker.id AS speaker
ORDER BY e.date
```

### Rebuttal Finder Query

Find statements by other parties that this new evidence might rebut:

```cypher
// Find statements by the opposing side about the same topic
MATCH (e:Evidence)-[:STATED_BY]->(speaker)
WHERE speaker.id NOT IN ["charles-penzien", "marie-awad"]
  AND (e.content CONTAINS "social security" OR e.content CONTAINS "SSA"
       OR e.title CONTAINS "social security")
MATCH (e)-[:CONTAINED_IN]->(d:Document)
RETURN e.id, e.title, e.verbatim_quote, d.title AS document, speaker.id AS speaker
```

### Legal Count Reference

These are the current LegalCount nodes in the graph:

| Count ID | Name | When to link |
|----------|------|-------------|
| count-breach-fiduciary-duty | Breach of Fiduciary Duty | Evidence of lying, concealment, or failure to disclose by a fiduciary |
| count-fraud | Fraud | Evidence of knowing misrepresentation of material facts |
| count-declaratory-ultra-vires | Declaratory/Ultra Vires | Evidence about rights, duties, or scope of authority |
| count-abuse-of-process | Abuse of Process | Evidence of using court proceedings for improper purposes |

### Decision Example: SSA Form 1724

Here's how the framework applies to the SSA Form 1724 "no living children" statement:

1. **CONTAINED_IN** → `doc-ssa-form-1724` ✅ (always required)
2. **STATED_BY** → `catholic-family-service` ✅ (CFS is the applicant)
3. **ABOUT** → `marie-awad` ✅ (and other Awad children — they're the ones denied)
4. **CONTRADICTS** → Run the contradiction finder for "children" statements by CFS. If CFS admitted knowing about children in their interrogatory responses, that's a CONTRADICTS edge. ✅
5. **REBUTS** → Not applicable here (this isn't countering someone else's statement)
6. **SUPPORTS** → `count-fraud` ✅ (knowingly false representation) AND `count-breach-fiduciary-duty` ✅ (fiduciary lying to federal agency)
7. **PROVES** → Check complaint allegations about concealment or misrepresentation
8. **CHARACTERIZES** → Not applicable (this isn't labeling an allegation)
9. **RELIES_ON** → Not applicable (this isn't a motion claim)

---

## Step 5: Review and Approve Extraction

**This step is critical — do not skip.**

Review each extracted claim:
- Is the verbatim quote actually in the document at the cited page?
- Is the speaker attribution correct?
- Are exhibit references accurate?
- Are there duplicate or overlapping claims that should be merged?

Edit the JSON as needed before import.

---

## Step 6: Import Evidence into Neo4j

### Generate Cypher from the reviewed JSON

Ask Claude to generate Cypher CREATE statements from your reviewed JSON. The pattern:

```cypher
// Evidence node
CREATE (e:Evidence {
  id: "evidence-CHANGEME-001",
  title: "Short description",
  content: "Longer description",
  verbatim_quote: "Exact quote",
  page_number: 1,
  date: "YYYY-MM-DD",
  topic: "category"
})

// Link to document
MATCH (d:Document {id: "doc-CHANGEME"})
MATCH (e:Evidence {id: "evidence-CHANGEME-001"})
CREATE (e)-[:CONTAINED_IN]->(d)

// Link to speaker
MATCH (p:Person {id: "george-phillips"})
MATCH (e:Evidence {id: "evidence-CHANGEME-001"})
CREATE (e)-[:STATED_BY]->(p)

// Link to subject (if about a specific person)
MATCH (p:Person {id: "marie-awad"})
MATCH (e:Evidence {id: "evidence-CHANGEME-001"})
CREATE (e)-[:ABOUT]->(p)
```

### Additional relationships (as applicable):

```cypher
// Evidence supports a legal count
MATCH (e:Evidence {id: "evidence-CHANGEME-001"})
MATCH (lc:LegalCount {id: "count-fraud"})
CREATE (e)-[:SUPPORTS]->(lc)

// Evidence contradicts another statement
MATCH (e1:Evidence {id: "evidence-CHANGEME-001"})
MATCH (e2:Evidence {id: "evidence-phillips-q48"})
CREATE (e1)-[:CONTRADICTS {topic: "surviving children", value: "none vs three"}]->(e2)

// Evidence proves a complaint allegation
MATCH (e:Evidence {id: "evidence-CHANGEME-001"})
MATCH (a:ComplaintAllegation {id: "allegation-005"})
CREATE (e)-[:PROVES]->(a)
```

### Run in Neo4j Browser

Run on DEV first. After each batch:
```cypher
// Verify — count new nodes
MATCH (e:Evidence) WHERE e.id STARTS WITH "evidence-CHANGEME"
RETURN count(e) AS new_evidence_count
```

---

## Step 7: Index Evidence in Qdrant

New Evidence nodes need to be embedded and indexed in the `colossus_evidence` Qdrant collection so the Chat/RAG pipeline can find them via vector search.

### Option A: Re-run the full indexing pipeline

Run the Qdrant sync playbook via Semaphore. This re-indexes all evidence from Neo4j.

### Option B: Add individual vectors (if the sync pipeline supports incremental)

Currently the sync is full-rebuild. If this becomes a bottleneck, we can add an incremental indexing endpoint to the backend.

### Verify

```bash
# Check point count increased
curl -s http://10.10.100.200:6333/collections/colossus_evidence | jq '.result.points_count'
```

The count should increase by the number of new Evidence nodes.

---

## Step 8: Add Decomposer Alias (if needed)

If the document should be findable by a short name in Chat (e.g., "supplemental brief" or "SSA form"), add aliases to two places:

### 1. Router aliases in `main.rs`

In `colossus-legal/backend/src/main.rs`, find the `document_aliases` HashMap and add:

```rust
document_aliases.insert("supplemental brief".into(), "doc-penzien-supplemental-brief".into());
document_aliases.insert("ssa form".into(), "doc-ssa-form-1724".into());
```

### 2. Decomposition prompt template (if externalized)

If using externalized prompts, the document aliases are injected from the HashMap — no prompt file change needed.

**Note:** Adding aliases requires a code change and rebuild. This is a candidate for future externalization (like we did with prompts).

---

## Step 9: Verify in Web App

### Document viewer
Navigate to Documents page → confirm the new document appears and the PDF opens.

### Chat/Ask
Ask a question that should retrieve content from the new document:
```
What does the supplemental brief say about the SSA Form 1724?
```

Confirm the answer cites evidence from the new document with correct page numbers.

### Evidence Explorer
Navigate to Evidence Explorer → confirm new evidence nodes appear with correct source links.

---

## Quick Reference: Existing Patterns

### Document ID patterns
| Type | Pattern | Example |
|------|---------|---------|
| Complaint | `doc-{plaintiff}-complaint` | `doc-awad-complaint` |
| Discovery response | `doc-{party}-discovery-response` | `doc-phillips-discovery-response` |
| Brief | `doc-{author}-{type}-{case}` | `doc-penzien-coa-brief-300891` |
| Court ruling | `doc-{court}-ruling-{date}` | `doc-coa-ruling-011212` |
| Affidavit | `doc-{person}-affidavit` | `doc-morris-affidavit` |
| Motion | `doc-{party}-{motion-type}` | `doc-phillips-motion-for-default` |
| Government form | `doc-{agency}-{form}` | `doc-ssa-form-1724` |

### Evidence ID patterns
| Source | Pattern | Example |
|--------|---------|---------|
| Discovery | `evidence-{party}-q{number}` | `evidence-phillips-q74` |
| Brief | `evidence-{brief}-{topic}` | `evidence-penzien-appeal-fiduciary` |
| Affidavit | `evidence-{person}-{topic}` | `evidence-morris-caregiver-daily` |
| Form | `evidence-{form}-{fact}` | `evidence-ssa1724-no-children` |

### Person ID patterns
All lowercase, hyphenated: `george-phillips`, `marie-awad`, `catholic-family-service`

---

## Checklist Template

Copy this for each new document:

```
Document: _______________
Document ID: doc-_______________
Filename: _______________.pdf

[ ] Step 1: ID chosen, filename chosen, registered in doc index
[ ] Step 2: PDF copied to DEV host (/dev-zfs/legal-docs/)
[ ] Step 2: PDF copied to PROD host (/prod-zfs/legal-docs/)
[ ] Step 3: Document node created in DEV Neo4j (with file_path property matching exact PDF filename)
[ ] Step 3: PDF accessible via https://colossus-legal-api-dev.cogmai.com/documents/{id}/file
[ ] Step 4: Claims extracted by Claude
[ ] Step 5: Claims reviewed and approved
[ ] Step 6: Evidence nodes created in DEV Neo4j
[ ] Step 6: Relationships created (CONTAINED_IN, STATED_BY, ABOUT, SUPPORTS, etc.)
[ ] Step 7: Qdrant re-indexed (point count verified)
[ ] Step 8: Decomposer aliases added (if needed) — requires rebuild
[ ] Step 9: Verified in Documents page
[ ] Step 9: Verified in Chat (question returns new evidence)
[ ] Step 9: Verified in Evidence Explorer
[ ] PROD: Document node created in PROD Neo4j (with file_path property)
[ ] PROD: Evidence nodes created in PROD Neo4j  
[ ] PROD: Qdrant re-indexed on PROD
```
