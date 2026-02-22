# Colossus-Legal — TASK TRACKER v2

**Last Updated:** 2026-02-22
**Current Phase:** Phase G — User Experience & Analytical Access
**Branch:** main (all prior features merged)

---

# Active Priorities

> **Guiding principle:** The knowledge graph is mature (227 nodes, 644 relationships, 18/18 PROVEN).
> The bottleneck is now *access* — making this data useful to Marie and Penzien.

| Priority | Feature | Why | Status |
|----------|---------|-----|--------|
| **0a** | Fix Damages Page (blank) | Broken existing feature | 🔴 BUG |
| **0b** | Fix View PDF (documents page) | Broken — no file-serving endpoint | 🔴 BUG |
| **1** | Dashboard Redesign — Case Intelligence | First thing users see; currently shows DB stats, not case insight | TODO |
| **2** | Person Detail View | Click person → statements, timeline, rebuttals (STATED_BY data exists) | TODO |
| **3** | Quick Queries / Shortcuts | 35 Cypher queries in QUERY_PATTERNS.md, none accessible from UI | TODO |
| **4** | Full-Text Search | Most natural access pattern for both users | TODO |
| **5** | Enhanced Document Detail | "What does this document prove?" evidence chains | TODO |
| **6** | Count-Centric View | Full evidence package per legal count for brief writing | TODO |
| **7** | Graph Visualization | Interactive evidence chains (React Flow + dagre) | TODO |

---

# Bugs

### BUG-001: Damages Page Blank
**Severity:** High — page renders with no header and no body
**Discovered:** 2026-02-22
**Likely Cause:** Backend `/harms` query may have broken after Schema v4 migrations, or frontend rendering issue. The 9 Harm nodes ($40,258.61) still exist in Neo4j.
**Action:** Investigate backend response first (`curl localhost:3403/harms`), then check frontend.

### BUG-002: View PDF Not Functional
**Severity:** High — "View PDF" links on Documents page go nowhere
**Discovered:** 2026-02-22
**Cause:** No document-serving backend endpoint exists. PDFs are in the project folder but the backend has no mechanism to serve them.
**Action:** Create `GET /documents/:id/file` endpoint serving PDFs from a configurable `DOCUMENTS_DIR`. Wire frontend links.

### BUG-003: Evidence Count Discrepancy (pre-existing)
**Severity:** Low — not blocking
**Description:** `/case` endpoint reports 102 evidence vs `/evidence` returning 109 and `/schema` showing 227 nodes.
**Action:** Investigate case stats Cypher query. Deferred.

---

# Phase G — User Experience & Analytical Access 🔄 IN PROGRESS

> **Goal:** Transform entity browsers into analytical workflows for Marie Awad and Chuck Penzien.
> **User context:** Marie is non-technical; Penzien is an attorney with court deadlines.
> Neither knows Cypher. The interface must be self-explanatory.

## Feature G.0 — Bug Fixes 🔴

| Task | Description | Status |
|------|-------------|--------|
| T.G.0.1 | Investigate and fix Damages page blank render | TODO |
| T.G.0.2 | Create GET /documents/:id/file endpoint (PDF serving) | TODO |
| T.G.0.3 | Wire frontend "View PDF" links to file endpoint | TODO |
| T.G.0.4 | Add DOCUMENTS_DIR env var to backend config | TODO |

## Feature G.1 — Dashboard Redesign

> **Goal:** Replace database-statistics dashboard with a case intelligence briefing.

**What it should show:**
- Case strength: "18 of 18 allegations PROVEN. All 4 legal counts fully supported."
- Damages: "$40,258.61 quantifiable + 4 reputational harms"
- Impeachment summary: George's 14 characterizations → 9 directly rebutted → 18/18 proven
- Key finding callout: George called claims "frivolous" — CFS admitted they were true
- Quick-launch links: "George's Contradictions" / "Proof for Count I" / "Damages Breakdown" / "CFS Admissions"

| Task | Description | Status |
|------|-------------|--------|
| T.G.1.1 | Design /case-summary backend endpoint (aggregated case intelligence) | TODO |
| T.G.1.2 | Implement case-summary repository (Cypher queries) | TODO |
| T.G.1.3 | Redesign Dashboard component — case briefing layout | TODO |
| T.G.1.4 | Add quick-launch navigation cards with deep links | TODO |

