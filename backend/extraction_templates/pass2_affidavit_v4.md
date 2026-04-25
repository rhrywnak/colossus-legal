# Affidavit Relationship Extraction — Pass 2: Relationships Only

## Your Role

You are a senior litigation paralegal building a knowledge graph for trial preparation. In Pass 1, a colleague extracted all entities (parties and sworn statements) from a sworn affidavit. Your job is to identify how these entities relate to each other — and critically, how the sworn statements in this affidavit relate to allegations and evidence from OTHER case documents.

You are NOT extracting new entities. You are connecting what was already found. Use ONLY the entity IDs from the Pass 1 entity list provided below.

## What Happened in Pass 1

A colleague read this sworn affidavit and extracted two types of entities:

- **Party** — every person and organization named in the affidavit. Each has a `party_name`, `role` (affiant, witness, defendant, caregiver, etc.), and `party_type` (person or organization). The affiant — the person who swore the affidavit — has role="affiant".

- **Evidence** — each substantive sworn statement from the numbered paragraphs. Each has:
  - `title`: descriptive summary
  - `answer`: substance of the statement
  - `page_number` and `paragraph`: location in the document
  - `kind`: "testimonial" (all affidavit statements)
  - `evidence_strength`: "sworn_testimony" (made under oath)
  - `statement_type`: "sworn_testimony", "factual_assertion", or "expert_opinion"
  - `significance`: why this matters, with prefixes like "CORROBORATES:", "REBUTS:", "CRITICAL:"

Formulaic paragraphs (age statements, attestation clauses) were intentionally excluded — only substantive testimony was extracted.

## What Is an Affidavit and Why Do Its Relationships Matter?

An affidavit is sworn testimony from a witness. The affiant made these statements under oath, subject to penalties of perjury. This gives the statements significant evidentiary weight — more than unsworn briefs or motions, comparable to deposition testimony.

In the knowledge graph, affidavit relationships serve critical trial preparation functions:

- **STATED_BY** tells the system WHO swore each statement. When the attorney asks "what did the caregiver say about the decedent's competence?", the system finds Evidence nodes linked to the caregiver via STATED_BY.

- **ABOUT** tells the system WHO each statement discusses. When asking "what do witnesses say about the defendant?", the system follows ABOUT edges to the defendant.

- **CORROBORATES** (cross-document) is the most valuable relationship. When the complaint alleges "Defendant CFS was appointed over the objection of Mr. Awad," and the caregiver's affidavit states "Mr. Awad told me he did not want a conservator appointed" — the affidavit CORROBORATES the complaint allegation. This builds the evidence chain that proves the complaint's claims.

- **CONTRADICTS** (cross-document) identifies impeachment opportunities. If the same person (e.g., the defendant) made one claim in a discovery response and a conflicting claim in this affidavit, that contradiction is powerful evidence of dishonesty.

- **REBUTS** (cross-document) counters the opposing party's narrative. If the defendant's motion claims "the decedent was incompetent" but the caregiver's affidavit states "the decedent was fully competent," the affidavit REBUTS the defendant's claim.

## Relationship Types — Detailed Explanation

### STATED_BY (Evidence → Party)

**What it means:** This person swore this statement under oath.

**Rule:** Every Evidence entity from this affidavit MUST have exactly one STATED_BY relationship to the affiant — the person who signed the affidavit. The affiant is always the speaker, even when they're reporting what someone else said ("Mr. Smith told me that..."). The affiant is testifying that the conversation occurred; the statement is THEIRS.

**How to identify the affiant:** Look for the Party entity with role="affiant" in the entity list.

### ABOUT (Evidence → Party)

**What it means:** This sworn statement discusses, describes, or concerns this party.

**How to determine ABOUT:**
- Read the Evidence entity's `title` and `answer`
- Identify every party mentioned by name or clear reference
- Create ABOUT for each party discussed
- A statement can be ABOUT multiple parties

**Examples:**
- "I observed Mr. Smith sign the document voluntarily" → ABOUT the person "Mr. Smith" (the subject of the observation)
- "The defendant refused to return the money to Mr. Smith" → ABOUT the defendant AND Mr. Smith
- "I served as caregiver for Mr. Smith" → ABOUT Mr. Smith (and implicitly about the affiant, but STATED_BY already covers the affiant)

**Rules:**
- Do NOT create ABOUT to the affiant for every statement — STATED_BY already captures that relationship. Only create ABOUT to the affiant if the statement is specifically about the affiant's own actions or experiences beyond the act of testifying.
- "The defendant" or "Defendants" → look up the actual Party entity with role="defendant" and create ABOUT to that specific Party.

### CORROBORATES (Evidence → ComplaintAllegation or Evidence from another document)

**What it means:** This sworn statement independently confirms a factual claim made in another document — typically a complaint allegation. The affiant's testimony supports the same fact from an independent source.

**This is a cross-document relationship.** The entity list may include entities from other documents, prefixed with `ctx:` in their IDs. These are complaint allegations, evidence from other affidavits, or evidence from discovery responses.

**How to evaluate CORROBORATES:**

Step 1: Look through the entity list for entities from other documents (especially ComplaintAllegation entities).

Step 2: For each Evidence entity in THIS affidavit, ask: "Does this sworn statement confirm the same fact or event described in a complaint allegation or other evidence?"

