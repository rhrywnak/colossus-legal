# Colossus Legal — Extraction Template Construction Guide

**Version:** 2.0
**Date:** 2026-04-24
**Status:** Living document — update after each template iteration
**Purpose:** Definitive reference for designing, testing, and deploying LLM extraction templates. Contains the domain knowledge, teaching patterns, and reusable content needed to build templates for any legal document type.

**Reference templates:** The working complaint templates (pass1_complaint_v4.md and pass2_complaint_v4.md) are the gold-standard examples. Every pattern described in this guide is demonstrated in those templates. Read them before building any new template.

---

## PART I: PRINCIPLES AND RULES

---

## 1. The Fundamental Truth

Each LLM pass is an ATOMIC request. The LLM has ZERO memory of any prior pass, any prior document, or any prior conversation. Pass 2 knows NOTHING about pass 1 unless you explicitly teach it everything it needs to know.

Every template must be completely self-contained. It must teach the LLM:
- WHO it is (persona and purpose)
- WHY this extraction matters (data model context, how the data will be used)
- WHAT the document is (document type, structure, sections)
- WHAT to extract (entity types with definitions, positive and negative examples)
- WHAT to skip (explicit filtering with examples of boilerplate to ignore)
- HOW to extract (step-by-step procedure, worked examples)
- HOW to format output (JSON structure with concrete example)
- HOW to verify completeness (checklist with both positive and negative checks)

If any of these is missing, the LLM guesses — and guesses wrong.

---

## 2. Research Foundation

Key findings that directly affect template design:

1. **Multi-stage extraction is universal.** Pass 1 = entities. Pass 2 = relationships. NEVER combine. (CORE-KG, LINK-KG, KGGen, GraphRAG all validate this.)

2. **Removing structured prompts increases noise by 73%.** (CORE-KG.) Structure means: persona, domain knowledge, step-by-step procedure, examples, filtering.

3. **Sequential entity extraction reduces attention dilution.** One entity type at a time, fixed order.

4. **Explicit filtering is essential.** Legal documents are full of boilerplate that looks like content.

5. **Few-shot with positive AND negative examples** dramatically improves quality.

6. **The LLM needs to understand WHY.** Data model context improves extraction decisions.

7. **Legal reasoning can't be mechanical.** SUPPORTS must be based on whether facts help prove legal elements — not on paragraph ranges or incorporation clauses.

Full research: LLM_PROMPT_ENGINEERING_LEGAL_RESEARCH_v2.md (project knowledge).

---

## 3. The Three-Stage Prompt Architecture

Every template follows three stages. See pass1_complaint_v4.md for a complete implementation.

### Stage 1: Task Definition
- **Persona:** "You are a senior litigation paralegal preparing for trial." Not "You are extracting structured information."
- **Purpose:** "You are building a knowledge graph for trial preparation — identifying bias, tracking misconduct, building evidence chains."
- **Scope:** "In this pass, you extract ENTITIES ONLY. Relationships come in pass 2."
- **Data model context:** How this document's entities connect to the broader case graph. What happens when an entity is missed.

### Stage 2: Knowledge Background
- **Document type tutorial:** What IS this document? What is its purpose in litigation? (See Part II for each type.)
- **Document anatomy:** Section-by-section breakdown. What each section contains, what to extract, what to skip.
- **Entity type definitions:** Plain language with the provability test. Positive AND negative examples.
- **For pass 2:** Re-teach what pass 1 did — explain each entity type, what it means, why it was extracted. Teach legal reasoning concepts (legal elements, incorporation by reference, SUPPORTS vs paragraph ranges).

### Stage 3: Reasoning Guidance
- **Step-by-step procedure:** Numbered steps, one entity type per step, fixed order.
- **Filtering instructions:** Specific boilerplate categories to skip, with examples.
- **Few-shot example:** Realistic input → correct output. PLUS a negative example (input → skip, with explanation).
- **Checklist:** Positive checks ("did I extract every party?") AND negative checks ("did I avoid extracting jurisdictional paragraphs?").

---

## 4. Non-Negotiable Rules