## Feature G.2 — Person Detail View

> **Goal:** Click a person → see everything they said, when, in what document, and how it was rebutted.
> **Key data:** STATED_BY relationships (84 total), ABOUT relationships (33), statement_date, verbatim_quote

**User stories served:** US-002 (Search by Person), US-007 (Defendant Admissions)

| Task | Description | Status |
|------|-------------|--------|
| T.G.2.1 | Design GET /persons/:id/detail endpoint | TODO |
| T.G.2.2 | Implement person detail repository — statements with sources | TODO |
| T.G.2.3 | Include rebuttal data where REBUTS relationships exist | TODO |
| T.G.2.4 | PersonDetailPage component — statement timeline view | TODO |
| T.G.2.5 | Wire "View Detail →" links from People page | TODO |

## Feature G.3 — Quick Queries / Analytical Shortcuts

> **Goal:** Surface the 35 predefined Cypher queries from QUERY_PATTERNS.md through the UI.
> **Categories:** Damages (5), Evidence Chains (4), Allegations (4), Defendant-Specific (5),
> Legal Counts (5), Court Presentation (5), Documents (4), Graph Exploration (5)

**User stories served:** US-006 (Find Contradictions), US-007 (Defendant Admissions)

| Task | Description | Status |
|------|-------------|--------|
| T.G.3.1 | Design GET /queries/:query_id endpoint (parameterized) | TODO |
| T.G.3.2 | Implement query registry — map query IDs to Cypher templates | TODO |
| T.G.3.3 | Implement query executor — run registered query, return structured results | TODO |
| T.G.3.4 | QuickQueriesPage component — categorized query cards | TODO |
| T.G.3.5 | QueryResultsView component — display tabular results with citations | TODO |
| T.G.3.6 | Add "Quick Queries" to header navigation | TODO |

## Feature G.4 — Full-Text Search

> **Goal:** Type "auction" or "North Korea" or "$50,000" → find every evidence node mentioning it.
> **Searchable fields:** verbatim_quote, answer, significance, title, allegation text

| Task | Description | Status |
|------|-------------|--------|
| T.G.4.1 | Design GET /search?q=... endpoint | TODO |
| T.G.4.2 | Implement search repository — CONTAINS queries across multiple fields | TODO |
| T.G.4.3 | SearchPage component — search box + results with highlighting | TODO |
| T.G.4.4 | Add search to header (persistent search bar or nav link) | TODO |

## Feature G.5 — Enhanced Document Detail

> **Goal:** Click a document → see "What does this document prove?" — evidence chain flowing from it.

**User stories served:** US-003 (Search by Document)

| Task | Description | Status |
|------|-------------|--------|
| T.G.5.1 | Extend GET /documents/:id endpoint with evidence chain data | TODO |
| T.G.5.2 | Show evidence nodes contained in document | TODO |
| T.G.5.3 | Show which allegations those evidence nodes prove (via MotionClaim chain) | TODO |
| T.G.5.4 | Update DocumentDetailPage with evidence chain section | TODO |

## Feature G.6 — Count-Centric View

> **Goal:** Select a legal count → see full evidence package (allegations, claims, evidence, documents).
> **Primary user:** Penzien drafting brief sections.

**User stories served:** US-008 (Evidence Chain Tracing)

| Task | Description | Status |
|------|-------------|--------|
| T.G.6.1 | Design GET /counts/:id/detail endpoint | TODO |
| T.G.6.2 | Implement count detail repository — full evidence chain per count | TODO |
| T.G.6.3 | CountDetailPage component — tree/outline of proof chain | TODO |
| T.G.6.4 | Add count selection to existing pages (Allegations, Dashboard) | TODO |

## Feature G.7 — Interactive Graph Visualization

> **Goal:** Visual evidence chains using React Flow + dagre for hierarchical display.
> **Hierarchies:** Legal Proof, Damages, Document, Party, Decomposition

