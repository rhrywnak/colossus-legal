# Colossus-Legal – Data Model (Neo4j Graph)

This document defines the **graph schema** for Colossus-Legal in Neo4j.

---

## 1. Node Types

### 1.1 Claim

Represents a legal claim (allegation, assertion, or issue).

**Label:** `Claim`

**Properties (initial):**

- `id: string` (UUID or stable string id)
- `title: string`
- `description: string?`
- `status: string` (`open | closed | refuted | pending`)
- `created_at: datetime`
- `updated_at: datetime`

---

### 1.2 Document

Represents a source document (filing, transcript, exhibit, etc.).

**Label:** `Document`

**Properties:**

- `id: string`
- `title: string`
- `type: string` (e.g., `complaint`, `transcript`, `exhibit`)
- `file_path: string?` (local path or storage key)
- `created_at: datetime`
- `ingested_at: datetime?`

---

### 1.3 Evidence

Represents a specific piece of evidence (excerpt, object, statement).

**Label:** `Evidence`

**Properties:**

- `id: string`
- `summary: string`
- `kind: string` (e.g., `testimonial`, `documentary`, `physical`)
- `weight: int?` (subjective scoring)
- `created_at: datetime`

---

### 1.4 Person

Represents a person relevant to the case.

**Label:** `Person`

**Properties:**

- `id: string`
- `name: string`
- `role: string` (e.g., `plaintiff`, `defendant`, `witness`, `judge`)
- `created_at: datetime`

---

### 1.5 Hearing

Represents a hearing or court event.

**Label:** `Hearing`

**Properties:**

- `id: string`
- `date: date`
- `location: string?`
- `description: string?`

---

### 1.6 Decision

Represents a decision, ruling, or order.

**Label:** `Decision`

**Properties:**

- `id: string`
- `title: string`
- `issued_at: date`
- `text: string?`
- `outcome: string?`

---

## 2. Relationships

All relationships are **directed** and have semantic meaning.

### 2.1 APPEARS_IN

**Pattern:** `(c:Claim)-[:APPEARS_IN]->(d:Document)`

Meaning: the claim appears in, or is asserted within, the given document.

---

### 2.2 RELIES_ON

**Pattern:** `(c:Claim)-[:RELIES_ON]->(e:Evidence)`

Meaning: the claim relies on the specified evidence.

---

### 2.3 PRESENTED_AT

**Pattern:** `(e:Evidence)-[:PRESENTED_AT]->(h:Hearing)`

Meaning: the evidence was presented at a specific hearing.

---

### 2.4 MADE_BY

**Pattern:** `(c:Claim)-[:MADE_BY]->(p:Person)`

Meaning: the claim was made by a particular person (e.g., witness, party).

---

### 2.5 DECIDES

**Pattern:** `(d:Decision)-[:DECIDES]->(c:Claim)`

Meaning: the decision resolves or addresses a specific claim.

---

### 2.6 REFUTES

**Pattern:** `(d:Decision)-[:REFUTES]->(c:Claim)`

Meaning: the decision explicitly refutes (rejects) a claim.

---

### 2.7 IGNORES

**Pattern:** `(d:Decision)-[:IGNORES]->(c:Claim)`

Meaning: the decision fails to address a claim that arguably should have been considered.

---

## 3. Example Subgraph

Example: a claim made by a witness, appearing in a complaint, supported by evidence presented at a hearing, and refuted in a decision:

```cypher
(c:Claim {id: "claim-1"})-[:MADE_BY]->(p:Person {id: "person-1"})
(c)-[:APPEARS_IN]->(doc:Document {id: "doc-1"})
(c)-[:RELIES_ON]->(e:Evidence {id: "ev-1"})
(e)-[:PRESENTED_AT]->(h:Hearing {id: "hearing-1"})
(dec:Decision {id: "dec-1"})-[:REFUTES]->(c)
(dec)-[:DECIDES]->(c)
```

This structure allows traversals like:

- “Show all evidence that supports claim X and where it was presented.”
- “List all claims ignored by decision Y.”
- “Show timeline: Claim creation → Evidence → Hearing → Decision.”

---

## 4. Versioning and Evolution

The schema is expected to evolve:

- Add optional properties (e.g., confidence scores, AI annotations).
- Add AI-related nodes:
  - `AISuggestion`
  - `Annotation`
- Add relationships like:
  - `SUGGESTS` (AISuggestion → Claim/Evidence)
  - `DERIVED_FROM` (AISuggestion → Document/Evidence)

Any schema changes **must** be reflected here and in implementation notes.

---

# End of DATA_MODEL.md
