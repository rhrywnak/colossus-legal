# Colossus-Legal Project Roadmap

**Primary Goal:** Support the lawsuit - evidence organization for legal team  
**Timeline Pressure:** Medium-High  
**Last Updated:** 2026-01-23

---

## The Big Picture

```
┌─────────────┐    ┌─────────────┐    ┌─────────────────────────────────────────────┐
│  DOCUMENTS  │ → │  PROCESSING │ → │              COMPLETE GRAPH                  │
│             │    │             │    │                                             │
│ Source PDFs │    │ Extract     │    │  All entities, relationships, and harms    │
│ & rulings   │    │ structured  │    │  fully connected = ANY question answerable │
│             │    │ claims      │    │                                             │
└─────────────┘    └─────────────┘    └──────────────────┬──────────────────────────┘
     INPUT            PROCESS                            │
                                                         ▼
                                    ┌─────────────────────────────────────────────┐
                                    │            FLEXIBLE QUERY                    │
                                    │                                             │
                                    │  • Query Pattern Library ✅                  │
                                    │  • Graph exploration                        │
                                    │  • Faceted search                           │
                                    │  • Common query shortcuts                   │
                                    └──────────────────┬──────────────────────────┘
                                                       │
                                                       ▼
                                    ┌─────────────────────────────────────────────┐
                                    │           COURT-READY OUTPUT                 │
                                    │                                             │
                                    │  Dynamic reports based on query results     │
                                    └─────────────────────────────────────────────┘
```

## Core Principle

**If the data is complete and properly connected, any reasonable question can be answered.**

We do NOT hard-code specific questions. Instead, we build:
1. A **complete graph** with all evidence, allegations, harms, and connections ✅
2. A **flexible query interface** that can explore the graph any way needed 🟡
3. **Dynamic output generation** that formats results for court use 🔴

---

## Current State Assessment

| Stage | Status | Details |
|-------|--------|---------|
| Documents | ✅ 100% | 13 documents in Neo4j |
| Processing | ✅ 100% | All core docs processed |
| Database | ✅ 100% | 146 nodes, 340 rels; 18/18 allegations complete |
| Query Library | ✅ 100% | 35 queries in QUERY_PATTERNS.md |
| Query UI | 🔴 5% | No user interface exists |
| Output | 🔴 10% | Manual PDF generation only |

---

## Phase 1: Foundation ✅ COMPLETE

**Goal:** Build a complete, fully-connected graph that can answer ANY question

### 1.1 Document Inventory ✅
- [x] List all source documents with status
- [x] Identify any missing documents
- [x] Standardize naming convention

**Documents in Database (13 total):**

| Document | Type | Filed Date | In Neo4j |
|----------|------|------------|----------|
| Sabrina Morris Affidavit | Affidavit | 2010-02-12 | ✅ |
| Jeffrey Humphrey Affidavit | Affidavit | 2010-02-12 | ✅ |
| COA Opinion - First Appeal | Court Ruling | 2012-01-12 | ✅ |
| Judge Tighe Opinion | Court Ruling | 2012-04-12 | ✅ |
| COA Opinion - Reconsideration | Court Ruling | 2013-04-25 | ✅ |
| Camille Hanley Affidavit | Affidavit | 2013-12-17 | ✅ |
| Nadia Awad Affidavit | Affidavit | 2013-12-18 | ✅ |
| Phillips Summary Disposition Motion | Motion | 2013-12-20 | ✅ |
| Marie Awad Complaint | Complaint | 2014-06-03 | ✅ |
| Phillips Discovery Response | Discovery | 2016-08-01 | ✅ |
| CFS Interrogatory Response | Discovery | 2016-08-08 | ✅ |
| Motion for Default (CFS) | Motion | — | ✅ |
| Motion for Default (Phillips) | Motion | — | ✅ |

### 1.2 Complete Document Processing ✅
- [x] Process Judge Tighe Opinion
- [x] Process COA Initial Ruling
- [x] Process COA Reconsideration
- [x] Process Caregiver Affidavits (Morris, Humphrey)
- [x] Process Phillips Summary Disposition Motion
- [x] Process Sisters' Affidavits (Nadia, Camille)
- [x] Verify all existing claims.json files are complete

