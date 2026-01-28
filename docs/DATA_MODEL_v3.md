# Colossus-Legal Data Model v3

**Version:** 3.0  
**Date:** 2026-01-19  
**Change:** Added Harm node type for damages tracking

---

## Overview

This data model supports a legal document analysis system designed to:
1. **Prove** all allegations in the Awad v. CFS civil lawsuit
2. **Quantify** harm caused to Marie Awad
3. **Generate** court-ready evidence chains

---

## Node Types

### 1. Case
Represents a legal proceeding.

```
Case {
  id: string,              // "awad-v-cfs"
  name: string,            // "Marie Awad v. Catholic Family Service et al."
  case_number: string,     // Court case number
  court: string,           // "Bay County Circuit Court"
  filed_date: date,
  status: string           // "active", "closed", "appealed"
}
```

### 2. LegalCount
A cause of action (legal theory) in the lawsuit.

```
LegalCount {
  id: string,              // "count-fraud"
  count_number: integer,   // 1, 2, 3, 4
  title: string,           // "Fraud"
  legal_basis: string,     // "Common Law" or statute citation
  paragraphs: string,      // "86-100" (complaint paragraphs)
  key_elements: string,    // Elements plaintiff must prove
  damages_sought: string   // "Exceeding $25,000"
}
```

### 3. ComplaintAllegation
A specific factual allegation from the complaint.

```
ComplaintAllegation {
  id: string,              // "complaint-005"
  paragraph: string,       // "16-18"
  title: string,           // "$50,000 Conversion by Sisters"
  allegation: string,      // Summary of the allegation
  verbatim: string,        // Exact text from complaint
  evidence_status: string, // "PROVEN", "PARTIAL", "UNPROVEN"
  category: string,        // "financial", "procedural", "defamation"
  severity: integer        // 1-10 scale
}
```

### 4. MotionClaim
A synthesized argument from the Motions for Default.

```
MotionClaim {
  id: string,              // "cfs-default-005-conversion"
  title: string,           // "CFS Motion: $50K Conversion"
  claim_text: string,      // The argument being made
  source_document_id: string,
  category: string,        // "admission", "argument", "evidence_summary"
  significance: string     // Why this matters
}
```

### 5. Evidence
A piece of evidence supporting a claim (typically sworn testimony).

```
Evidence {
  id: string,              // "evidence-phillips-q73"
  exhibit_number: string,  // "Phillips Interrogatory Q73"
  title: string,           // Descriptive title
  question: string,        // The question asked
  answer: string,          // The sworn answer
  kind: string,            // "testimonial", "documentary", "physical"
  weight: integer,         // 1-10 (10 = most probative)
  page_number: integer,    // Page in source document
  significance: string     // Why this evidence matters
}
```

### 6. Document
A source document in the case.

```
Document {
  id: string,              // "doc-phillips-discovery-response"
  title: string,           // "George Phillips Response to Discovery"
  document_type: string,   // "complaint", "discovery", "motion", "ruling"
  date: date,
  source_file: string,     // Original filename
  page_count: integer
}
```

### 7. Person
An individual involved in the case.

```
Person {
  id: string,              // "marie-awad"
  name: string,            // "Marie Awad"
  role: string,            // "plaintiff", "defendant", "witness", "attorney"
  description: string      // Additional context
}
```

### 8. Organization
An entity involved in the case.

```
Organization {
  id: string,              // "catholic-family-service"
  name: string,            // "Catholic Family Service"
  role: string,            // "defendant", "personal_representative"
  description: string
}
```

### 9. Harm (NEW in v3)
A specific harm or damage suffered by Marie.

```
Harm {
  id: string,              // "harm-001"
  title: string,           // "100% Appellate Costs Assessed to Marie"
  category: string,        // "financial_direct", "financial_estate", "reputational"
  subcategory: string,     // "sanction", "incompetence", "character_attack"
  amount: float | null,    // Dollar amount if quantifiable
  description: string,     // Detailed description of the harm
  date: date | null,       // When harm occurred
  source_reference: string // Document/page where harm is documented
}
```

**Harm Categories:**
| Category | Subcategory | Description |
|----------|-------------|-------------|
| `financial_direct` | `sanction` | Money taken directly from Marie via court order |
| `financial_estate` | `incompetence` | Estate losses reducing Marie's inheritance |
| `financial_estate` | `unnecessary_cost` | Costs that should not have been incurred |
| `reputational` | `character_attack` | False statements damaging Marie's reputation |
| `reputational` | `discriminatory` | Unequal treatment compared to sisters |

### 10. Event (Optional)
A significant event in the case timeline.

```
Event {
  id: string,
  title: string,
  date: date,
  description: string,
  event_type: string       // "filing", "hearing", "ruling", "action"
}
```

---

## Relationships

### Core Evidence Chain (Existing)