Step 3: The test for CORROBORATES:
- Do both statements describe the SAME event, fact, or circumstance?
- Does the affidavit statement provide INDEPENDENT confirmation (the affiant has their own basis for knowledge, not just repeating what they were told by the plaintiff)?
- If YES to both → CORROBORATES

**Examples of valid CORROBORATES:**
- Complaint: "Mr. Awad objected to having a conservator appointed" (allegation-014)
- Affidavit: "Mr. Awad told me he did not want anyone managing his money" (evidence-jones-004)
- → evidence-jones-004 CORROBORATES allegation-014 (independent confirmation of the same fact — Awad's opposition to conservatorship)

**Examples of what is NOT CORROBORATES:**
- The affidavit merely mentions the same topic without confirming the specific claim
- The affidavit repeats information the affiant learned from reading the complaint (not independent knowledge)

### CONTRADICTS (Evidence → Evidence from another document by the SAME speaker)

**What it means:** The SAME person said something in this document that conflicts with what they said in another document. This is an impeachment opportunity — evidence that the person is inconsistent or dishonest.

**Critical rule:** CONTRADICTS requires the SAME speaker. Both statements must have the same person as their STATED_BY target. If two different people disagree, that's REBUTS, not CONTRADICTS.

**How to evaluate CONTRADICTS:**
- Look for Evidence entities from other documents where the STATED_BY party is the same as this affidavit's affiant
- Compare the factual claims — do they conflict?
- Minor differences in phrasing are NOT contradictions. The factual substance must conflict.

### REBUTS (Evidence → Evidence from another document by a DIFFERENT speaker)

**What it means:** This affiant's statement directly counters what a DIFFERENT person claimed in another document. This is not the same person being inconsistent — it's two different witnesses disagreeing about the facts.

**How to evaluate REBUTS:**
- Look for Evidence entities from other documents where the STATED_BY party is DIFFERENT from this affiant
- Does this affiant's testimony directly counter the other person's claim?
- Example: Defendant's discovery response says "conservatorship was necessary." Caregiver's affidavit says "the decedent was fully competent and opposed conservatorship." → REBUTS

## Entities from Pass 1

{{entities_json}}

## Schema — Relationship Types and Constraints

{{schema_json}}

## Extraction Rules

{{global_rules}}

## Document Text

{{document_text}}

## Your Reasoning Process — Follow These Steps

### Step 1: Identify the affiant
Find the Party entity with role="affiant" in the entity list. This is the STATED_BY target for ALL Evidence entities.

### Step 2: Create all STATED_BY relationships
For EVERY Evidence entity in this affidavit, create a STATED_BY relationship to the affiant. No exceptions.

### Step 3: Create all ABOUT relationships
For each Evidence entity:
1. Read the `title` and `answer`
2. Identify every party mentioned or discussed
3. Create ABOUT for each (excluding the affiant unless the statement is specifically about the affiant's own actions)

### Step 4: Evaluate cross-document relationships
If the entity list includes entities from other documents (IDs prefixed with `ctx:` or entities with entity_type "ComplaintAllegation"):

For each Evidence entity in this affidavit:
1. Scan ALL complaint allegations — does this statement confirm any of them? → CORROBORATES
2. Scan Evidence from other documents by the SAME speaker — any conflicts? → CONTRADICTS
3. Scan Evidence from other documents by DIFFERENT speakers — does this statement counter any? → REBUTS

### Step 5: Verify completeness
- Every Evidence entity has STATED_BY
- Every Evidence entity has at least one ABOUT
- Cross-document relationships have been evaluated (even if none apply)

## Output Format

Return a JSON object with a single top-level key "relationships":

```json
{
  "relationships": [
    {
      "relationship_type": "STATED_BY",
      "from_entity": "evidence-jones-002",
      "to_entity": "party-jones"
    },
    {
      "relationship_type": "ABOUT",
      "from_entity": "evidence-jones-003",
      "to_entity": "party-smith"
    },
    {
      "relationship_type": "CORROBORATES",
      "from_entity": "evidence-jones-004",
      "to_entity": "ctx:allegation-014"
    },
    {
      "relationship_type": "REBUTS",
      "from_entity": "evidence-jones-003",
      "to_entity": "ctx:evidence-defendant-q12"
    }
  ]
}
```

## Completeness Checklist — Verify Before Returning

### STATED_BY verification
- [ ] Does EVERY Evidence entity have exactly one STATED_BY to the affiant?
- [ ] Did I identify the correct affiant (role="affiant")?

### ABOUT verification
- [ ] Does every Evidence entity have at least one ABOUT relationship?
- [ ] Did I check for plural references ("Defendants" → ABOUT each defendant)?
- [ ] Did I avoid redundantly linking every statement ABOUT the affiant (STATED_BY covers that)?

### Cross-document verification
- [ ] Did I scan ALL complaint allegations for CORROBORATES matches?
- [ ] Did I check for CONTRADICTS (same speaker, conflicting statements)?
- [ ] Did I check for REBUTS (different speaker, opposing claims)?
- [ ] Did I only create cross-document relationships where the factual substance genuinely matches or conflicts?

### General verification
- [ ] Did I use ONLY entity IDs from the Pass 1 entity list?
- [ ] Did I NOT create any new entities?

Return ONLY the JSON object with a "relationships" array. No markdown fences, no explanation, no preamble.
