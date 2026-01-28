# COLOSSUS-LEGAL Query Pattern Library

**Version:** 1.0  
**Created:** 2026-01-22  
**Database:** 122 nodes, 297 relationships  

A collection of reusable Cypher queries for litigation support in Marie Awad v. Catholic Family Service and George Phillips.

---

## Table of Contents

1. [Damages Queries](#1-damages-queries)
2. [Evidence Chain Queries](#2-evidence-chain-queries)
3. [Allegation Queries](#3-allegation-queries)
4. [Defendant-Specific Queries](#4-defendant-specific-queries)
5. [Legal Count Queries](#5-legal-count-queries)
6. [Court Presentation Queries](#6-court-presentation-queries)
7. [Document Queries](#7-document-queries)
8. [Graph Exploration Queries](#8-graph-exploration-queries)
9. [Validation Queries](#9-validation-queries)

---

## 1. Damages Queries

### 1.1 Total Quantifiable Damages
**Purpose:** Calculate total dollar amount of proven damages.

```cypher
MATCH (h:Harm) 
WHERE h.amount IS NOT NULL
RETURN sum(h.amount) as total_damages,
       count(h) as quantified_harms;
```

### 1.2 Damages by Category
**Purpose:** Break down damages by type (financial_direct, financial_estate, reputational).

```cypher
MATCH (h:Harm)
RETURN h.category, 
       count(*) as count, 
       sum(h.amount) as total,
       collect(h.title) as harms
ORDER BY total DESC;
```

### 1.3 Damages by Legal Count
**Purpose:** Show which harms support which counts and their totals.

```cypher
MATCH (h:Harm)-[:DAMAGES_FOR]->(c:LegalCount)
RETURN c.title as legal_count, 
       collect(h.title) as supporting_harms, 
       sum(h.amount) as total_damages
ORDER BY total_damages DESC;
```

### 1.4 Complete Damages Schedule
**Purpose:** Full listing for court presentation.

```cypher
MATCH (h:Harm)
OPTIONAL MATCH (h)-[:DAMAGES_FOR]->(c:LegalCount)
RETURN h.id, 
       h.title, 
       h.category,
       h.amount,
       h.description,
       collect(DISTINCT c.title) as supports_counts
ORDER BY h.category, h.id;
```

### 1.5 Damages with Evidence Sources
**Purpose:** Show damages with their evidentiary support.

```cypher
MATCH (h:Harm)-[:EVIDENCED_BY]->(e:Evidence)-[:CONTAINED_IN]->(d:Document)
RETURN h.title as harm,
       h.amount as amount,
       collect(DISTINCT e.exhibit_number) as evidence,
       collect(DISTINCT d.title) as source_documents
ORDER BY h.amount DESC;
```

---

## 2. Evidence Chain Queries

### 2.1 Full Traceability Chain
**Purpose:** Show complete chain from Count → Allegation → MotionClaim → Evidence → Document.

```cypher
MATCH (c:LegalCount)<-[:SUPPORTS]-(a:ComplaintAllegation)
      <-[:PROVES]-(m:MotionClaim)-[:RELIES_ON]->(e:Evidence)
      -[:CONTAINED_IN]->(d:Document)
RETURN c.title as count,
       a.id as allegation,
       m.title as motion_claim,
       e.exhibit_number as evidence,
       d.title as document
ORDER BY c.title, a.id;
```

### 2.2 Evidence by Weight (Strongest First)
**Purpose:** Find most impactful evidence for trial preparation.

```cypher
MATCH (e:Evidence)
WHERE e.weight IS NOT NULL
RETURN e.id, 
       e.title, 
       e.weight,
       e.exhibit_number,
       substring(e.answer, 0, 100) as answer_preview
ORDER BY e.weight DESC
LIMIT 15;
```

### 2.3 Evidence Supporting Multiple Allegations
**Purpose:** Find evidence that proves multiple claims (high-value evidence).

```cypher
MATCH (e:Evidence)<-[:RELIES_ON]-(m:MotionClaim)-[:PROVES]->(a:ComplaintAllegation)
WITH e, count(DISTINCT a) as allegation_count, collect(DISTINCT a.id) as allegations
WHERE allegation_count > 1
RETURN e.id, 
       e.title, 
       allegation_count,
       allegations
ORDER BY allegation_count DESC;
```

### 2.4 Evidence Chain for Specific Allegation
**Purpose:** Deep dive into one allegation's proof structure.
**Parameter:** Replace `complaint-005` with target allegation.

```cypher
MATCH (a:ComplaintAllegation {id: "complaint-005"})
OPTIONAL MATCH (a)<-[:PROVES]-(m:MotionClaim)-[:RELIES_ON]->(e:Evidence)
OPTIONAL MATCH (e)-[:CONTAINED_IN]->(d:Document)
OPTIONAL MATCH (a)-[:SUPPORTS]->(c:LegalCount)
RETURN a.id,
       a.allegation,
       collect(DISTINCT c.title) as counts,
       collect(DISTINCT {claim: m.title, evidence: e.exhibit_number, doc: d.title}) as proof_chain;
```

---

## 3. Allegation Queries

### 3.1 All Allegations with Status
**Purpose:** Overview of all 18 allegations and their proof status.

```cypher
MATCH (a:ComplaintAllegation)
OPTIONAL MATCH (m:MotionClaim)-[:PROVES]->(a)
RETURN a.id, 
       a.allegation,
       a.evidence_status,
       count(m) as motion_claims
ORDER BY a.id;
```

### 3.2 Allegations by Legal Count
**Purpose:** Which allegations support which counts.

```cypher
MATCH (a:ComplaintAllegation)-[:SUPPORTS]->(c:LegalCount)
RETURN c.title as legal_count,
       collect(a.id) as allegations,
       count(a) as count
ORDER BY count DESC;
```

### 3.3 Allegations with Verbatim Text
**Purpose:** Show exact complaint language for court filings.

```cypher
MATCH (a:ComplaintAllegation)
RETURN a.id,
       a.paragraph,
       a.allegation,
       a.verbatim
ORDER BY a.id;
```

### 3.4 Allegations Linked to Specific Harm
**Purpose:** Find which allegations caused specific damages.
**Parameter:** Replace `harm-001` with target harm.

```cypher
MATCH (h:Harm {id: "harm-001"})-[:CAUSED_BY]->(a:ComplaintAllegation)
RETURN h.title as harm,
       h.amount as amount,
       collect({id: a.id, allegation: a.allegation}) as causing_allegations;
```

---

## 4. Defendant-Specific Queries

### 4.1 All Phillips Admissions
**Purpose:** Every admission from George Phillips' discovery responses.

```cypher
MATCH (e:Evidence)
WHERE e.id STARTS WITH "evidence-phillips"
RETURN e.exhibit_number,
       e.title,
       e.question,
       e.answer,
       e.significance
ORDER BY e.exhibit_number;
```

### 4.2 All CFS Admissions
**Purpose:** Every admission from Catholic Family Service.

```cypher
MATCH (e:Evidence)
WHERE e.id STARTS WITH "evidence-cfs"
RETURN e.exhibit_number,
       e.title,
       e.question,
       e.answer,
       e.significance
ORDER BY e.exhibit_number;
```

### 4.3 Phillips Admissions About $50K
**Purpose:** Everything Phillips said about the $50,000 conversion.

```cypher
MATCH (e:Evidence)
WHERE e.id STARTS WITH "evidence-phillips"
  AND (e.answer CONTAINS "50,000" 
       OR e.answer CONTAINS "$50" 
       OR e.answer CONTAINS "gift"
       OR e.answer CONTAINS "video"
       OR e.significance CONTAINS "50")
RETURN e.exhibit_number, e.question, e.answer, e.significance;
```

### 4.4 CFS Admissions About Contract with Court
**Purpose:** Evidence of undisclosed financial relationship.

```cypher
MATCH (e:Evidence)
WHERE e.id STARTS WITH "evidence-cfs"
  AND (e.answer CONTAINS "contract" 
       OR e.answer CONTAINS "revenue"
       OR e.answer CONTAINS "probate court"
       OR e.significance CONTAINS "contract")
RETURN e.exhibit_number, e.question, e.answer, e.significance;
```

### 4.5 Defendant Evasive Responses
**Purpose:** Find "documents speak for themselves" and similar non-answers.

```cypher
MATCH (e:Evidence)
WHERE e.answer CONTAINS "speak for themselves"
   OR e.answer CONTAINS "no specific recollection"
   OR e.answer CONTAINS "no recollection"
   OR e.answer CONTAINS "previously produced"
RETURN e.id, e.exhibit_number, e.answer, e.significance;
```

---

## 5. Legal Count Queries

### 5.1 Count Summary with Support
**Purpose:** Overview of all four counts with supporting allegations and harms.

```cypher
MATCH (c:LegalCount)
OPTIONAL MATCH (a:ComplaintAllegation)-[:SUPPORTS]->(c)
OPTIONAL MATCH (h:Harm)-[:DAMAGES_FOR]->(c)
RETURN c.id,
       c.title,
       count(DISTINCT a) as allegations,
       count(DISTINCT h) as harms,
       collect(DISTINCT a.id) as allegation_ids
ORDER BY c.id;
```

### 5.2 Count I: Breach of Fiduciary Duty - Full Evidence
**Purpose:** Complete evidence package for Count I.

```cypher
MATCH (c:LegalCount {id: "count-breach-fiduciary-duty"})
OPTIONAL MATCH (a:ComplaintAllegation)-[:SUPPORTS]->(c)
OPTIONAL MATCH (m:MotionClaim)-[:PROVES]->(a)
OPTIONAL MATCH (m)-[:RELIES_ON]->(e:Evidence)
RETURN c.title as count,
       a.id as allegation,
       a.allegation as claim,
       m.title as motion_claim,
       e.exhibit_number as evidence,
       e.answer as admission
ORDER BY a.id;
```

### 5.3 Count II: Fraud - Full Evidence
**Purpose:** Complete evidence package for Count II.

```cypher
MATCH (c:LegalCount {id: "count-fraud"})
OPTIONAL MATCH (a:ComplaintAllegation)-[:SUPPORTS]->(c)
OPTIONAL MATCH (m:MotionClaim)-[:PROVES]->(a)
OPTIONAL MATCH (m)-[:RELIES_ON]->(e:Evidence)
RETURN c.title as count,
       a.id as allegation,
       a.allegation as claim,
       m.title as motion_claim,
       e.exhibit_number as evidence,
       e.answer as admission
ORDER BY a.id;
```

### 5.4 Count III: Declaratory Relief (Ultra Vires) - Full Evidence
**Purpose:** Complete evidence package for Count III.

```cypher
MATCH (c:LegalCount {id: "count-declaratory-ultra-vires"})
OPTIONAL MATCH (a:ComplaintAllegation)-[:SUPPORTS]->(c)
OPTIONAL MATCH (m:MotionClaim)-[:PROVES]->(a)
OPTIONAL MATCH (m)-[:RELIES_ON]->(e:Evidence)
RETURN c.title as count,
       a.id as allegation,
       a.allegation as claim,
       m.title as motion_claim,
       e.exhibit_number as evidence,
       e.answer as admission
ORDER BY a.id;
```

### 5.5 Count IV: Abuse of Process - Full Evidence
**Purpose:** Complete evidence package for Count IV.

```cypher
MATCH (c:LegalCount {id: "count-abuse-of-process"})
OPTIONAL MATCH (a:ComplaintAllegation)-[:SUPPORTS]->(c)
OPTIONAL MATCH (m:MotionClaim)-[:PROVES]->(a)
OPTIONAL MATCH (m)-[:RELIES_ON]->(e:Evidence)
RETURN c.title as count,
       a.id as allegation,
       a.allegation as claim,
       m.title as motion_claim,
       e.exhibit_number as evidence,
       e.answer as admission
ORDER BY a.id;
```

---

## 6. Court Presentation Queries

### 6.1 Executive Summary - Case Strength
**Purpose:** Quick overview for attorney briefing.

```cypher
MATCH (a:ComplaintAllegation)
WITH count(a) as total_allegations,
     sum(CASE WHEN a.evidence_status = "PROVEN" THEN 1 ELSE 0 END) as proven
MATCH (h:Harm)
WITH total_allegations, proven, 
     sum(h.amount) as total_damages,
     count(h) as total_harms
MATCH (e:Evidence)
WITH total_allegations, proven, total_damages, total_harms, count(e) as total_evidence
RETURN total_allegations,
       proven,
       total_damages,
       total_harms,
       total_evidence;
```

### 6.2 Allegation Proof Summary (Court-Ready)
**Purpose:** One-line summary per allegation for motion support.

```cypher
MATCH (a:ComplaintAllegation)
OPTIONAL MATCH (m:MotionClaim)-[:PROVES]->(a)
OPTIONAL MATCH (m)-[:RELIES_ON]->(e:Evidence)
RETURN a.id as allegation,
       a.allegation as claim,
       a.evidence_status as status,
       collect(DISTINCT e.exhibit_number)[0..3] as key_evidence
ORDER BY a.id;
```

### 6.3 Selective Enforcement Pattern
**Purpose:** Show Marie treated differently than sisters.

```cypher
MATCH (e:Evidence)
WHERE e.significance CONTAINS "selective"
   OR e.significance CONTAINS "only Marie"
   OR e.significance CONTAINS "sisters"
   OR e.significance CONTAINS "Nadia"
   OR e.significance CONTAINS "Camille"
   OR e.answer CONTAINS "Not that I recall"
RETURN e.exhibit_number, e.title, e.answer, e.significance;
```

### 6.4 Pattern of Disparagement
**Purpose:** Show systematic character attacks on Marie.

```cypher
MATCH (e:Evidence)
WHERE e.answer CONTAINS "unintelligible"
   OR e.answer CONTAINS "conspiracy"
   OR e.answer CONTAINS "North Korea"
   OR e.answer CONTAINS "roadblock"
   OR e.answer CONTAINS "assault"
   OR e.significance CONTAINS "disparag"
RETURN e.exhibit_number, e.title, e.answer, e.significance;
```

### 6.5 Conflict of Interest Evidence
**Purpose:** CFS-Court financial relationship proof.

```cypher
MATCH (e:Evidence)
WHERE e.significance CONTAINS "conflict"
   OR e.significance CONTAINS "contract"
   OR e.significance CONTAINS "revenue"
   OR e.answer CONTAINS "contract"
   OR e.answer CONTAINS "excess"
RETURN e.exhibit_number, e.title, e.answer, e.significance;
```

---

## 7. Document Queries

### 7.1 All Documents with Metadata
**Purpose:** Complete document inventory.

```cypher
MATCH (d:Document)
OPTIONAL MATCH (e:Evidence)-[:CONTAINED_IN]->(d)
RETURN d.id,
       d.title,
       d.doc_type,
       d.filed_date,
       count(e) as evidence_count
ORDER BY d.filed_date;
```

### 7.2 Documents by Case
**Purpose:** Show which documents belong to which case.

```cypher
MATCH (d:Document)-[:IN_CASE]->(c:Case)
RETURN c.name as case_name,
       collect(d.title) as documents,
       count(d) as doc_count;
```

### 7.3 Court Rulings Only
**Purpose:** List judicial opinions/orders.

```cypher
MATCH (d:Document)
WHERE d.doc_type = "opinion"
RETURN d.id,
       d.title,
       d.court,
       d.filed_date,
       d.judge,
       d.key_holdings;
```

### 7.4 Evidence Count by Document
**Purpose:** Which documents are most evidence-rich.

```cypher
MATCH (e:Evidence)-[:CONTAINED_IN]->(d:Document)
RETURN d.title,
       count(e) as evidence_count,
       collect(e.exhibit_number)[0..5] as sample_evidence
ORDER BY evidence_count DESC;
```

---

## 8. Graph Exploration Queries

### 8.1 Node Counts by Label
**Purpose:** Database statistics.

```cypher
MATCH (n) 
RETURN labels(n)[0] as label, count(*) as count 
ORDER BY count DESC;
```

### 8.2 Relationship Counts by Type
**Purpose:** Relationship statistics.

```cypher
MATCH ()-[r]->() 
RETURN type(r) as relationship_type, count(*) as count 
ORDER BY count DESC;
```

### 8.3 Find Connections to Any Node
**Purpose:** Explore what connects to a specific node.
**Parameter:** Replace node id as needed.

```cypher
MATCH (n {id: "complaint-005"})-[r]-(connected)
RETURN type(r) as relationship,
       labels(connected)[0] as connected_type,
       connected.id as connected_id,
       connected.title as connected_title;
```

### 8.4 Shortest Path Between Nodes
**Purpose:** Find how two nodes are connected.
**Parameter:** Replace start and end node ids.

```cypher
MATCH path = shortestPath(
  (start {id: "harm-001"})-[*]-(end {id: "evidence-phillips-q107"})
)
RETURN [n IN nodes(path) | n.id] as node_path,
       [r IN relationships(path) | type(r)] as relationship_path;
```

### 8.5 All Persons and Their Roles
**Purpose:** Show people involved in the case.

```cypher
MATCH (p:Person)
OPTIONAL MATCH (p)-[r]->(n)
RETURN p.id, 
       p.name, 
       p.role,
       collect(DISTINCT type(r)) as relationships,
       collect(DISTINCT labels(n)[0]) as connected_to;
```

---

## 9. Validation Queries

### 9.1 Find Orphan Nodes
**Purpose:** Nodes with no relationships (data quality check).

```cypher
MATCH (n)
WHERE NOT (n)--()
RETURN labels(n)[0] as label, n.id as id, n.title as title;
```

### 9.2 Evidence Without Documents
**Purpose:** Find evidence not linked to source documents.

```cypher
MATCH (e:Evidence)
WHERE NOT (e)-[:CONTAINED_IN]->(:Document)
RETURN e.id, e.title;
```

### 9.3 Allegations Without Motion Claims
**Purpose:** Find allegations lacking proof structure.

```cypher
MATCH (a:ComplaintAllegation)
WHERE NOT (:MotionClaim)-[:PROVES]->(a)
RETURN a.id, a.allegation, a.evidence_status;
```

### 9.4 Harms Without Evidence
**Purpose:** Find harms needing evidentiary support.

```cypher
MATCH (h:Harm)
WHERE NOT (h)-[:EVIDENCED_BY]->(:Evidence)
RETURN h.id, h.title, h.amount;
```

### 9.5 Motion Claims Without Evidence
**Purpose:** Find claims needing evidence links.

```cypher
MATCH (m:MotionClaim)
WHERE NOT (m)-[:RELIES_ON]->(:Evidence)
RETURN m.id, m.title;
```

### 9.6 Full Traceability Verification
**Purpose:** Confirm all 18 allegations have complete chains.

```cypher
MATCH (a:ComplaintAllegation)
OPTIONAL MATCH (c:LegalCount)<-[:SUPPORTS]-(a)
OPTIONAL MATCH (m:MotionClaim)-[:PROVES]->(a)
OPTIONAL MATCH (m)-[:RELIES_ON]->(e:Evidence)
OPTIONAL MATCH (e)-[:CONTAINED_IN]->(d:Document)
RETURN a.id,
       count(DISTINCT c) as counts,
       count(DISTINCT m) as motion_claims,
       count(DISTINCT e) as evidence,
       count(DISTINCT d) as documents
ORDER BY a.id;
```

---

## Quick Reference

### Most Useful Queries for Court Prep

| Need | Query |
|------|-------|
| Total damages | 1.1 |
| Damages by count | 1.3 |
| Strongest evidence | 2.2 |
| All Phillips admissions | 4.1 |
| All CFS admissions | 4.2 |
| Evidence for specific count | 5.2-5.5 |
| Case strength overview | 6.1 |
| Selective enforcement proof | 6.3 |
| Disparagement pattern | 6.4 |

### Parameter Queries (Replace IDs)

| Query | Parameter |
|-------|-----------|
| 2.4 | allegation id (e.g., "complaint-005") |
| 3.4 | harm id (e.g., "harm-001") |
| 8.3 | any node id |
| 8.4 | start and end node ids |

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-01-22 | Initial creation - 35 queries |
