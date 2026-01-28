# Colossus-Legal: Neo4j Graph Architecture

## Current Node Schema (from DATA_MODEL.md)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         NEO4J GRAPH SCHEMA                                  │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────┐         ┌─────────────────┐         ┌─────────────────┐
│     Person      │         │      Claim      │         │    Document     │
├─────────────────┤         ├─────────────────┤         ├─────────────────┤
│ id: string      │         │ id: string      │         │ id: string      │
│ name: string    │◄────────│ title: string   │────────►│ title: string   │
│ role: string    │ MADE_BY │ description?    │APPEARS  │ type: string    │
│ created_at: dt  │         │ status: string  │  _IN    │ file_path?      │
└─────────────────┘         │ created_at: dt  │         │ created_at: dt  │
                            │ updated_at: dt  │         │ ingested_at?: dt│
                            └────────┬────────┘         └─────────────────┘
                                     │
                                     │ RELIES_ON
                                     ▼
                            ┌─────────────────┐
                            │    Evidence     │
                            ├─────────────────┤
                            │ id: string      │
                            │ summary: string │
                            │ kind: string    │────────►┌─────────────────┐
                            │ weight?: int    │PRESENTED│     Hearing     │
                            │ created_at: dt  │  _AT    ├─────────────────┤
                            └─────────────────┘         │ id: string      │
                                                        │ date: date      │
                                                        │ location?       │
                            ┌─────────────────┐         │ description?    │
                            │    Decision     │         └─────────────────┘
                            ├─────────────────┤
                            │ id: string      │
                            │ title: string   │
                            │ issued_at: date │
                            │ text?           │
                            │ outcome?        │
                            └────────┬────────┘
                                     │
                    ┌────────────────┼────────────────┐
                    │ DECIDES        │ REFUTES        │ IGNORES
                    ▼                ▼                ▼
                 (Claim)          (Claim)          (Claim)


## Relationships Summary

┌──────────────────┬─────────────────────┬─────────────────────────────────────┐
│ Relationship     │ Pattern             │ Meaning                             │
├──────────────────┼─────────────────────┼─────────────────────────────────────┤
│ APPEARS_IN       │ (Claim)→(Document)  │ Claim is asserted in document       │
│ MENTIONS         │ (Document)→(Claim)  │ Document references a claim         │
│ MADE_BY          │ (Claim)→(Person)    │ Person made/asserted the claim      │
│ RELIES_ON        │ (Claim)→(Evidence)  │ Claim depends on this evidence      │
│ PRESENTED_AT     │ (Evidence)→(Hearing)│ Evidence shown at this hearing      │
│ DECIDES          │ (Decision)→(Claim)  │ Decision addresses this claim       │
│ REFUTES          │ (Decision)→(Claim)  │ Decision rejects this claim         │
│ IGNORES          │ (Decision)→(Claim)  │ Decision fails to address claim     │
└──────────────────┴─────────────────────┴─────────────────────────────────────┘
```

## Visual Graph Example

```
                                    ┌──────────────┐
                                    │   Person     │
                                    │ "Marie Awad" │
                                    │ role:plaintiff│
                                    └──────┬───────┘
                                           │
                                      MADE_BY
                                           │
┌──────────────────┐               ┌───────▼───────┐               ┌──────────────────┐
│    Document      │◄──APPEARS_IN──│    Claim      │──RELIES_ON───►│    Evidence      │
│ "Motion for     │               │ "$50K stolen" │               │ "Bank records"   │
│  Default"        │               │ severity: 9   │               │ kind: documentary│
│ type: motion     │               │ status: open  │               └────────┬─────────┘
└──────────────────┘               └───────┬───────┘                        │
                                           │                           PRESENTED_AT
                                           │                                │
                                      DECIDES                               ▼
                                           │                        ┌──────────────────┐
                                   ┌───────▼───────┐                │    Hearing       │
                                   │   Decision    │                │ "Oct 14, 2010"   │
                                   │ "Estate Order"│                │ location: Bay Cty│
                                   │ outcome: ...  │                └──────────────────┘
                                   └───────────────┘