### 1.3 Complete Neo4j Population ✅
- [x] Add 3 court ruling Document nodes
- [x] Add 4 affidavit Document nodes
- [x] Add probate motion Document node
- [x] Complete all 18 allegations with evidence chains:

| ID | Allegation | Status |
|----|------------|--------|
| complaint-001 | Undisclosed CFS-Court Contract | ✅ Complete |
| complaint-002 | Funds Flow to Probate Court | ✅ Complete |
| complaint-003 | CFS Appointed Over Emil's Objection | ✅ Complete |
| complaint-004 | Emil Requested Dr. Armaly | ✅ Complete |
| complaint-005 | $50K Conversion by Sisters | ✅ Complete |
| complaint-006 | CFS Held $50K Without Authority | ✅ Complete |
| complaint-007 | Estate Was Unnecessary | ✅ Complete |
| complaint-008 | Pattern of Spurious Accusations | ✅ Complete |
| complaint-009 | "North Korea" Video Comment | ✅ Complete |
| complaint-010 | "Fanciful Conspiracy Theories" | ✅ Complete |
| complaint-011 | Auction Caused $6K Loss | ✅ Complete |
| complaint-012 | Marie's Certified Letter to Cooperate | ✅ Complete |
| complaint-013 | False Attorney Cost Claims | ✅ Complete |
| complaint-014 | Fraudulent Funeral Expense Claims | ✅ Complete |
| complaint-015 | Selective Sanctions | ✅ Complete |
| complaint-016 | 100% Costs to Marie Only | ✅ Complete |
| complaint-017 | MCL 700.1212 Violation | ✅ Complete |
| complaint-018 | CFS Ultra Vires | ✅ Complete |

### 1.4 Implement Harm Tracking ✅
- [x] Create Harm nodes for all identified damages:

| ID | Title | Category | Amount |
|----|-------|----------|--------|
| harm-001 | 100% Appellate Costs to Marie | financial_direct | $15,246.94 |
| harm-002 | MCR 2.114 Sanction - Lost Reimbursement | financial_direct | $2,345.00 |
| harm-003 | Unnecessary Auction Loss | financial_estate | $6,000.00 |
| harm-004 | Estate Depletion from Fees | financial_estate | TBD |
| harm-005 | Lost 1/3 of $50K Conversion | financial_estate | $16,666.67 |
| harm-006 | "North Korea" Comparison | reputational | N/A |
| harm-007 | "Fanciful Conspiracy Theories" | reputational | N/A |
| harm-008 | "Obstructive" Characterization | reputational | N/A |
| harm-009 | Selective Sanctions vs Sisters | reputational | N/A |

**Total Quantifiable Damages: $40,258.61**

- [x] Link each Harm to causing Allegation(s) via [:CAUSED_BY]
- [x] Link each Harm to supporting Evidence via [:EVIDENCED_BY]
- [x] Link each Harm to applicable LegalCount(s) via [:DAMAGES_FOR]

### 1.5 Ensure Graph Completeness ✅
- [x] Every Evidence node links to a Document (source)
- [x] Every MotionClaim links to Evidence it relies on
- [x] Every Allegation links to Count(s) it supports
- [x] Every Harm links to Allegation(s) and Evidence
- [x] No orphan nodes (everything connected to the graph)

### 1.6 Verify Traceability ✅
- [x] Run verification queries on all allegations
- [x] Confirm every allegation traces: Count ← Allegation ← MotionClaim ← Evidence
- [x] Verify Harm chains: Count ← Harm → Allegation → Evidence
- [x] Document any gaps

**Phase 1 Deliverable:** ✅ Complete, fully-connected Neo4j graph (146 nodes, 340 relationships)

---

## Phase 2: Query Layer (Flexible Access) 🟡 IN PROGRESS

**Goal:** Enable ANY question to be answered through flexible query mechanisms

### 2.1 Query Pattern Library ✅ COMPLETE
- [x] Build reusable Cypher patterns (not hard-coded questions)
- [x] Document in QUERY_PATTERNS.md

