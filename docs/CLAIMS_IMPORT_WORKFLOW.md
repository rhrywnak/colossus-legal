# Claims Import Workflow Specification

## Overview

This document specifies the workflow for importing Claude-extracted claims JSON files into the Colossus-Legal Neo4j database.

---

## Workflow Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        CLAIMS IMPORT WORKFLOW                               │
└─────────────────────────────────────────────────────────────────────────────┘

     ┌──────────────┐
     │  User uploads│
     │  claims.json │
     └──────┬───────┘
            │
            ▼
┌───────────────────────────────────────┐
│  STAGE 1: FILE VALIDATION             │
├───────────────────────────────────────┤
│  □ Valid JSON syntax                  │
│  □ Required top-level fields present  │
│  □ Schema version compatible          │
└───────────────────────────────────────┘
            │
            ▼ Pass
┌───────────────────────────────────────┐
│  STAGE 2: DATA VALIDATION             │
├───────────────────────────────────────┤
│  □ All claims have required fields    │
│  □ No duplicate claim IDs             │
│  □ Valid category values              │
│  □ Severity in range (1-10)           │
│  □ Line numbers are integers          │
│  □ Parties array not empty            │
│  □ Quote field not empty              │
└───────────────────────────────────────┘
            │
            ▼ Pass
┌───────────────────────────────────────┐
│  STAGE 3: REFERENCE RESOLUTION        │
├───────────────────────────────────────┤
│  □ Resolve person names → Person IDs  │
│  □ Resolve document → Document ID     │
│  □ Resolve evidence refs → Evidence   │
│  □ Flag unresolved references         │
└───────────────────────────────────────┘
            │
            ▼
┌───────────────────────────────────────┐
│  STAGE 4: DUPLICATE DETECTION         │
├───────────────────────────────────────┤
│  □ Check for existing claims (by ID)  │
│  □ Check for similar quotes (fuzzy)   │
│  □ Flag potential duplicates          │
└───────────────────────────────────────┘
            │
            ▼
┌───────────────────────────────────────┐
│  STAGE 5: IMPORT PREVIEW              │
├───────────────────────────────────────┤
│  Generate summary:                    │
│  - New claims to create: N            │
│  - New persons to create: N           │
│  - New evidence to create: N          │
│  - Relationships to create: N         │
│  - Warnings: N                        │
└───────────────────────────────────────┘
            │
            ▼
    ┌───────┴───────┐
    │  MODE SELECT  │
    └───────┬───────┘
            │
    ┌───────┴───────┐
    │               │
    ▼               ▼
┌────────┐    ┌──────────────┐
│  AUTO  │    │  SUPERVISED  │
│  MODE  │    │     MODE     │
└────┬───┘    └──────┬───────┘
     │               │
     │               ▼
     │         ┌───────────────────┐
     │         │ For each entity:  │
     │         │ - Show preview    │
     │         │ - [Approve/Skip]  │
     │         │ - [Edit] option   │
     │         └─────────┬─────────┘
     │                   │
     ▼                   ▼
┌───────────────────────────────────────┐
│  STAGE 6: NEO4J IMPORT                │
├───────────────────────────────────────┤
│  Transaction per claim:               │
│  1. MERGE Case node                   │
│  2. MERGE Document node               │
│  3. MERGE Person nodes                │
│  4. CREATE Claim node                 │
│  5. CREATE relationships              │
│  6. MERGE Evidence nodes              │
└───────────────────────────────────────┘
            │
            ▼