```

---

## Step 2: Mapping Extracted Claims to Graph Structure

### Current Claim JSON Structure (from extraction)

```json
{
  "id": "CLAIM-001",
  "category": "conversion",
  "severity": 9,
  "claim_type": "factual_event",
  "quote": "Shortly before his death...",
  "line_start": 71,
  "line_end": 75,
  "made_by": "plaintiff",
  "against": ["Camille Hanley", "Nadia Awad"],
  "amount": "$50,000.00",
  "date_reference": "Shortly before May 4, 2009",
  "evidence_refs": ["Exhibit 3", "Exhibit 4"]
}
```

### Required Mapping

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    EXTRACTED JSON → NEO4J MAPPING                           │
└─────────────────────────────────────────────────────────────────────────────┘

JSON Field              Neo4j Node/Property           Notes
─────────────────────────────────────────────────────────────────────────────
id                  →   Claim.id                      Direct map
category            →   Claim.category (NEW)          Not in current schema!
severity            →   Claim.severity (NEW)          Not in current schema!
claim_type          →   Claim.claim_type (NEW)        Not in current schema!
quote               →   Claim.quote (NEW)             Not in current schema!
                        OR Claim.description          Repurpose existing?
line_start/end      →   Claim.line_start/end (NEW)   For grounding verification
made_by             →   Person + MADE_BY relationship Need to resolve "plaintiff"→Person
against             →   Person[] + AGAINST rel (NEW)  Not in current schema!
amount              →   Claim.amount (NEW)            Or separate Amount node?
date_reference      →   Claim.event_date (NEW)        Not in current schema!
evidence_refs       →   Evidence + RELIES_ON rel      Need to create Evidence nodes

document.title      →   Document node                 Source document
document.case       →   Case node (NEW?)              Not in current schema!
parties.plaintiff   →   Person (role: plaintiff)      
parties.defendants  →   Person[] (role: defendant)    
key_dates           →   Event nodes (NEW?)            Or Hearing nodes?
key_amounts         →   Amount nodes (NEW?)           Or claim properties?
evidence_exhibits   →   Evidence nodes                
```

---

## Step 3: Identified Gaps

### GAP 1: Claim Node Missing Properties

**Current Claim schema:**
```
- id, title, description, status, created_at, updated_at
```

**Needed from extraction:**
```
+ quote: string          # Verbatim text from document (CRITICAL for grounding)
+ category: string       # conversion, fraud, breach_of_fiduciary_duty, etc.
+ severity: int          # 1-10 scale
+ claim_type: string     # factual_event, legal_conclusion, etc.
+ line_start: int        # For source verification
+ line_end: int
+ amount: string?        # Dollar amounts referenced
+ event_date: string?    # When the alleged event occurred
```

### GAP 2: Missing AGAINST Relationship

**Current:** Claims have MADE_BY (who asserted the claim)
**Missing:** AGAINST (who the claim is against)

```
(c:Claim)-[:AGAINST]->(p:Person)
```

### GAP 3: No Case Node

The extraction includes case-level metadata:
- Case name: "Marie Awad v. Catholic Family Service and George Phillips"
- Court: "Bay County Circuit Court, Michigan"
- Case type

**Suggested:** Add a `Case` node to group related documents/claims:

```
(:Case {
  id: string,
  name: string,
  court: string,
  case_number?: string,
  filed_date?: date
})

(d:Document)-[:PART_OF]->(case:Case)
(c:Claim)-[:IN_CASE]->(case:Case)
```

### GAP 4: Evidence Node Needs Expansion

**Current Evidence:**
```
- id, summary, kind, weight, created_at
```

**Needed:**
```
+ exhibit_number: string   # "Exhibit 1", "Exhibit 36"
+ description: string      # "Orders Compelling Discovery"
+ document_ref?: string    # If it's a referenced document
```

### GAP 5: No Event/Timeline Node

Key dates in the case don't map cleanly to existing nodes:
- "May 4, 2009 - Emil Awad died"
- "July 23, 2009 - CFS appointed as Personal Representative"

These aren't Hearings or Decisions. Consider:

```
(:Event {
  id: string,
  date: date,
  description: string,
  event_type: string  # death, filing, appointment, etc.
})

(e:Event)-[:RELATES_TO]->(c:Claim)
(e:Event)-[:INVOLVES]->(p:Person)
```

### GAP 6: Claim-to-Claim Relationships

Some claims reference or build upon other claims. Consider:

```
(c1:Claim)-[:SUPPORTS]->(c2:Claim)
(c1:Claim)-[:CONTRADICTS]->(c2:Claim)
(c1:Claim)-[:RELATED_TO]->(c2:Claim)
```

---

## Summary of Required Schema Changes

### New Node Types

| Node | Purpose |
|------|---------|
| `Case` | Groups documents and claims for a legal case |
| `Event` | Timeline events that aren't hearings/decisions |

### Modified Node Properties

| Node | New Properties |
|------|----------------|
| `Claim` | `quote`, `category`, `severity`, `claim_type`, `line_start`, `line_end`, `amount`, `event_date` |
| `Evidence` | `exhibit_number`, `description` |

### New Relationships

| Relationship | Pattern | Purpose |
|--------------|---------|---------|
| `AGAINST` | `(Claim)-[:AGAINST]->(Person)` | Who the claim accuses |
| `PART_OF` | `(Document)-[:PART_OF]->(Case)` | Document belongs to case |
| `IN_CASE` | `(Claim)-[:IN_CASE]->(Case)` | Claim belongs to case |
| `RELATES_TO` | `(Event)-[:RELATES_TO]->(Claim)` | Event relevant to claim |
| `INVOLVES` | `(Event)-[:INVOLVES]->(Person)` | Person involved in event |
| `SUPPORTS` | `(Claim)-[:SUPPORTS]->(Claim)` | Claim supports another |

---

## Proposed Updated Schema

See `DATA_MODEL_v2.md` for the complete updated schema.