| Task | Description | Status |
|------|-------------|--------|
| T.G.7.1 | Expand GraphNodeType enum (add Person, Harm, Organization) | TODO |
| T.G.7.2 | GET /graph/:hierarchy_type endpoint with enum dispatch | TODO |
| T.G.7.3 | Legal Proof query (Count → Allegation → Claim → Evidence → Document) | TODO |
| T.G.7.4 | Damages query (Count → Harm → Allegation → Evidence) | TODO |
| T.G.7.5 | Document query (Document → Evidence → Claims → Allegations) | TODO |
| T.G.7.6 | Party query (Person → Evidence → Documents) | TODO |
| T.G.7.7 | Decomposition query (Person → CHARACTERIZES → Allegation + REBUTS) | TODO |
| T.G.7.8 | Install @xyflow/react + @dagrejs/dagre | TODO |
| T.G.7.9 | GraphViewer component (React Flow + dagre layout) | TODO |
| T.G.7.10 | GraphPage with hierarchy selector | TODO |
| T.G.7.11 | Node styling by type + click-to-navigate | TODO |

---

# Completed Phases (Compressed)

## Phase 0 — Project Initialization ✅ COMPLETE
- T0.1 Repository Bootstrap, T0.2 Architecture Docs, T0.3 Codex Safety Bundle

## Phase 1 — Foundations ✅ COMPLETE
- T1.1 Backend Skeleton (Axum), T1.2 Neo4j Integration, T1.3 Frontend Skeleton (React/Vite), T1.5 Dev Env Config

## Phase 2 — Query Layer ✅ COMPLETE (2026-01-29)

> **Goal:** Expose Neo4j database to end users through REST API and React UI.

| Feature | Tasks | Deliverable |
|---------|-------|-------------|
| F2.1 Schema Discovery | T2.1.1–T2.1.3 | GET /schema, Dashboard with entity counts |
| F2.2 Persons API + UI | T2.2.1–T2.2.2 | GET /persons, People page with role badges |
| F2.3 Allegations API + UI | T2.3.1–T2.3.3 | GET /allegations, Allegations page, duplicate fix |
| F2.4 Evidence API + UI | T2.4.1–T2.4.3 | GET /evidence, Evidence page with CRITICAL highlighting |
| F2.5 Harms/Damages API + UI | T2.5.1–T2.5.3 | GET /harms, Damages page with totals |

## Phase 2.5 — Extended Query Layer ✅ COMPLETE (2026-01-29)

| Feature | Tasks | Deliverable |
|---------|-------|-------------|
| F2.6 MotionClaims API + UI | T2.6.1–T2.6.3 | GET /motion-claims, Claims page |
| F2.7 Contradictions API + UI | T2.7.1–T2.7.2 | GET /contradictions, Side-by-side comparison |

## Phase 3 — Document Slice ⏸️ PARTIAL

| Task | Status |
|------|--------|
| T3.1a Document API L0 (Skeleton) | DONE (2025-12-03) |
| T3.1b Document API L1 (Neo4j) | DONE (2025-12-03) |
| T3.1c Document API L2 (Validation) | DEFERRED |
| T3.2a–T3.2b Document UI | DONE (2025-12-03) |
| T3.3 Document Slice Integration | DONE (2025-12-02) |

## Phase 5 — Schema v2 + Claims Import ⏸️ PARTIAL

| Task | Status |
|------|--------|
| F5.1 Schema v2 Migration | DONE (2025-12-20) |
| F5.2 Import Validation Endpoint | DONE (2025-12-23) |
| F5.3 Import Execution Endpoint | DEFERRED |
| F5.5 Frontend Import UI | DEFERRED |

## Schema Evolution v4 — Phases A–E ✅ COMPLETE (2026-02-20)

> **Goal:** Statement attribution, timeline support, claim decomposition.

| Phase | What It Did | Migrations |
|-------|-------------|------------|
| A — Non-Destructive Additions | statement_date, statement_type, verbatim_quote on Evidence; 18 Event nodes | MIG-001–MIG-003 |
| B — Statement Attribution | 84 STATED_BY, 33 ABOUT relationships; retired INVOLVES on Evidence | MIG-004–MIG-005 |
| C — New Evidence Nodes | 14 George Phillips CoA evidence nodes; SSA fraud thread (10 nodes) | MIG-006–MIG-009 |
| D — Claim Decomposition | 47 CHARACTERIZES, 19 REBUTS; 18/18 allegations covered | MIG-010–MIG-012b |
| E — Validation | All completeness checks passed | — |

## Phase F — Decomposition Query Layer ✅ COMPLETE (2026-02-20)