┌───────────────────────────────────────┐
│  STAGE 7: IMPORT REPORT               │
├───────────────────────────────────────┤
│  - Claims created: N                  │
│  - Claims skipped: N                  │
│  - Persons created: N                 │
│  - Relationships created: N           │
│  - Errors: N                          │
│  - Warnings: N                        │
└───────────────────────────────────────┘
```

---

## Stage Details

### Stage 1: File Validation

**Endpoint:** `POST /api/claims/import/validate`

**Input:** Multipart form with JSON file

**Checks:**
```rust
struct FileValidation {
    is_valid_json: bool,
    has_document_section: bool,
    has_claims_array: bool,
    has_parties_section: bool,
    schema_version: Option<String>,
    claim_count: usize,
}
```

**Response:**
```json
{
  "valid": true,
  "claim_count": 36,
  "document_title": "Motion for Default...",
  "errors": [],
  "warnings": []
}
```

---

### Stage 2: Data Validation

**Checks per claim:**

| Field | Validation Rule |
|-------|-----------------|
| `id` | Required, string, unique within file |
| `quote` | Required, non-empty string |
| `category` | Required, must be valid enum value |
| `severity` | Optional, if present must be 1-10 |
| `claim_type` | Optional, must be valid enum value |
| `status` | Optional, must be valid enum value (open, closed, refuted, pending) |
| `made_by` | Required, non-empty string (person ID) |
| `against` | Required, non-empty array of person IDs |
| `source.document_id` | Required, non-empty string |
| `source.document_title` | Required, non-empty string |
| `source.document_type` | Required, valid document type enum |
| `source.line_start` | Optional, if present must be positive int |
| `source.line_end` | Optional, if present must be >= line_start |
| `source.is_primary_source` | Optional, boolean |

**Additional file-level validations:**

| Field | Validation Rule |
|-------|-----------------|
| `schema_version` | Required, must be "2.1" or compatible |
| `source_document.id` | Required, matches claims' source.document_id |
| `source_document.doc_type` | Required, valid document type |
| `case.id` | Required, non-empty string |
| `parties.plaintiff` | Required, valid person object |
| `parties.defendants` | Required, non-empty array |

**Valid categories:**
```rust
enum ClaimCategory {
    Conversion,
    Fraud,
    BreachOfFiduciaryDuty,
    Defamation,
    Bias,
    DiscoveryObstruction,
    Perjury,
    Collusion,
    FinancialHarm,
    ProceduralMisconduct,
    ConflictOfInterest,
    UnauthorizedPossession,
    ImpartialityViolation,
    // ... extensible
}
```

---

### Stage 3: Reference Resolution

**Person Resolution:**
```
"plaintiff" → Look up Person with role="plaintiff" in this case
"Marie Awad" → Look up Person by name
"Camille Hanley" → Look up or create Person
```

**Document Resolution:**
```
document.title → Look up Document by title
               → Or create new Document node
```

**Evidence Resolution:**
```
"Exhibit 1" → Look up Evidence by exhibit_number
           → Or create placeholder Evidence node
```

**Output:**
```json
{
  "resolved_persons": {
    "plaintiff": "person-marie-awad",
    "Camille Hanley": "person-camille-hanley"
  },
  "new_persons": ["George Phillips", "Milton Higgs"],
  "resolved_evidence": {
    "Exhibit 1": "evidence-001"
  },
  "new_evidence": ["Exhibit 36", "Exhibit 76"]
}
```

---

### Stage 4: Duplicate Detection

**By ID:**
```cypher
MATCH (c:Claim {id: $claim_id})
RETURN c IS NOT NULL AS exists
```

**By Quote (Fuzzy):**
```cypher
MATCH (c:Claim)
WHERE apoc.text.jaroWinklerDistance(c.quote, $quote) > 0.85
RETURN c.id, c.quote, apoc.text.jaroWinklerDistance(c.quote, $quote) AS similarity
```

**Output:**
```json
{
  "exact_duplicates": [],
  "potential_duplicates": [
    {
      "new_claim_id": "CLAIM-005",
      "existing_claim_id": "CLAIM-OLD-123",
      "similarity": 0.92,
      "recommendation": "review"
    }
  ]
}
```

---

### Stage 5: Import Preview

**Summary Response:**
```json
{
  "import_id": "import-2025-12-20-001",
  "source_file": "Awad_v_Catholic_Family_Motion_for_Default_claims.json",
  "preview": {
    "case": {
      "action": "merge",
      "id": "awad-v-cfs",
      "name": "Marie Awad v. Catholic Family Service"
    },
    "document": {
      "action": "create",
      "title": "Motion for Default and Summary Disposition"
    },
    "claims": {
      "total": 36,
      "new": 36,
      "duplicates": 0,
      "skipped": 0
    },
    "persons": {
      "total": 12,
      "existing": 3,
      "new": 9
    },
    "evidence": {
      "total": 22,
      "existing": 0,
      "new": 22
    },
    "relationships": {
      "APPEARS_IN": 36,
      "MADE_BY": 36,
      "AGAINST": 58,
      "RELIES_ON": 45,
      "IN_CASE": 36
    }
  },
  "warnings": [
    "Person 'Milton Higgs' not found - will be created",
    "Evidence 'Exhibit 36' not found - will be created as placeholder"
  ],
  "ready_to_import": true
}
```

---

### Stage 6: Neo4j Import

**Auto Mode:**
- Execute all imports in sequence
- Rollback on error
- Return final report

**Supervised Mode:**
- Present each entity for approval
- Allow edits before import
- Skip individual items
- Batch approve remaining

**API Endpoints:**

```
POST /api/claims/import/auto
  Body: { import_id: string }
  
POST /api/claims/import/supervised/start
  Body: { import_id: string }
  Response: { session_id: string, first_item: ImportItem }

POST /api/claims/import/supervised/approve
  Body: { session_id: string, item_id: string }
  