### Architecture
| Rule | Why |
|------|-----|
| Pass 1 = entities ONLY | Multi-pass validated by all research |
| Pass 2 = relationships ONLY, no new entities | LLM needs complete entity list for relationship reasoning |
| Sequential extraction in fixed order | Reduces attention dilution (CORE-KG) |
| Specific expert persona | Improves quality per research |

### Content
| Rule | Why |
|------|-----|
| Case-agnostic — no party names, no case-specific references | Templates must work for any case. Context via {{admin_instructions}} |
| Positive AND negative few-shot examples | LLM learns from examples better than instructions |
| Explicit filtering with examples | Without filtering, 73% noise increase |
| Provability test for allegations | "Could a witness testify? Could a document prove this?" |
| Pass 2 re-teaches everything relevant from pass 1 | LLM has zero memory between passes |
| Pass 2 teaches legal reasoning, not mechanical rules | SUPPORTS based on elements, not paragraph ranges |

### Format
| Rule | Why |
|------|-----|
| verbatim_quote at TOP LEVEL, not inside properties | Parser and Verify read from top level |
| Property names match code (see Section 14) | Mismatched names cause node collapse |
| Concrete JSON example in output format | LLM needs to see exact structure expected |

### Process
| Rule | Why |
|------|-----|
| NEVER rewrite a working template from scratch | Rewrites lose proven strategies |
| Trace complete pipeline path before deploying | Extract → parse → verify → approve → ingest → Neo4j |
| Desk-check against actual document | Verify template handles each section correctly |
| Test with one document first | Catch problems before burning API credits on batch |

---

## PART II: DOCUMENT TYPE REFERENCE

Each document type has unique structure, entity types, and relationship logic. This section provides the domain knowledge needed to write templates. For the tutorial text format, follow the patterns in pass1_complaint_v4.md (document anatomy section) and pass2_complaint_v4.md (relationship reasoning section).

---

## 5. Civil Complaint

**Reference implementation:** pass1_complaint_v4.md, pass2_complaint_v4.md

### Purpose in litigation
Initiates the lawsuit. Defines the case skeleton — parties, factual allegations, legal counts, harms. Foundation document that everything else connects to.

### Structure
1. Caption/header → extract nothing
2. Jurisdictional/party identification (first several paragraphs) → Party entities only, NOT allegations
3. Factual allegations section ("COMMON ALLEGATIONS" or similar) → ComplaintAllegation entities
4. Incorporation paragraphs → skip entirely
5. Counts/causes of action → LegalCount entities; new factual claims within counts → ComplaintAllegation
6. Prayer for relief → nothing (note dollar amounts for Harms)
7. Signature block → nothing

### Entity types
- **Party** (reference, name_match): named persons and organizations
- **ComplaintAllegation** (evidence, verbatim): substantive factual claims of wrongdoing — apply the provability test
- **LegalCount** (foundation, heading_match): causes of action with legal basis and required elements
- **Harm** (foundation, derived): damages suffered, with provenance array (no verbatim_quote)

### Filtering rules (what NOT to extract as ComplaintAllegation)
- Party identification paragraphs ("Plaintiff is an individual residing in...")
- Jurisdictional statements ("Venue is proper in this court...")
- Incorporation paragraphs ("Plaintiff hereby incorporates paragraphs 1 through X...")
- Formulaic damages conclusions ("Plaintiff has been damaged in an amount exceeding $25,000")
- Count headings
- Relief/wherefore sections

### Pass 2 relationship logic
- **SUPPORTS** requires legal reasoning — does the allegation help prove an element of the count? NOT mechanical paragraph-range matching. Template must teach what legal elements are and how to evaluate relevance per count type.
- **ABOUT** links allegations to mentioned parties. "Defendants" (plural) = ABOUT each defendant.
- **CAUSED_BY, DAMAGES_FOR, SUFFERED_BY, EVIDENCED_BY** for harm relationships.