> **Goal:** Expose Phase D analytical results through API and UI.

| Feature | Tasks | Deliverable |
|---------|-------|-------------|
| F.1 Decomposition API | 3 endpoints | GET /decomposition, GET /allegations/:id/detail, GET /rebuttals |
| F.2 Decomposition UI | 2 pages | DecompositionPage (summary + table), AllegationDetailPage (characterization → rebuttal chains) |
| F.3 Existing UI Refresh | Updated Evidence page | Added stated_by, verbatim_quote (blue blockquotes), page badges |
| Module Size Refactoring | Split 505-line file | 3 compliant modules (190, 233, 184 lines) |

---

# API Endpoints Summary (17 endpoints)

| Endpoint | Method | Purpose | Status |
|----------|--------|---------|--------|
| `/health` | GET | Health check | ✅ |
| `/api/status` | GET | Backend status | ✅ |
| `/schema` | GET | Database discovery | ✅ |
| `/persons` | GET | List persons | ✅ |
| `/allegations` | GET | List allegations | ✅ |
| `/evidence` | GET | List evidence (with stated_by, verbatim_quote) | ✅ |
| `/harms` | GET | List harms/damages | ⚠️ BUG-001 |
| `/motion-claims` | GET | List motion claims | ✅ |
| `/contradictions` | GET | List contradictions | ✅ |
| `/documents` | GET | List documents | ✅ |
| `/documents/:id` | GET | Document detail | ✅ |
| `/documents` | POST | Create document | ✅ |
| `/documents/:id` | PUT | Update document | ✅ |
| `/claims` | GET | List claims (old) | ✅ |
| `/claims/:id` | GET | Claim detail | ✅ |
| `/decomposition` | GET | Decomposition overview (18 allegations) | ✅ |
| `/allegations/:id/detail` | GET | Allegation detail (characterizations → rebuttals) | ✅ |
| `/rebuttals` | GET | All rebuttals grouped by claim | ✅ |
| `/import/validate` | POST | Validate import JSON | ✅ |

**Planned endpoints (Phase G):**
- `GET /documents/:id/file` — Serve PDF files
- `GET /case-summary` — Aggregated case intelligence
- `GET /persons/:id/detail` — Person statement timeline
- `GET /queries/:query_id` — Parameterized predefined queries
- `GET /search?q=...` — Full-text search
- `GET /counts/:id/detail` — Count evidence package
- `GET /graph/:hierarchy_type` — Graph visualization data

---

# Frontend Pages Summary (13 pages)

| Route | Page | Status |
|-------|------|--------|
| `/` | Dashboard | ✅ (needs redesign — G.1) |
| `/allegations` | Allegations | ✅ |
| `/allegations/:id/detail` | Allegation Detail | ✅ |
| `/claims` | Motion Claims | ✅ |
| `/documents` | Documents List | ✅ |
| `/documents/:id` | Document Detail | ✅ |
| `/evidence` | Evidence | ✅ |
| `/damages` | Harms/Damages | ⚠️ BUG-001 (blank) |
| `/people` | People | ✅ |
| `/contradictions` | Contradictions | ✅ |
| `/decomposition` | Decomposition Overview | ✅ |
| `/hearings` | Hearings | Placeholder |
| `/decisions` | Decisions | Placeholder |

**Planned pages (Phase G):**
- `/people/:id` — Person Detail (G.2)
- `/queries` — Quick Queries (G.3)
- `/search` — Full-Text Search (G.4)
- `/counts/:id` — Count Detail (G.6)
- `/graph` — Graph Visualization (G.7)

---

# Database Statistics (2026-02-20)

| Entity | Count |
|--------|-------|
| Evidence | 102 (100% grounded with verbatim quotes + page numbers) |
| ComplaintAllegations | 18 (all PROVEN) |
| MotionClaims | 26 |
| Documents | 16 (in Neo4j; 18 source PDFs total) |
| Persons | 10 |
| Organizations | 3 |
| Events | 18 |
| Harms | 9 ($40,258.61 quantifiable) |
| LegalCounts | 4 |
| Case | 1 |
| **Total Nodes** | **227** |
| **Total Relationships** | **644** |