POST /api/claims/import/supervised/skip
  Body: { session_id: string, item_id: string }
  
POST /api/claims/import/supervised/edit
  Body: { session_id: string, item_id: string, updates: {...} }
  
POST /api/claims/import/supervised/approve-all
  Body: { session_id: string }
```

---

### Stage 7: Import Report

**Response:**
```json
{
  "import_id": "import-2025-12-20-001",
  "status": "completed",
  "duration_ms": 1250,
  "results": {
    "case": { "created": 0, "updated": 1 },
    "document": { "created": 1 },
    "claims": { "created": 36, "skipped": 0, "errors": 0 },
    "persons": { "created": 9, "existing": 3 },
    "evidence": { "created": 22 },
    "relationships": { "created": 211 }
  },
  "errors": [],
  "warnings": [
    "Evidence 'Exhibit 99' referenced but not in exhibits list"
  ],
  "neo4j_stats": {
    "nodes_created": 68,
    "relationships_created": 211,
    "properties_set": 412
  }
}
```

---

## Data Structures

### ImportItem (for supervised mode)

```typescript
interface ImportItem {
  item_id: string;
  item_type: "case" | "document" | "person" | "claim" | "evidence" | "relationship";
  action: "create" | "merge" | "update";
  data: Record<string, any>;
  preview_cypher: string;  // The Cypher that will be executed
  depends_on: string[];    // IDs of items that must be imported first
}
```

### ValidationError

```typescript
interface ValidationError {
  claim_id: string;
  field: string;
  error_type: "missing" | "invalid" | "duplicate" | "out_of_range";
  message: string;
  value?: any;
}
```

---

## UI Mockup (Supervised Mode)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Import Claims: Motion for Default...                          [X] Close   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Progress: ████████░░░░░░░░░░░░  12 / 36 claims                            │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  CLAIM-012: Discovery Obstruction                                   │   │
│  │                                                                     │   │
│  │  Quote:                                                             │   │
│  │  "Interrogatory 17 specifically asked to provide a complete list   │   │
│  │  of cases in which the Defendant previously represented Catholic   │   │
│  │  Family Service. The Defendant simply ignores the question..."     │   │
│  │                                                                     │   │
│  │  Category: discovery_obstruction    Severity: 6                     │   │
│  │  Made by: Marie Awad (plaintiff)                                    │   │
│  │  Against: George Phillips                                           │   │
│  │  Source: Lines 361-366                                              │   │
│  │                                                                     │   │
│  │  Will create:                                                       │   │
│  │  • Claim node (CLAIM-012)                                          │   │
│  │  • APPEARS_IN → Motion for Default                                  │   │
│  │  • MADE_BY → Marie Awad                                            │   │
│  │  • AGAINST → George Phillips                                        │   │
│  │  • RELIES_ON → Exhibit 7 (will be created)                         │   │
│  │                                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  [✓ Approve]  [✗ Skip]  [✎ Edit]  [▶▶ Approve All Remaining]              │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Implementation Phases

### Phase A: Backend API (Rust)

1. `POST /api/claims/import/upload` - File upload + Stage 1-2
2. `POST /api/claims/import/preview` - Stage 3-5
3. `POST /api/claims/import/auto` - Stage 6 (auto)
4. `POST /api/claims/import/supervised/*` - Stage 6 (supervised)
5. `GET /api/claims/import/{id}/report` - Stage 7

### Phase B: Frontend UI (React)

1. File upload component with drag-drop
2. Validation results display
3. Preview summary page
4. Mode selection (auto/supervised)
5. Supervised approval interface
6. Import progress + report display

### Phase C: Neo4j Integration

1. Cypher generation from claims JSON
2. Transaction management
3. Rollback on error
4. Duplicate detection queries
5. Fuzzy matching for similar claims

---

## Error Handling

| Error Type | Handling |
|------------|----------|
| Invalid JSON | Return 400 with parse error location |
| Missing required field | Include in validation errors, block import |
| Duplicate claim ID | Flag in preview, allow user to skip |
| Neo4j connection error | Retry 3x, then fail with clear message |
| Constraint violation | Rollback transaction, report which constraint |
| Timeout | Rollback, suggest smaller batch |

---

## Configuration

```toml
[import]
max_file_size_mb = 10
max_claims_per_file = 500
duplicate_similarity_threshold = 0.85
transaction_timeout_seconds = 30
batch_size = 50  # Claims per transaction in auto mode

[validation]
required_claim_fields = ["id", "quote", "category", "made_by", "against"]
valid_categories = ["conversion", "fraud", "breach_of_fiduciary_duty", ...]
severity_range = [1, 10]
```

---

# End of CLAIMS_IMPORT_WORKFLOW.md
