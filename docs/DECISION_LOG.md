# Colossus-Legal Decision Log

This log tracks decisions made across all roles that may impact other team members. All roles must check this document at session start.

---

## 2026-01-19 | DATA ARCHITECT (Roman Approved)

### Decision
Added **Harm** node type to the data model to track damages caused to Marie.

### Rationale
The lawsuit's core purpose is to prove:
1. Incompetence and negligence by CFS/Phillips
2. **Harm caused to Marie** as a result

Without tracking Harm explicitly, we cannot:
- Quantify damages for court
- Connect misconduct to actual injury
- Answer "what did this cost Marie?"

### Harm Categories Defined

| Category | Type | Description |
|----------|------|-------------|
| `financial_direct` | Sanctions | Money taken directly from Marie (e.g., 100% cost assessment) |
| `financial_estate` | Incompetence | Estate losses that reduced Marie's inheritance |
| `reputational` | Character Attacks | False accusations damaging Marie's reputation |

### New Node Type: Harm

```
Harm {
  id: string,                    // "harm-001"
  title: string,                 // Short description
  category: string,              // financial_direct, financial_estate, reputational
  subcategory: string,           // sanction, incompetence, character_attack
  amount: float | null,          // Dollar amount if quantifiable
  description: date | null,      // Detailed description
  date: date | null,             // When harm occurred
  source_reference: string       // Document/page reference
}
```

### New Relationships

| Relationship | From | To | Meaning |
|--------------|------|-----|---------|
| `CAUSED_BY` | Harm | ComplaintAllegation | This harm resulted from this misconduct |
| `EVIDENCED_BY` | Harm | Evidence | This evidence proves the harm occurred |
| `DAMAGES_FOR` | Harm | LegalCount | This harm supports damages claim for this count |

### Impacts
- Data Architect: Schema updated to v3 with Harm node
- DB Engineer: **MUST IMPLEMENT** - Add Harm nodes for all identified damages
- Software Architect: API must expose harm/damages queries

### Action Required
- [ ] DB Engineer: Create Harm nodes for identified damages
- [ ] DB Engineer: Link Harms to Allegations via CAUSED_BY
- [ ] DB Engineer: Link Harms to Evidence via EVIDENCED_BY
- [ ] Software Architect: Plan API endpoints for damage queries

### Known Harms to Populate

| ID | Title | Category | Amount |
|----|-------|----------|--------|
| harm-001 | 100% Appellate Costs to Marie | financial_direct | $15,246.94 |
| harm-002 | MCR 2.114 Sanction - Lost Reimbursement | financial_direct | $2,345.00 |
| harm-003 | Unnecessary Auction Loss | financial_estate | ~$6,000 |
| harm-004 | Estate Depletion from Fees | financial_estate | TBD |
| harm-005 | Lost 1/3 of $50K Conversion | financial_estate | ~$16,667 |
| harm-006 | "North Korea" Comparison | reputational | N/A |
| harm-007 | "Fanciful Conspiracy Theories" | reputational | N/A |
| harm-008 | "Obstructive" Characterization | reputational | N/A |
| harm-009 | Selective Sanctions vs Sisters | reputational | N/A |

---

## 2026-01-19 | PROJECT MANAGEMENT (Roman)

### Decision
Clarified project mission and team structure.

### Key Points
1. Legal team is THREE people: one attorney, Marie, and Roman
2. Colossus-Legal must function as their "high powered law firm"
3. System must not just organize evidence, but:
   - Prove ALL allegations
   - Quantify HARM to Marie
   - Generate COURT-READY output

### Impacts
- All Roles: Shift from "reference tool" to "litigation engine"
- Phase 4 (Court Output) elevated in priority

### Action Required
- [ ] All: Ensure every feature supports litigation goals
- [ ] Data Architect: Harm tracking added (see above)

---

## 2026-01-14 | PROJECT MANAGEMENT (Roman)

### Decision
Established multi-role coordination structure with three specialized Claude personas.

### Rationale
- Context management: Each role has focused scope, preventing context overflow
- Clear authority: Data model decisions cascade down, preventing conflicts
- Parallel work: Roles can operate concurrently with defined interfaces

### Roles Defined
1. **Senior Data Architect** - Schema, data model, use cases
2. **Senior DB Engineer** - Neo4j, Cypher, document processing
3. **Senior Software Architect** - Rust/Axum, React, implementation

### Authority Hierarchy
```
Roman (Product Owner) → Data Architect → DB Engineer → Software Architect
```

### Impacts
- Data Architect: Now owns architecture/ folder and schema decisions
- DB Engineer: Now owns database/ folder, adapts to schema changes
- Software Architect: Now owns development/ folder, adapts to both above

### Action Required
- [x] Create COORDINATION.md
- [x] Create DECISION_LOG.md
- [x] Create PROJECT_ROADMAP.md
- [ ] Data Architect: Review and formalize DATA_MODEL_v2.md into v3
- [ ] DB Engineer: Document current Neo4j state (81 nodes, 180 relationships)
- [ ] Software Architect: Review current codebase state

---

## 2026-01-14 | DB ENGINEER (Prior Session - Documented Retroactively)

### Decision
Completed evidence chains for 8 of 18 complaint allegations in Neo4j.

### Rationale
Prioritized allegations with strongest evidence from CFS and Phillips interrogatory responses.

### Current Neo4j State
- **Nodes:** 81 total
  - Evidence: 29
  - ComplaintAllegation: 18
  - MotionClaim: 12
  - Person: 7
  - Document: 5
  - LegalCount: 4
  - Organization: 3
  - Case: 2
  - Event: 1

- **Relationships:** 180 total
  - RELIES_ON: 29
  - CONTAINED_IN: 29
  - INVOLVES: 28
  - SUPPORTS: 26
  - IN_CASE: 22
  - APPEARS_IN: 14
  - PROVES: 12
  - Others: 20

### Completed Allegations
| ID | Title | Counts Supported |
|----|-------|------------------|
| complaint-001 | Undisclosed CFS-Court Contract | I |
| complaint-005 | $50K Conversion by Sisters | I, II |
| complaint-007 | Estate Was Unnecessary | I, II |
| complaint-011 | Auction Caused $6K Loss | I, IV |
| complaint-015 | Selective Sanctions | I, IV |
| complaint-016 | 100% Costs to Marie | I, IV |
| complaint-017 | MCL 700.1212 Violation | I |
| complaint-018 | CFS Ultra Vires | III |

### Impacts
- Data Architect: None - implemented existing model
- DB Engineer: Continue with remaining 10 allegations
- Software Architect: Traceability queries available for API

### Action Required
- [ ] DB Engineer: Populate remaining 10 allegations
- [ ] DB Engineer: Add court documents (Judge Tighe, COA rulings) as evidence
- [ ] DB Engineer: Implement Harm nodes per 2026-01-19 decision

---

## Template for Future Entries

```markdown
## YYYY-MM-DD | [ROLE]

### Decision
[Clear statement of what was decided]

### Rationale
[Why this decision was made]

### Impacts
- Data Architect: [impact or "None"]
- DB Engineer: [impact or "None"]
- Software Architect: [impact or "None"]

### Action Required
- [ ] [Role]: [Specific action needed]
```

---

## Quick Reference: What Requires Logging?

**Always Log:**
- Schema changes (new node types, relationships, properties)
- API contract changes
- Query pattern changes that affect API
- Processing methodology changes
- Authority/process changes

**Don't Log:**
- Routine data entry following established patterns
- Bug fixes that don't change interfaces
- Documentation updates within owned files
- Exploratory work that doesn't result in decisions