| Category | Queries | Purpose |
|----------|---------|---------|
| Damages | 5 | Calculate and break down $40K+ in damages |
| Evidence Chains | 4 | Trace proof from count to source document |
| Allegations | 4 | Navigate 18 complaint allegations |
| Defendant-Specific | 5 | Phillips and CFS admissions |
| Legal Counts | 5 | Full evidence for each of 4 counts |
| Court Presentation | 5 | Ready-to-use for motions/trial |
| Documents | 4 | Document inventory and analysis |
| Graph Exploration | 5 | Navigate and explore the graph |
| Validation | 6 | Data quality checks |
| **Total** | **35** | |

### 2.2 Document Expansion 🟡 IN PROGRESS
- [x] Add caregiver affidavits (Morris, Humphrey) - 2026-01-23
- [x] Add sisters' affidavits (Nadia, Camille) - 2026-01-23
- [x] Add Phillips Summary Disposition Motion - 2026-01-23
- [x] Create contradiction chains between caregiver and sister testimony
- [ ] Add billing statements (Phillips billed for missing docs)
- [ ] Add bank records (joint account proof)
- [ ] Add estate inventory documents

### 2.3 Query Service API 🔴
- [ ] GraphQL or REST API exposing flexible queries
- [ ] Accept parameters: node type, filters, traversal depth
- [ ] Return structured JSON with full context
- [ ] Include source citations in all responses

### 2.4 Natural Language Interface (Optional Enhancement) 🔴
- [ ] Parse user question to identify intent
- [ ] Map to appropriate query pattern
- [ ] Execute and return results
- [ ] Can be LLM-assisted or rule-based

**Phase 2 Deliverable:** Flexible API that can answer any graph question

---

## Phase 3: User Interface (Graph Exploration) 🔴 NOT STARTED

**Goal:** Non-technical users can explore the graph and find answers

### 3.1 Graph Explorer View
- [ ] Visual node/relationship display
- [ ] Click on node to see details and connections
- [ ] Expand/collapse related nodes
- [ ] Filter by node type, person, document

### 3.2 Search Interface
- [ ] Full-text search across all content
- [ ] Faceted filtering (by type, person, date, category)
- [ ] Results show context and connections

### 3.3 Quick Access Shortcuts
- [ ] "View all evidence for Count X" (one-click)
- [ ] "Show all harms" (one-click)
- [ ] "Phillips admissions" (one-click)
- [ ] These are convenience shortcuts, NOT the only options

### 3.4 Natural Language Query (Optional)
- [ ] Text input for questions
- [ ] System interprets and queries graph
- [ ] Falls back to search if intent unclear

**Phase 3 Deliverable:** Intuitive interface for exploring the complete graph

---

## Phase 4: Court-Ready Output (Dynamic Generation) 🔴 NOT STARTED

**Goal:** Any query result can be formatted for court use

### 4.1 Dynamic Report Templates
- [ ] Evidence Chain Report: Start from any node, show full chain to Count
- [ ] Damages Summary: Aggregate all harms with evidence
- [ ] Person Report: Everything involving a specific person
- [ ] Allegation Report: Complete proof for one allegation
- [ ] Count Report: All evidence supporting one legal count
- [ ] Contradiction Report: What was said vs. admitted

### 4.2 Report Generation Engine
- [ ] Accept any query result as input
- [ ] Apply appropriate template based on content
- [ ] Include proper legal citations
- [ ] Format for court submission standards

### 4.3 Export Options
- [ ] PDF with professional formatting
- [ ] Word document (editable)
- [ ] Markdown (for review)
- [ ] Print-optimized view

**Phase 4 Deliverable:** Any query result → court-ready document

---

## Database Statistics

| Label | Count |
|-------|-------|
| Evidence | 60 |
| MotionClaim | 5 |
| ComplaintAllegation | 18 |
| Document | 13 |
| Person | 12 |
| Harm | 9 |
| LegalCount | 4 |
| Organization | 3 |
| Case | 2 |
| Event | 1 |
| **Total Nodes** | **146** |
| **Total Relationships** | **340** |

**Last Backup:** 2026-01-23

---

## Recent Progress (2026-01-23)

### Documents Added Today
| Document | Type | Evidence Added | Significance |
|----------|------|----------------|--------------|
| Sabrina Morris Affidavit | Affidavit | 7 items | Corroborates Emil's video, $50K demand |
| Jeffrey Humphrey Affidavit | Affidavit | 3 items | Emil competent, health declined from Nadia visits |
| Phillips Summary Disposition | Motion | 3 items | Shows CFS attack strategy |
| Nadia Awad Affidavit | Affidavit | 1 item | Claims Marie took $140K (rebutted) |
| Camille Hanley Affidavit | Affidavit | 1 item | Identical to Nadia (coordination evidence) |