### Key teaching points for pass 2
The template must explain:
- What legal elements are (what must be proven for each type of cause of action)
- Common elements for: breach of fiduciary duty, fraud, abuse of process, declaratory relief, negligence
- That incorporation by reference makes facts AVAILABLE but not automatically RELEVANT
- The analogy: "Incorporation puts evidence on the table. SUPPORTS means the evidence proves your point."

---

## 6. Affidavit

**Reference implementation:** To be built. Model after pass1_complaint_v4.md structure.

### Purpose in litigation
Sworn testimony from a witness. The affiant makes statements under oath, notarized. Carries the weight of sworn testimony — can be used as evidence in court.

### Structure
1. Caption/header ("AFFIDAVIT OF [NAME]") → note affiant name
2. Preamble/identity ("I, [NAME], being duly sworn...") → Party entity for affiant. May also contain case-relevant facts (affiant's role, qualifications, relationship to the case).
3. Numbered sworn statements (body) → Evidence entities for each substantive paragraph
4. Conclusion ("Further affiant sayeth not") → nothing
5. Signature and notarization → note date for statement_date property

### Entity types
- **Party** (reference, name_match): the affiant, every person/organization mentioned
- **Evidence** (evidence, verbatim): each substantive sworn statement
  - kind: "testimonial"
  - evidence_strength: "sworn_testimony"
  - statement_type: "sworn_testimony", "factual_assertion", "expert_opinion"

### Filtering rules
- Skip formulaic identity paragraphs ("I am over 18 years of age and competent to testify") UNLESS they contain case-relevant facts
- Skip the conclusion/attestation
- Skip notary block text

### Pass 2 relationship logic
- **STATED_BY** (Evidence → Party): always the affiant, even when quoting others
- **ABOUT** (Evidence → Party): whoever the statement discusses
- Cross-document: **CORROBORATES** (supports a complaint allegation), **CONTRADICTS** (conflicts with same speaker's statement in another document), **REBUTS** (counters a different speaker)

### Key teaching points for pass 2
- What CORROBORATES means: independent testimony confirming a complaint allegation
- What CONTRADICTS means: SAME person said conflicting things in different documents
- What REBUTS means: DIFFERENT people's statements counter each other
- How to evaluate whether a sworn statement corroborates an allegation (does it confirm the same facts?)

---

## 7. Discovery Response

**Reference implementation:** To be built. Model after pass1_complaint_v4.md structure.

### Purpose in litigation
Sworn answers to questions posed by the opposing party. The most important type is interrogatories — numbered questions that must be answered under oath. Admissions in discovery are binding on the party. Evasive answers ("documents speak for themselves") suggest concealment.

### Structure
1. Caption/header → note responding party name
2. Verification/oath → note respondent name (the STATED_BY party)
3. Question-answer pairs (body) → Evidence entity for EACH Q&A pair
4. Objections within answers → still extract as Evidence, note the objection in the answer
5. Document references within answers → note in significance property
6. Signature/verification block → nothing

### Entity types
- **Party** (reference, name_match): respondent, requesting party, all mentioned persons/organizations
- **Evidence** (evidence, verbatim): each Q&A pair
  - kind: "testimonial"
  - evidence_strength: "sworn_party_admission" (these are sworn statements by a named party)
  - statement_type: "admission" (fact acknowledged), "denial" (fact contested), "evasive" (non-answer)
  - question: the interrogatory question
  - answer: the sworn response
  - paragraph: interrogatory number (e.g., "Q73", "RFA 9")

### Filtering rules
- Do NOT skip objection-only responses — extract them as Evidence with statement_type="evasive"
- Do NOT skip "documents speak for themselves" answers — these are evidence of evasion
- Skip the verification boilerplate but extract the respondent as a Party

### Key patterns to teach the LLM
- **Admissions** are the strongest evidence — a party admitting a fact under oath
- **Evasive responses** suggest concealment — the party won't directly answer
- **Selective treatment** — did the respondent treat one party differently than another?
- **Financial details** — amounts, dates, account information

### Pass 2 relationship logic
- **STATED_BY**: always the respondent (who signed), NOT the attorney who prepared it
- **ABOUT**: who or what the question concerns — may differ from the respondent
- Cross-document: **CORROBORATES/CONTRADICTS** against complaint allegations and other documents

---

## 8. Court Ruling

**Reference implementation:** To be built. Model after pass1_complaint_v4.md structure.

### Purpose in litigation
The court's own findings and conclusions. Carries judicial authority — the court determined these facts and reached these legal conclusions. NOT a party's argument.

### Structure
1. Caption/header → note judge name and date
2. Procedural history ("This matter comes before the Court...") → skip unless it contains factual findings
3. Factual findings ("The Court finds that...") → Evidence entities
4. Legal analysis/conclusions ("Based on the foregoing...") → Evidence entities
5. Orders/directives ("IT IS HEREBY ORDERED...") → Evidence entities
6. Signature → nothing

### Entity types
- **Party** (reference, name_match): judge(s), all parties mentioned
- **Evidence** (evidence, verbatim): each finding, conclusion, and order
  - kind: "documentary"
  - evidence_strength: "court_finding" (carries judicial authority)
  - statement_type: "court_finding" (factual determination), "legal_conclusion" (legal determination), "court_order" (directive)

### Filtering rules
- Skip procedural history that merely recites prior filings without making findings
- Skip citations to legal authority (they're reasoning, not findings) — but note cited statutes in significance
- Extract EVERY distinct factual finding, legal conclusion, and order

### Key teaching points
- Court findings carry unique weight — they're the court's own determinations
- Watch for: does the court credit one party's arguments without stated basis? (judicial_bias tag)
- Watch for: does the court apply different standards to different parties? (selective_enforcement tag)

---

## 9. Motion

**Reference implementation:** To be built. Model after pass1_complaint_v4.md structure.

### Purpose in litigation
A party's request for court action (summary judgment, sanctions, dismissal, etc.). Contains BOTH legal arguments (MotionClaim) AND factual assertions/characterizations (Evidence). Motions are NOT sworn — they're arguments, not testimony.

### Structure
1. Caption/header → note movant name
2. Introduction/statement of issues → may contain characterizations of opposing party
3. Statement of facts → factual assertions (some accurate, some characterizations)
4. Legal argument → legal theories and reasoning
5. Relief requested → what the motion asks the court to do
6. Signature → nothing

### Entity types
- **Party** (reference, name_match): movant, respondent, all mentioned
- **MotionClaim** (evidence, derived): synthesized legal arguments and positions
  - category: "admission", "factual_allegation", "legal_argument", "characterization", "evidence_summary"
- **Evidence** (evidence, verbatim): specific factual claims and characterizations with verbatim quotes
  - kind: "documentary"
  - evidence_strength: "party_statement" (NOT sworn)
  - statement_type: "factual_assertion", "characterization", "admission", "legal_argument"

### Key teaching points
- Motions are arguments, not evidence — evidence_strength is "party_statement", not "sworn_testimony"
- Characterizations of the opposing party are important — extract as Evidence with statement_type="characterization"
- Inadvertent admissions embedded in arguments are valuable — extract with statement_type="admission"
- Watch for disparagement, selective enforcement, misrepresentation patterns

---

## 10. Legal Brief

**Reference implementation:** To be built. Model after pass1_complaint_v4.md structure.

### Purpose in litigation
Written legal argument filed with the court (appellate brief, response brief, reply brief). Briefs are the PRIMARY source of characterizations — where parties label and dismiss each other's claims in writing. NOT sworn.

### Structure
Similar to motions but typically longer and more formal:
1. Table of contents/authorities → skip
2. Statement of issues → extract as MotionClaim if substantive
3. Statement of facts → factual assertions and characterizations → Evidence entities
4. Legal argument → legal reasoning → MotionClaim entities
5. Conclusion → nothing

### Entity types
Same as Motion (Party, MotionClaim, Evidence).

### Key teaching points
- **CHARACTERIZATIONS ARE THE PRIORITY.** Every instance where the brief labels, dismisses, or disparages the opposing party's claims is an Evidence node with statement_type="characterization" and pattern_tags="disparagement"
- Misrepresentations of fact — claims that other evidence contradicts
- Selective narrative — discussing one party's actions but not another's identical conduct

---

## PART III: PASS 2 TEACHING PATTERNS

---

## 11. How to Re-Teach Pass 1 Context in Pass 2

Pass 2 receives entities from pass 1 as a JSON blob ({{entities_json}}). But the LLM doesn't know what these entities mean or why they were extracted. Pass 2 must include a section explaining:

### Pattern: "What Happened in Pass 1"

Tell the LLM:
1. What role the pass 1 analyst had
2. What entity types were extracted and what each means
3. What was intentionally EXCLUDED and why (so the LLM doesn't wonder why there are "gaps")
4. That the entity IDs in the list are the ONLY valid IDs — do not invent new ones

See pass2_complaint_v4.md "What Happened in Pass 1" section for the reference implementation.

### Pattern: "Why These Relationships Matter"

Give concrete examples of questions the knowledge graph will answer, and show how missing relationships break those queries:
- "What evidence supports Count I?" → follows SUPPORTS edges
- "What did this misconduct cost the plaintiff?" → follows CAUSED_BY and DAMAGES_FOR
- "What allegations involve this defendant?" → follows ABOUT edges

See pass2_complaint_v4.md "Why These Relationships Matter" section for the reference implementation.

---

## 12. How to Teach Legal Reasoning

### Pattern: Teaching Legal Elements

The LLM needs to understand that each cause of action has ELEMENTS — specific things that must be proven. The template should:

1. Explain what elements are in general terms
2. List common elements for the cause of action types relevant to the document
3. Show how to evaluate whether a factual allegation helps prove any element
4. Provide worked examples showing the reasoning process

See pass2_complaint_v4.md "SUPPORTS" section for the reference implementation, especially:
- The elements breakdown for each count type
- The worked examples showing "this allegation supports this count because..."
- The common mistakes section

### Pattern: Explaining Incorporation by Reference

This is specific to complaints but critical:
- Explain what it is (legal convention, not substantive content)
- Explain what it means (facts become available, not automatically relevant)
- Use the analogy: "puts evidence on the table" vs "proves your point"
- Show a worked example of an allegation that IS in the incorporation range but does NOT support that count

### Pattern: Cross-Document Relationship Reasoning

For pass 2 on evidence documents (not complaints), the LLM must understand:
- **CORROBORATES**: independent confirmation of a fact from the complaint. The test: "Does this statement confirm the same facts as the complaint allegation, from an independent source?"
- **CONTRADICTS**: same speaker, different statements. The test: "Did the same person say something different in another document?"
- **REBUTS**: different speakers, opposing claims. The test: "Does this person's statement directly counter what another person claimed?"

---

## 13. Step-by-Step Reasoning Process Pattern

Every pass 2 template should include an explicit reasoning procedure. The pattern:

### Step 1: Understand the framework
For complaints: map out each count's legal elements.
For evidence documents: review the complaint allegations that exist in the entity list.

### Step 2: Evaluate each entity against the framework
For each entity, systematically check: does this fact relate to element X? Element Y?
Don't skip any entity. Don't skip any possible relationship target.

### Step 3: Create relationships with justification
The LLM should be reasoning about each relationship, not mechanically creating them.

### Step 4: Verify completeness
Every entity should have at least one relationship. If an entity has zero relationships, re-examine it.

---

## PART IV: REFERENCE

---

## 14. Property Name Reference

These are canonical — they match what the ingest code reads and what PROD Neo4j queries expect.

### Party
```
party_name, role, party_type, aliases
```

### ComplaintAllegation
```
paragraph_number, summary, category, severity, applies_to, amount, event_date
```

### LegalCount
```
count_name, count_number, legal_basis, paragraphs, key_elements, damages_claimed, applies_to
```

### Harm
```
description, category, subcategory, amount, harm_type, date
```

### Evidence
```
title, question, answer, page_number, paragraph, exhibit_number, kind, evidence_strength,
statement_type, significance, weight, statement_date, event_date, page_note, pattern_tags
```

---

## 15. Schema YAML Requirements

Every entity type MUST have:
```yaml
- name: EntityTypeName
  grounding_mode: verbatim | name_match | heading_match | derived
  category: evidence | foundation | reference
  description: "..."
  properties: [...]
```

Missing grounding_mode → defaults to Verbatim → Parties/LegalCounts fail grounding.
Missing category → defaults to Evidence → wrong UI display. System now logs ERROR.
verbatim_quote is NOT a schema property — it's a top-level entity field in the template output format.

---

## 16. Failure Log

| # | Failure | Rule |
|---|---------|------|
| 1 | Rewrote working template from scratch | NEVER rewrite — modify minimally |
| 2 | Asked for entities AND relationships in one pass | Pass 1 = entities ONLY |
| 3 | Used "improved" property names that didn't match code | Property names MUST match code |
| 4 | Missing grounding_mode in schema | Every entity type needs grounding_mode |
| 5 | Missing category in schema | Every entity type needs category |
| 6 | verbatim_quote in schema properties AND template top level | verbatim_quote is NOT a schema property |
| 7 | Case-specific references in template | Templates MUST be case-agnostic |
| 8 | SUPPORTS based on incorporation ranges, not legal reasoning | Teach legal elements, not mechanical rules |
| 9 | Deployed without tracing pipeline path | ALWAYS trace extract → parse → verify → approve → ingest → Neo4j |
| 10 | Missing character normalization in canonical verifier | Canonical verifier must normalize same chars as PageGrounder |

---

## 17. Template Deployment Checklist

### Pre-deployment
- [ ] Follows three-stage architecture
- [ ] Case-agnostic
- [ ] Expert persona assigned
- [ ] Document structure explained
- [ ] Entity types defined with positive AND negative examples
- [ ] Explicit filtering instructions
- [ ] Step-by-step procedure
- [ ] Few-shot examples
- [ ] Entities ONLY (pass 1) or relationships ONLY (pass 2)
- [ ] verbatim_quote at top level
- [ ] Property names match code (Section 14)
- [ ] Schema has grounding_mode and category on every entity type
- [ ] Pass 2 re-teaches pass 1 context
- [ ] Pass 2 teaches legal reasoning (not mechanical rules)

### Pipeline verification
- [ ] Property names match stable_entity_id()
- [ ] Property names match create_entity_node()
- [ ] Property names match create_party_nodes()
- [ ] grounding_mode values match verify.rs
- [ ] category values match items.rs

### Post-deployment
- [ ] Process one document, check results
- [ ] Verify entity counts, grounding rates, Neo4j nodes, relationships
- [ ] Spot-check in Review tab and Evidence Explorer
- [ ] Update this document with any new lessons learned

---

## 18. Template File Inventory

| File | Type | Document Type | Status |
|------|------|--------------|--------|
| complaint_v4.yaml | Schema | Complaint | ✅ Complete |
| pass1_complaint_v4.md | Pass 1 template | Complaint | ✅ Complete |
| pass2_complaint_v4.md | Pass 2 template | Complaint | ✅ Complete |
| discovery_response_v4.yaml | Schema | Discovery | Needs category field |
| pass1_discovery_v4.md | Pass 1 template | Discovery | Needs full redesign per this guide |
| affidavit_v4.yaml | Schema | Affidavit | Needs category field |
| pass1_affidavit_v4.md | Pass 1 template | Affidavit | Needs full redesign per this guide |
| court_ruling_v4.yaml | Schema | Court Ruling | Needs category field |
| pass1_court_ruling_v4.md | Pass 1 template | Court Ruling | Needs full redesign per this guide |
| motion_v4.yaml | Schema | Motion | Needs category field |
| pass1_motion_v4.md | Pass 1 template | Motion | Needs full redesign per this guide |
| brief_v4.yaml | Schema | Brief | Needs category field |
| pass1_brief_v4.md | Pass 1 template | Brief | Needs full redesign per this guide |
| pass2_universal_v4.md | Pass 2 template | All evidence types | Needs full redesign per this guide |
| global_rules_v4.md | Shared rules | All | Deployed |
| legal_extraction_system.md | System prompt | All | Deployed |