**Key relationship counts:**
- 84 STATED_BY (statement attribution)
- 47 CHARACTERIZES (George's claims → allegations)
- 33 ABOUT (evidence about a person/org)
- 19 REBUTS (third-party countering defendant statements)
- 5 CONTRADICTS (same speaker, conflicting statements)

---

# Technical Debt

| Item | Source | Priority |
|------|--------|----------|
| 2 tests ignored in `tests/claims_list.rs` (v1 test data) | Phase 5 | Low |
| 1 test ignored in `tests/claims_validation.rs` (ClaimRepository v1) | Phase 5 | Low |
| 1 test ignored in `tests/documents_list.rs` (invalid doc_type) | Phase 5 | Low |
| 2 pre-existing clippy warnings (not from our code) | — | Low |
| `.env` password requires backslash-escaped `$` for dotenvy | — | Documented |
| Hearings and Decisions pages are placeholders | Phase 2 | Low |

---

# Data Quality Issues

### DATA-001: Review CONTRADICTED_BY Relationships
**Priority:** Medium | **Status:** Open
Current CONTRADICTED_BY relationships may not represent actual logical contradictions. Need review.

### DATA-002: Identify Missing Contradictions
**Priority:** Medium | **Status:** Partially addressed by Phase D
Phase D added 19 REBUTS and 47 CHARACTERIZES. Systematic contradiction review for remaining document pairs still needed.

### DATA-003: Refine Document Ingestion Process
**Priority:** Low | **Status:** Open
Extraction prompts need improvement for better contradiction detection.

---

# Known Issues

- Evidence count discrepancy: `/case` reports 102 vs `/evidence` returning 109 (BUG-003, pre-existing)
- `.env` password requires `\$` escaping for dotenvy `$`-variable expansion
- Nadia Awad and Camille Hanley affidavit PDFs not uploaded to project (Evidence nodes exist, quotes need backfill from parent doc #13 exhibits)
- Penzien Reply Brief Parts 1 & 2 at STAGE 1 (need OCR + extraction)
- 7 Evidence nodes from Penzien CoA Brief missing verbatim quotes

---

# Infrastructure

### Production Deployment ✅ COMPLETE
Managed by the **colossus-homelab** project (separate repo).
- **Ansible + Semaphore UI** — automated containerization and deployment pipeline
- **Targets:** DEV and PROD servers on Proxmox cluster
- **Capabilities:** Build frontend/backend containers, deploy to either environment, version deployed code
- **Process:** Code changes on main → Ansible playbook → containerize → deploy → verify

---

# Future Phases (After Phase G)

| Phase | Description | Estimated Effort |
|-------|-------------|------------------|
| Password Protection | Basic auth for UI | 1 session |
| Export / Court-Ready Output | PDF/Word generation from query results | 2-3 sessions |
| Natural Language Query | LLM-powered document Q&A (RAG with Qdrant) | Exploration |
| Document Upload + Extraction | In-app document processing pipeline | Future |

---

# Development Standards (Reminder)

```
✅ 0-200 lines: IDEAL
⚠️ 201-300 lines: ACCEPTABLE
🛑 301-500 lines: MANDATORY refactoring
🚨 501+ lines: PROHIBITED — cannot commit
Functions: max 50 lines
```

**Workflow:** Opus (architecture) → Claude Code Opus 4.6 (implementation with approval gates) → Roman (execution/verification)

**Git:** Feature branches per feature set. Merge to main after verification. Main stays deployable.

---

# User Stories Cross-Reference

| User Story | Description | Served By |
|------------|-------------|-----------|
| US-001 | Database Discovery | Phase 2 (basic) → G.1 Dashboard Redesign (enhanced) |
| US-002 | Search by Person | G.2 Person Detail View |
| US-003 | Search by Document | G.5 Enhanced Document Detail |
| US-004 | View Allegations + Evidence | Phase F (AllegationDetailPage) ✅ |
| US-005 | View Damages | Phase 2.5 (when BUG-001 fixed) → G.1 Dashboard |
| US-006 | Find Contradictions | Phase 2.5 (basic) → G.3 Quick Queries (enhanced) |
| US-007 | Defendant Admissions | G.2 Person Detail + G.3 Quick Queries |
| US-008 | Evidence Chain Tracing | G.6 Count-Centric View + G.7 Graph Visualization |

---

# End of TASK_TRACKER v2
