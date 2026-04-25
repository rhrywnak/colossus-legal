# LLM Prompt Engineering for Legal Document Extraction — Research Findings

**Date:** 2026-04-24
**Purpose:** Consolidated reference for designing extraction templates for Colossus Legal
**Sources:** CORE-KG (2025), LINK-KG (2025), Legal KG frameworks (Nature 2025), NetDocuments prompt engineering guide, Databricks extraction guide, LangChain extraction patterns, prior Colossus session research (April 2022)

---

## 1. The Core Problem: Legal Noise and Extraction Quality

Legal documents are dense, structured, and contain significant amounts of **boilerplate** — procedural language, jurisdictional statements, incorporation paragraphs, formulaic headings, and certification blocks that are structurally necessary but carry no substantive factual content.

**CORE-KG finding:** Removing structured prompts from their pipeline increased **noisy nodes by 73.33%**. The noise consists of procedural terms (Court, Appeal, Judicial Proceedings) that clutter the graph without contributing to analysis. Their structured prompt with explicit filtering instructions reduced this noise by 38.37%.

**Key insight for Colossus:** Our complaint extraction produced 126 ComplaintAllegations when it should have produced ~18-25 substantive allegations. The extra ~100 items are jurisdictional paragraphs, party identification paragraphs, incorporation paragraphs, and count headings — exactly the legal noise CORE-KG describes.

---

## 2. Three-Stage Hierarchical Prompt Architecture

Research from the comprehensive legal dispute analysis framework (Nature, 2025) establishes a three-stage prompt structure:

### Stage 1: Task Definition
- Clearly define the role: "You are a litigation analyst extracting factual allegations of misconduct from a civil complaint"
- Define what a complaint IS and what its sections mean
- Define what you want extracted and WHY (for trial preparation, not document cataloging)

### Stage 2: Knowledge Background
- Explain the structure of the specific document type
- For a complaint: caption → jurisdiction → party identification → common allegations → legal counts → prayer for relief
- Explain which sections contain substantive content and which are procedural
- Provide domain-specific definitions: "A factual allegation is a specific claim of wrongdoing — not a party identification, not a jurisdictional statement, not an incorporation by reference"

### Stage 3: Reasoning Guidance
- Step-by-step extraction procedure (proven to work in our v2 template)
- Explicit filtering instructions: "DO NOT extract jurisdictional paragraphs, party identification paragraphs, incorporation paragraphs, or formulaic count headings"
- Few-shot examples showing input → output for BOTH good extractions AND items to skip
- Completeness checklist with negative checks ("Did I avoid extracting boilerplate?")

---

## 3. CORE-KG Structured Prompting Patterns

The CORE-KG framework achieved 33% reduction in node duplication and 38% reduction in legal noise through these specific prompt techniques:

### 3.1 Sequential Entity Extraction
Extract one entity type at a time in a fixed order, rather than all types simultaneously. This reduces "attention dilution" — when the LLM tries to extract everything at once, it loses focus and misclassifies entities.

**Application to Colossus:** Our v2 template already does this with its step-by-step extraction strategy (Step 1: Parties, Step 2: Legal Counts, Step 3: Allegations, Step 4: Harms). The v4 template I wrote lost this — it used bullet points instead of numbered steps. Keep the sequential approach.

### 3.2 Explicit Filtering Instructions
Include in-prompt instructions to IGNORE procedural and boilerplate terms. CORE-KG's prompt explicitly tells the LLM to filter out "high-frequency irrelevant entities" like Court, Appeal Process, Judicial Proceedings.

**Application to Colossus complaint template:**
```
DO NOT extract the following as ComplaintAllegations:
- Paragraphs 1-6 (party identification and jurisdictional statements)
- Paragraphs that begin with "Plaintiff hereby incorporates paragraphs X through Y"
- "RELIEF REQUESTED" / "WHEREFORE" sections
- Signature blocks, attorney information, certification statements
- Count headings (these are extracted as LegalCount entities, not allegations)
```

### 3.3 Entity Type Definitions with Examples
Provide clear definitions and examples for each entity type directly in the prompt. Reduces misclassification errors.