| Relationship | From | To | Meaning |
|--------------|------|-----|---------|
| `SUPPORTS` | ComplaintAllegation | LegalCount | Allegation supports this count |
| `PROVES` | MotionClaim | ComplaintAllegation | Claim proves this allegation |
| `RELIES_ON` | MotionClaim | Evidence | Claim relies on this evidence |
| `CONTAINED_IN` | Evidence | Document | Evidence found in this document |
| `APPEARS_IN` | MotionClaim | Document | Claim appears in this document |

### Harm Relationships (NEW in v3)

| Relationship | From | To | Meaning |
|--------------|------|-----|---------|
| `CAUSED_BY` | Harm | ComplaintAllegation | This harm resulted from this misconduct |
| `EVIDENCED_BY` | Harm | Evidence | This evidence proves the harm occurred |
| `DAMAGES_FOR` | Harm | LegalCount | This harm supports damages for this count |

### Entity Relationships

| Relationship | From | To | Meaning |
|--------------|------|-----|---------|
| `INVOLVES` | Evidence | Person/Organization | Evidence involves this party |
| `PARTY_TO` | Person/Organization | Case | Party to this case |
| `IN_CASE` | Various | Case | Node belongs to this case |
| `RELATED_TO` | Person | Person | Family or professional relationship |
| `REPRESENTED_BY` | Organization | Person | Organization represented by attorney |

---

## Visual Data Model

```
                                    ┌─────────────┐
                                    │    Case     │
                                    └──────┬──────┘
                                           │ [:IN_CASE]
                    ┌──────────────────────┼──────────────────────┐
                    │                      │                      │
                    ▼                      ▼                      ▼
            ┌─────────────┐        ┌─────────────┐        ┌─────────────┐
            │ LegalCount  │        │   Person    │        │Organization │
            └──────┬──────┘        └─────────────┘        └─────────────┘
                   │
    ┌──────────────┼──────────────┐
    │ [:SUPPORTS]  │              │ [:DAMAGES_FOR]
    ▼              │              ▼
┌─────────────────────┐    ┌─────────────┐
│ ComplaintAllegation │    │    HARM     │ ◄── NEW in v3
└──────────┬──────────┘    └──────┬──────┘
           │                      │
           │ [:CAUSED_BY]─────────┘
           │
           │ [:PROVES]
           ▼
┌─────────────────────┐
│    MotionClaim      │
└──────────┬──────────┘
           │
           │ [:RELIES_ON]
           ▼
┌─────────────────────┐    [:EVIDENCED_BY]
│      Evidence       │◄─────────────────── (from Harm)
└──────────┬──────────┘
           │
           │ [:CONTAINED_IN]
           ▼
┌─────────────────────┐
│      Document       │
└─────────────────────┘
```

---

## Query Patterns

### 1. Full Evidence Chain for an Allegation
```cypher
MATCH path = (count:LegalCount)<-[:SUPPORTS]-(allegation:ComplaintAllegation)
             <-[:PROVES]-(motion:MotionClaim)-[:RELIES_ON]->(evidence:Evidence)
WHERE allegation.id = "complaint-005"
RETURN count.title, allegation.title, motion.title, evidence.answer;
```

### 2. All Harms for a Count
```cypher
MATCH (count:LegalCount)<-[:DAMAGES_FOR]-(harm:Harm)
WHERE count.id = "count-fraud"
RETURN harm.title, harm.category, harm.amount;
```

### 3. Total Quantifiable Damages
```cypher
MATCH (harm:Harm)
WHERE harm.amount IS NOT NULL
RETURN sum(harm.amount) as total_damages,
       collect({title: harm.title, amount: harm.amount}) as breakdown;
```

### 4. Harm with Supporting Evidence
```cypher
MATCH (harm:Harm)-[:EVIDENCED_BY]->(evidence:Evidence)
MATCH (harm)-[:CAUSED_BY]->(allegation:ComplaintAllegation)
WHERE harm.id = "harm-001"
RETURN harm.title, harm.amount, allegation.title, evidence.answer;
```

### 5. Evidence Supporting a Count (Including Harm)
```cypher
// Direct evidence chain
MATCH (count:LegalCount)<-[:SUPPORTS]-(a)<-[:PROVES]-(m)-[:RELIES_ON]->(e:Evidence)
WHERE count.id = "count-fraud"
RETURN DISTINCT e.title, e.answer

UNION

// Evidence supporting harms for this count
MATCH (count:LegalCount)<-[:DAMAGES_FOR]-(h:Harm)-[:EVIDENCED_BY]->(e:Evidence)
WHERE count.id = "count-fraud"
RETURN DISTINCT e.title, e.answer;
```

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2024-10 | Initial model |
| 2.0 | 2025-01 | Added Organization as separate node type |
| 3.0 | 2026-01-19 | Added Harm node type for damages tracking |

---

## Migration Notes (v2 → v3)

**New Node Type:**
- Add `Harm` label to schema
- Create Harm nodes for identified damages

**New Relationships:**
- `CAUSED_BY`: Harm → ComplaintAllegation
- `EVIDENCED_BY`: Harm → Evidence  
- `DAMAGES_FOR`: Harm → LegalCount

**No Breaking Changes:**
- All existing nodes and relationships remain valid
- New Harm layer is additive