### Key Evidence Chains Created
1. **Caregiver corroboration chain**: Morris/Humphrey → Emil's video → complaint-009
2. **Sisters' coordination evidence**: Identical affidavits → bias indicator → complaint-001
3. **Contradiction chain**: Sisters' claims ← CONTRADICTED_BY → Caregiver testimony

### Database Growth
| Metric | Start of Day | End of Day | Change |
|--------|--------------|------------|--------|
| Nodes | 122 | 146 | +24 |
| Relationships | 297 | 340 | +43 |
| Documents | 8 | 13 | +5 |
| Evidence | 47 | 60 | +13 |

---

## Role Assignments

| Phase | Primary Role | Supporting Roles |
|-------|--------------|------------------|
| Phase 1 | DB Engineer | Data Architect (model validation) |
| Phase 2 | Software Architect | DB Engineer (query patterns) |
| Phase 3 | Software Architect | DB Engineer (API integration) |
| Phase 4 | Software Architect | All (output format review) |

---

## Success Criteria

**Phase 1 Complete When:** ✅ ACHIEVED
- [x] All 18 allegations have evidence chains in Neo4j
- [x] All identified harms are in database with connections
- [x] All source documents are represented
- [x] No orphan nodes - everything connected
- [x] Any traversal query returns complete chains

**Phase 2 Complete When:**
- [x] Query Pattern Library documented
- [x] Document expansion (caregiver affidavits, sisters' affidavits)
- [ ] API can execute any graph traversal
- [ ] Query results include full context and citations
- [ ] Response times acceptable (<2 seconds)

**Phase 3 Complete When:**
- [ ] Non-technical user can find any information without help
- [ ] Multiple access paths: visual, search, shortcuts, natural language
- [ ] Interface is intuitive and responsive

**Phase 4 Complete When:**
- [ ] Any query result can be exported as court-ready document
- [ ] Legal team approves output format
- [ ] Documents meet court submission standards

---

## Immediate Next Steps

1. ~~**Add caregiver affidavits**~~ ✅ Done 2026-01-23
2. ~~**Add sisters' affidavits**~~ ✅ Done 2026-01-23  
3. ~~**Add Phillips Summary Disposition**~~ ✅ Done 2026-01-23
4. **Optional:** Add billing statements to prove "billed but missing" pattern
5. **Optional:** Add bank records to prove joint account ownership
6. **Future:** Build Query Service API (Phase 2.3)

---

## Project Files

| File | Description |
|------|-------------|
| DATA_MODEL_v3.md | Current data model with Harm nodes |
| QUERY_PATTERNS.md | 35 reusable Cypher queries |
| DECISION_LOG.md | Architectural decisions |
| COORDINATION.md | Role coordination protocol |
| COLOSSUS_LEGAL_DOC_INDEX_V020.md | Document inventory (13 docs) |
| PROJECT_ROADMAP.md | This file |

---

## Key Architectural Decision

**DECIDED 2026-01-19:** We do NOT hard-code specific questions.

| ❌ Old Approach | ✅ New Approach |
|-----------------|-----------------|
| 16 pre-defined questions | Complete graph + flexible query |
| Fixed Cypher queries | Query pattern library |
| Question dropdown UI | Graph explorer + search + shortcuts |
| Limited answers | Any question answerable |

This decision ensures the system remains flexible as litigation needs evolve.

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-01-19 | Initial roadmap created |
| 1.1 | 2026-01-19 | Added Harm tracking (Phase 1.4), expanded question list |
| 1.2 | 2026-01-19 | **Major revision:** Replaced hard-coded questions with flexible query approach |
| 2.0 | 2026-01-22 | **Phase 1 Complete:** All 18 allegations, 9 harms, 8 documents, 122 nodes, 297 rels |
| 2.1 | 2026-01-22 | **Phase 2.1 Complete:** Query Pattern Library (35 queries) |
| 2.2 | 2026-01-23 | **Document Expansion:** 13 documents, 146 nodes, 340 rels; added caregiver/sister affidavits |