**Application to Colossus:**
```
A ComplaintAllegation is a SUBSTANTIVE FACTUAL CLAIM of wrongdoing by the defendants.
Examples of ComplaintAllegations:
- "Defendant CFS was illegally in possession of Mr. Awad's money" (paragraph 21)
- "Defendant Phillips began a pattern of spurious accusations" (paragraph 29)

NOT ComplaintAllegations (do not extract):
- "Plaintiff, MARIE AWAD, is an individual presently residing in..." (party identification)
- "Jurisdiction and venue are properly located in Macomb County" (jurisdictional)
- "Plaintiff hereby incorporates paragraphs 1 through 71" (incorporation by reference)
```

### 3.4 Persona Assignment
Assign the LLM a specific expert role. Research shows this improves extraction quality.

**Application to Colossus:** "You are a senior litigation paralegal preparing a civil complaint for trial. Your job is to identify every factual claim of misconduct that could be used to prove the plaintiff's case."

---

## 4. Few-Shot Examples — Critical for Quality

### 4.1 Why Few-Shot Matters
Research consistently shows that providing concrete input → output examples dramatically improves extraction quality. The v2 complaint template had a few-shot example showing a hypothetical paragraph and its correct extraction. The v4 template only showed JSON structure without input context.

### 4.2 Best Practice: Show Both Positive and Negative Examples
- **Positive example:** "Given this paragraph... extract this entity"
- **Negative example:** "Given this paragraph... this is NOT an allegation because it's a jurisdictional statement. Skip it."

### 4.3 Use Real Document Excerpts
The few-shot examples should use text that looks like real legal documents, not generic hypotheticals. For the Awad complaint, we can use actual paragraphs from the document as examples (since the template is complaint-specific, not case-specific).

---

## 5. Two-Pass Architecture — Validated by Research

### Research validation
Every major production legal KG system uses multi-stage extraction:
- CORE-KG: coreference resolution → entity extraction → relationship extraction
- LINK-KG: three-stage coreference → extraction → graph construction
- KGGen, GraphRAG, CORE-KG: all use multi-pass

### Our validated architecture
- Pass 1 (Sonnet): Entity extraction ONLY — no relationships
- Pass 2 (Opus): Relationship extraction with cross-document context

**Rule:** NEVER ask for entities AND relationships in the same pass. This was the exact mistake that caused the v4 template to produce worse results.

---

## 6. Coreference Resolution

### The Problem
Legal documents refer to the same entity using different names: "Defendant Phillips," "Phillips," "George Phillips," "CFS's attorney," "he." Without resolution, the graph fragments.

### CORE-KG Solution
Type-aware coreference resolution — process each entity type separately, merge aliases to a canonical form. Reduced node duplication by 33%.

### Application to Colossus
Our Party entity resolution in `ingest_resolver.rs` handles this for Person/Organization nodes. But within a document, the LLM needs to recognize that "Defendant Phillips," "Phillips," "George Phillips," and "CFS's attorney" are the same person and use one consistent identifier.

**Prompt instruction:** "When referring to a party in entity IDs and STATED_BY/ABOUT relationships, always use the canonical name from the first mention (e.g., 'George Phillips', not 'Defendant Phillips' or 'Phillips')."

---

## 7. Confidence and Grounding

### Research Pattern
The llm-extract library assigns per-field confidence scores:
- 0.95-1.0: Explicitly stated, unambiguous
- 0.80-0.94: Clearly implied
- 0.60-0.79: Inferred from context, may be wrong
- Below 0.60: Very uncertain

### Application to Colossus
Our grounding system (Verify step) serves a similar purpose — it verifies that the verbatim quote actually exists in the document. But we should also consider whether the LLM should self-assess confidence, especially for items where grounding may fail (long quotes that span page boundaries, quotes with OCR artifacts).

---

## 8. Document Structure Awareness

### The Key Insight
Different sections of a legal document serve different purposes. The LLM must understand the document's structure to extract the right things from the right sections.

### Civil Complaint Structure (Michigan, Awad case)
1. **Caption** — case name, court, parties (NOT allegations)
2. **Attorney block** — counsel information (NOT allegations)
3. **Jurisdictional certification** — no prior actions statement (NOT allegations)
4. **JURISDICTION section** (paragraphs 1-6) — party identification, venue (NOT allegations — extract as Party entities)
5. **COMMON ALLEGATIONS section** (paragraphs 7-71) — the SUBSTANTIVE factual claims (THESE are the ComplaintAllegations)
6. **COUNT I-IV sections** (paragraphs 72-126) — legal counts that incorporate earlier paragraphs (extract as LegalCount, NOT as ComplaintAllegation)
7. **RELIEF REQUESTED** — prayer for relief (NOT allegations)

### The v4 template's mistake
It said "extract EVERY numbered paragraph as a ComplaintAllegation." This treated paragraphs 1-6 (party identification), paragraph 7 (incorporation), and paragraphs 72-126 (count headings and incorporations) the same as paragraphs 8-71 (actual allegations). The template needed document structure awareness.

---

## 9. Rules for Colossus Template Design

Based on all research, these rules govern ALL template design:

1. **Three-stage structure:** Task definition → Knowledge background → Reasoning guidance
2. **Sequential extraction:** One entity type at a time, in a fixed order
3. **Explicit filtering:** Tell the LLM what NOT to extract, with specific examples
4. **Persona assignment:** "You are a senior litigation paralegal..."
5. **Document structure explanation:** Explain what each section of the document contains
6. **Few-shot with positive AND negative examples:** Show correct extractions AND things to skip
7. **Pass 1 = entities only:** NEVER ask for relationships in pass 1
8. **Verbatim quote at top level:** Always outside properties, per parser expectations
9. **Completeness checklist with negative checks:** "Did I avoid extracting boilerplate?"
10. **Property names match code:** Use PROD-compatible property names
11. **Never rewrite a working template from scratch:** Modify minimally from the last working version
12. **Entity type definitions IN the prompt:** Not just in the schema — the prompt must explain what each type means in plain language

---

## 10. Template Review Checklist

Before deploying ANY template, verify:

- [ ] Does it assign a specific expert persona?
- [ ] Does it explain the document type's structure?
- [ ] Does it define each entity type with examples?
- [ ] Does it have explicit filtering instructions (what NOT to extract)?
- [ ] Does it have a step-by-step extraction procedure?
- [ ] Does it have few-shot examples with input → output?
- [ ] Does it include negative examples (things to skip)?
- [ ] Does it request entities ONLY (no relationships in pass 1)?
- [ ] Does it put verbatim_quote at the top level of each entity?
- [ ] Does it use property names that match the existing code?
- [ ] Does its JSON example match the parser's expected format?
- [ ] Does it have a completeness checklist with negative checks?
- [ ] Has it been desk-checked against the actual document?
- [ ] Has it been traced through the complete pipeline path (extract → parse → verify → approve → ingest → Neo4j)?
- [ ] Does it explain the data model and why extraction matters?

---

## 11. Data Model Context in Prompts

### Why It Matters

The LLM makes better extraction decisions when it understands the downstream purpose of its output. Without context, it's a document scanner. With context, it's a litigation analyst who understands what matters for trial preparation.

### What to Include

A brief section in each template explaining:

1. **The knowledge graph structure** — "Your extractions become nodes in a knowledge graph. Each ComplaintAllegation connects to LegalCounts (causes of action), Harms (damages), and Evidence (from other documents). Missing an allegation means evidence supporting it has nowhere to attach."

2. **How this document connects to others** — "The complaint is the foundation document. Discovery responses, affidavits, and court rulings will be analyzed against what you extract here. Parties must use canonical names because the same person appears across all documents."

3. **What the extracted data is used for** — "This data supports trial preparation. An attorney and a plaintiff will use it to identify bias, track misconduct, and build evidence chains. Every entity you extract must be specific enough to prove or disprove in court."

4. **What makes a good vs bad extraction** — "A good ComplaintAllegation is a specific, provable claim: 'Defendant CFS held $50,000 for two and a half months without court authorization.' A bad extraction is a procedural statement: 'Jurisdiction and venue are properly located in Macomb County.' The first can be proven with evidence. The second is not disputed."

### What NOT to Include

Do not dump the full data model specification into the prompt. The LLM doesn't need to know about Neo4j MERGE semantics, stable entity IDs, or grounding modes. It needs to understand purpose, not implementation.
