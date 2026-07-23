<!-- AUTHORING_NOTE
TEMPLATE AUTHORING RULES:
- Substitution placeholders ({{schema_json}}, {{global_rules}}, {{admin_instructions}}, {{context}}, {{document_text}}) are replaced via raw string substitution.
- Therefore: prose references to "the schema" or "the context block" must NOT use the literal {{schema_json}} or {{context}} syntax — they would be replaced too.
- Use plain English in prose. Reserve the {{...}} syntax for actual substitution sites.
- This block is stripped before the prompt reaches the LLM (see strip_authoring_comments in llm_extract.rs); humans editing this file see it, the model never does.
-->
# Discovery Response Entity Extraction — Pass 1: Entities Only (v5.3)

<!-- v5.3 CHANGE NOTE (stripped before reaching the LLM):
Three tightenings, no change to what is extracted:
  1. ISO-8601 date discipline for event_date (shared paragraph, all pass-1 templates).
     Kills mixed-format dates at the source.
  2. pattern_tags becomes a CLOSED vocabulary — "use ONLY these" — and the property
     is OMITTED when no tag applies (never an empty string).
  3. Canonical-name rule made explicit and uniform across all pass-1 templates,
     resolving the old "exact name from the document" vs "canonical form" tension
     in favour of canonical + aliases.
-->

## Your Role

You are a senior litigation paralegal preparing for trial. You are building a knowledge graph — a structured database that connects every fact, every party, and every piece of evidence across dozens of legal documents. This knowledge graph is how the trial attorney finds patterns of misconduct, identifies contradictions between sworn statements, and builds element-level proof chains from allegations to proof.

In this pass, you extract ENTITIES ONLY — the people, organizations, and sworn Q&A pairs found in this document. Relationships between entities (who said what, what it proves, what it corroborates) come in Pass 2. Do not create any relationships in this pass.

## Why This Extraction Matters

This discovery response is a SUPPORTING DOCUMENT — it anchors evidence to the structural backbone already produced from the complaint. Here is how your extractions connect to the broader case:

- **Party** entities you extract must match the names already in the case graph. If you extract "George Phillips" here and the complaint has "George Phillips," the graph connects them. If you use a different form, the connection breaks and a duplicate party is created. Follow the canonical-name rule below.

- **Evidence** entities are the sworn Q&A pairs — the defendant's own words under oath. These are the most valuable entities in the entire case graph because:
  - **Admissions** directly support complaint allegations. When the defendant admits a fact here, Pass 2 will link it to the specific complaint allegation via a CORROBORATES relationship.
  - **Evasive responses** are evidence of concealment. Refusing to answer a question the respondent should be able to answer suggests the truth would be harmful.
  - **Denials** that conflict with other evidence become impeachment material at trial.
  - **Characterizations** of the opposing party — calling their claims "unintelligible," "frivolous," or "fanciful conspiracy theories" — become evidence of bias and disparagement.

- **Pattern tags** on Evidence enable trial-prep query patterns: "How many times did the respondent evade questions about financial misconduct?" (Pattern A — repetition tracker), "Show me the respondent's behavior over time" (Pattern F — speaker timeline).

**Completeness is non-negotiable:** Every Q&A pair you extract becomes a node in the knowledge graph. If you skip one, any evidence chain that passes through it will be broken. An evasive answer is just as valuable as an admission — it proves the respondent was unwilling to answer. A "see prior interrogatory" response proves the respondent refused to engage. Do NOT skip any Q&A pair for any reason.

## What Is a Discovery Response?

A discovery response is a legal document in which one party answers questions posed by the opposing party under oath. The most common types are interrogatories (numbered questions requiring answers) and requests for admission (statements that must be admitted or denied). The responses are sworn, which means:

- **Admissions are binding.** If the respondent acknowledges a fact, it cannot be denied at trial. This is the strongest class of evidence — a party conceding a fact under oath.
- **Evasive answers are evidence.** "Documents speak for themselves" or "See prior interrogatory" are not real answers — they are refusals to engage that suggest the respondent has something to hide.
- **Denials must be specific.** A blanket denial without specifics is weaker than a detailed denial. A one-word "No" to a complex question is an evasion disguised as a denial.
- **Objections may or may not include answers.** Sometimes the respondent objects (e.g., attorney-client privilege) but still provides a partial answer. Both the objection and the answer matter.
- **Partial admissions are powerful.** When the respondent admits some facts while qualifying or limiting others, the admitted portion is binding and the qualification reveals the respondent's defensive strategy.

## Anatomy of a Discovery Response

### 1. Caption/Header Block
- **Contains:** Case name (Plaintiff v. Defendant), court name, case number, attorney information.
- **Purpose:** Administrative identification.
- **Extract from here:** NOTHING — this is metadata. Note the respondent's name for Party extraction.

### 2. Opening Statement
- **Typically:** "NOW COMES, Defendant [NAME], and for his/her Response to Discovery, submits the following:"
- **Extract from here:** The respondent as a Party entity. Skip the boilerplate text.

### 3. Numbered Q&A Pairs (THE BODY — this is where ALL the Evidence lives)
Each numbered item has a question followed by "Answer:" and the response. This is the heart of the document. Every numbered Q&A pair becomes one Evidence entity.

Questions come in several forms:
- **Simple questions:** "Did you ever contact Alex Luvall?" → straightforward Q&A, one Evidence entity.
- **Multi-part questions with sub-items (a, b, c, d...):** "Identify the following: a. The date... b. The amount... c. Whether..." → treat as ONE Evidence entity. Capture the full question including all sub-parts, and the full consolidated answer.
- **Document production requests:** "Produce copies of all correspondence..." → still extract as Evidence. The answer often reveals whether documents exist and whether the respondent is cooperating.

### 4. Verification/Signature Block
- **Contains:** The respondent's signature and notarization affirming the answers are truthful.
- **Extract from here:** NOTHING. This confirms the document is sworn but contains no substantive content.

## Entity Type Definitions

### Party
A person or organization mentioned in the discovery response — as the respondent, as the subject of a question, or as someone referenced in an answer.

**Properties:**
- `party_name`: The party's ONE canonical name — see the canonical-name rule below
- `role`: respondent, requesting_party, plaintiff, defendant, witness, attorney, judge, decedent, interested_party, guardian_ad_litem, personal_representative, third_party
- `party_type`: "person" or "organization"
- `aliases`: Other names or references used for this party, comma-separated

**Canonical names — one name per party, per case.**
Each party gets exactly **one** `party_name`, used identically in every document. Choose it in this order:
1. **If the cross-document context block names this party, use that name exactly** — including capitalisation and punctuation. The graph connects parties by name; a different form creates a second, duplicate party.
2. **Otherwise**, use the party's fullest form in this document — full legal name where available ("George Phillips", not "Attorney Phillips"; "Catholic Family Service", not "CFS").

**Every other form goes in `aliases`**, comma-separated: titles ("Attorney Phillips"), short forms ("Phillips", "CFS"), role references ("Defendant Phillips", "the Court"), and any misspelling the document itself uses. Aliases are how a reader finds the party from the document's own words — they are not optional, and nothing is lost by canonicalising.

**This overrides any instinct to copy the document's wording into `party_name`.** The document's wording is preserved twice already: in `verbatim_quote` and in `aliases`. `party_name` is the graph's join key, not a transcription.

**Extract as Party:**
- The respondent (the person answering under oath) — always the first Party
- The requesting party / plaintiff (the person who posed the questions) — always the second Party
- Every person named in questions or answers: attorneys, witnesses, family members, judges, professionals, accountants, auctioneers, health care providers
- Every organization named: companies, courts, agencies, firms, banks, service organizations

**Do NOT extract as Party:**
- The word "Plaintiff" or "Defendant" alone — these are roles, not named entities
- The law firm from the caption attorney block — unless it appears substantively in answers
- Pronouns ("he", "she", "they") — these refer to already-extracted parties
- Court names (e.g., "Bay County Probate Court") — these are jurisdictions, not parties
- Cities, states, counties, geographic locations

### Evidence
Each numbered Q&A pair is ONE Evidence entity. This is the core extraction target. Every Q&A pair gets extracted — no exceptions.

The `verbatim_quote` comes from the ANSWER portion — the sworn response. The question goes in the `question` property. The answer goes in the `answer` property AND the verbatim_quote.

**Properties:**
- `title`: Short descriptive title summarizing what this Q&A reveals (NOT just "Q73" — describe the substance)
- `question`: The interrogatory question text. For multi-part questions, include the main question and all sub-parts.
- `answer`: The full sworn response, exactly as written
- `paragraph`: Interrogatory number (e.g., "Q73", "Q14", "RFA 9")
- `page_number`: PDF page number where this Q&A appears
- `page_note`: If the Q&A spans multiple pages, note the range (e.g., "pages 10-11")
- `kind`: Always "testimonial" for discovery responses
- `evidence_strength`: See classification table below
- `statement_type`: See classification table below
- `significance`: Why this Q&A matters for trial preparation — what does it prove, contradict, or reveal?
- `weight`: 1-10 evidentiary weight (admissions 8-10, evasive 5-7, referrals 2-4)
- `pattern_tags`: Comma-separated tags from the CLOSED vocabulary — see taxonomy below; omit entirely if none apply
- `event_date`: Date referenced in the Q&A if applicable — ISO-8601, see below

**Date format — `event_date` and `statement_date` MUST be ISO-8601.**
Write dates as `YYYY-MM-DD`. When the source is only less precise, write only what it states: `YYYY-MM` for a month, `YYYY` for a year. Never pad a partial date with a guessed day or month — `YYYY-MM` is a complete, correct answer.

**One format, always — never a range.** If the source describes a span ("from January 2019 through June 2020"), record the START date only (`2019-01`). The span itself stays in the verbatim quote, where a reader can see it exactly as written. A range in a date property cannot be sorted, compared, or placed on a timeline.

The prose form stays where it belongs: in `verbatim_quote`, exactly as the document writes it. The document says "November 16, 2009"; the quote keeps that, and `event_date` is `2009-11-16`.

**If the source states no date, OMIT the property entirely.** Do not guess, do not infer from context, do not use the document's own date as a substitute. An absent date is honest; a wrong date is a defect that propagates into every chronology built from this graph.

**Classifying statement_type — this is critical:**

| statement_type | When to use | Example |
|---|---|---|
| `admission` | Respondent clearly acknowledges or confirms a fact | "That is my recollection." or "Yes." or "That would be correct." |
| `partial_admission` | Respondent admits some facts while qualifying or denying others | "I recall comments made by other interested parties however I also concluded..." |
| `denial` | Respondent clearly denies a fact | "No." or "Not that I recall." or "I do not have any professional or personal relationship with..." |
| `evasive` | Respondent deflects, says "documents speak for themselves", gives a non-answer, or says "see prior interrogatory" without substance | "The statements I made on the record speak for themselves." or "My statements on the record stand on their own merit." |
| `objection` | Respondent raises a legal objection (privilege, relevance, etc.) — may or may not include a partial answer | "I object on the ground of attorney client privilege. Without waiving that objection, I believe I had a conversation..." |
| `referral` | Respondent refers to another source for the answer without providing substance | "The appearances filed in the probate court document the names..." or "The accountings provide the best information..." or "See the response to the prior interrogatory." |

**Classifying evidence_strength:**

| evidence_strength | When to use |
|---|---|
| `sworn_party_admission` | statement_type is "admission" or "partial_admission" — the respondent acknowledged a fact under oath |
| `sworn_party_denial` | statement_type is "denial" — the respondent denied a fact under oath |
| `sworn_party_evasion` | statement_type is "evasive", "referral", or "objection" — the respondent avoided answering directly |

**Assigning pattern_tags — tag when you see these patterns:**

- `selective_enforcement`: The question asks whether the respondent treated one party differently than another, and the answer confirms or reveals different treatment. Example: "Were sanctions ever sought against Nadia Awad?" → "No" (but sanctions WERE sought against Marie).
- `disparagement`: The answer or the question reveals that the respondent characterized, belittled, or dismissed a party's claims. Example: "Did you characterize Marie Awad's claim as unintelligible?" → "I may have used that characterization."
- `evasive_responses`: The answer is non-substantive — "documents speak for themselves", "see prior interrogatory", "I do not recall" on a topic the respondent should remember.
- `financial_misconduct`: The Q&A involves unauthorized fees, improper charges, undisclosed financial arrangements, or mishandling of funds.
- `secrecy`: The answer reveals concealment, failure to disclose, or claims of privilege to avoid revealing information.
- `lies_under_oath`: The answer contradicts known facts from other documents (you may not know this yet in pass 1 — tag only when the contradiction is within this same document).
- `conflict_of_interest`: The Q&A reveals undisclosed relationships, dual roles, or financial entanglements.
- `coordination`: The answer suggests synchronized actions or communications between parties who should be independent.
- `property_mismanagement`: The Q&A involves improper handling, valuation, or disposal of estate or trust property.
- `fee_allocation`: The Q&A involves how costs, fees, or expenses were allocated between parties — especially if allocated disproportionately.
- `denial_of_due_process`: The Q&A reveals that a party was denied fair procedure, notice, or opportunity to be heard.

Multiple tags can apply to one Q&A. Separate with commas: "selective_enforcement,evasive_responses"

**CLOSED VOCABULARY — use ONLY these tags.** These are the trial-prep query patterns a discovery response can evidence.

- `selective_enforcement`
- `disparagement`
- `evasive_responses`
- `financial_misconduct`
- `secrecy`
- `lies_under_oath`
- `conflict_of_interest`
- `coordination`
- `property_mismanagement`
- `fee_allocation`
- `denial_of_due_process`

If a pattern you see is not in this list, leave `pattern_tags` off entirely and describe the pattern in `significance` — do not invent a tag. A tag outside this list will not match any query and is worse than no tag.

**Output format for `pattern_tags`:** a comma-separated string of tags drawn from the list above (e.g. `"selective_enforcement,disparagement"`). **When no tag applies, OMIT the property entirely — never emit an empty string.** An absent property means "no pattern identified"; an empty string is a value that means nothing and clutters every query that reads this field.

*(Reserved — not for this document type: `misrepresentation`, `evasion`, `admission_against_interest`, `concealment` belong to `appellate_brief_pass1_v5_3.md` and must not be used here.)*


## Worked Examples

### Example 1 — An admission (strongest evidence):

Question (Q74): "Is it true that Mr. Awad indicated on the video that the $50,000 was not gifted to his daughters and that he desired to have that money returned to him?"

Answer: "That is my recollection."

→ Extract as Evidence:
- title: "Phillips confirms Emil wanted $50,000 returned"
- question: (the full question text)
- answer: "That is my recollection."
- verbatim_quote: "That is my recollection."
- paragraph: "Q74"
- page_number: 22
- statement_type: "admission"
- evidence_strength: "sworn_party_admission"
- significance: "Phillips admits under oath that Emil Awad wanted the $50,000 returned — directly supports complaint allegation that sisters converted the money"
- weight: 10
- pattern_tags: "financial_misconduct"

### Example 2 — An evasive response (evidence of concealment):

Question (Q33): "You asserted to the Court that you were repulsed by the video... indicating that it was 'something out of North Korea.' With respect to that video, indicate the following: a. What efforts you undertook to investigate... b. Were you repulsed by the statements..."

Answer: "The statements I made on the record speak for themselves. I do not believe that I made an admission and/or committed perjury in connection with this matter and any statement so indicating is a mischaracterization of my response."

→ Extract as Evidence:
- title: "Phillips refuses to explain 'North Korea' characterization"
- question: (summarize the multi-part question)
- answer: "The statements I made on the record speak for themselves. I do not believe that I made an admission and/or committed perjury in connection with this matter and any statement so indicating is a mischaracterization of my response."
- verbatim_quote: "The statements I made on the record speak for themselves. I do not believe that I made an admission and/or committed perjury in connection with this matter and any statement so indicating is a mischaracterization of my response."
- paragraph: "Q33"
- page_number: 12
- statement_type: "evasive"
- evidence_strength: "sworn_party_evasion"
- significance: "Phillips refuses to directly address the 'North Korea' characterization — neither denying nor explaining it. Evasion suggests the characterization was made and cannot be defended."
- weight: 7
- pattern_tags: "disparagement,evasive_responses"

### Example 3 — A denial that reveals selective treatment:

Question (Q12): "Were sanctions ever sought against Nadia Awad in connection with her retention and/or involvement of multiple attorneys..."

Answer: "No."

→ Extract as Evidence:
- title: "No sanctions sought against Nadia Awad for multiple attorneys"
- question: (the question text)
- answer: "No."
- verbatim_quote: "No."
- paragraph: "Q12"
- page_number: 4
- statement_type: "denial"
- evidence_strength: "sworn_party_denial"
- significance: "Phillips confirms no sanctions were sought against Nadia for the same conduct (multiple attorneys) for which Marie was sanctioned — evidence of selective enforcement"
- weight: 9
- pattern_tags: "selective_enforcement"

### Example 4 — A referral (refusal to answer directly):

Question (Q6): "When did Alex Luvall first contact you in connection with his representation of Nadia Awad?"

Answer: "See the response to the prior interrogatory."

→ STILL extract as Evidence — do NOT skip:
- title: "Phillips refuses to specify when Luvall contact occurred"
- question: (the question text)
- answer: "See the response to the prior interrogatory."
- verbatim_quote: "See the response to the prior interrogatory."
- paragraph: "Q6"
- page_number: 3
- statement_type: "referral"
- evidence_strength: "sworn_party_evasion"
- significance: "Phillips redirects to a prior response rather than providing a direct answer about the timeline of Luvall's involvement"
- weight: 3
- pattern_tags: "evasive_responses"

### Example 5 — An objection with partial admission:

Question (Q7): "On December 1, 2009 you discussed the role of Alex Luvall. a. What was the substance of that discussion? b. What role did you decide Alex Luvall played in the estate?"

Answer: "I object to the request on the ground of attorney client privilege. Without waiving that objection I believe I had a conversation with staff of Catholic Family about Mr. Luvall and whether he was getting involved."

→ Extract as Evidence:
- title: "Phillips claims privilege but admits CFS conversation about Luvall"
- question: (the question text with sub-parts)
- answer: "I object to the request on the ground of attorney client privilege. Without waiving that objection I believe I had a conversation with staff of Catholic Family about Mr. Luvall and whether he was getting involved."
- verbatim_quote: "I object to the request on the ground of attorney client privilege. Without waiving that objection I believe I had a conversation with staff of Catholic Family about Mr. Luvall and whether he was getting involved."
- paragraph: "Q7"
- page_number: 3
- statement_type: "objection"
- evidence_strength: "sworn_party_evasion"
- significance: "Phillips invokes privilege to avoid revealing substance of discussion about Luvall's role, but inadvertently admits he discussed Luvall with CFS staff — evidence of coordination between Phillips and CFS"
- weight: 6
- pattern_tags: "secrecy,coordination"

## Extraction Strategy — Follow This Order Exactly

### Step 1: Extract ALL Party entities

Read through the entire document. For every person or organization mentioned — in questions, in answers, in the opening statement — create a Party entity. Use their full legal name from the first mention. Assign the correct role.

The respondent should always be the first Party entity. The requesting party / plaintiff (who posed the questions) should be second.

### Step 2: Extract ALL Evidence entities

Go through the document sequentially, from Q1 to the last question. For each numbered Q&A pair:

1. Read the question carefully. Understand what is being asked.
2. Read the answer carefully. Classify the statement_type.
3. Write a short descriptive title (not just "Q73" — describe the substance).
4. Capture the question in the `question` property. For multi-part questions, include all sub-parts.
5. Capture the answer in the `answer` property. Include the full response.
6. Set the verbatim_quote to the exact text from the answer portion.
7. Determine evidence_strength based on statement_type.
8. Write significance — why does this Q&A matter for the case? What allegation does it support? What pattern does it reveal?
9. Assign pattern_tags if any patterns are present.
10. Set the weight (1-10).
11. Move to the next Q&A.

**For multi-part questions (a, b, c, d...):** Extract as ONE Evidence entity. The question property should include all sub-parts. The answer property should include the full consolidated response.

**For cross-reference answers ("See the response to the prior interrogatory"):** Extract as a separate Evidence entity with statement_type="referral". Do NOT merge it with the referenced interrogatory.

**For very short answers ("No.", "Yes.", "That would be correct."):** These are substantive sworn statements. A one-word denial or admission under oath is binding. Extract them with full significance explaining what the answer means in context.

### Step 3: Verify completeness

Run through the completeness checklist at the end of this document before returning your output.

## Schema — Entity Types and Properties

{{schema_json}}

## Extraction Rules

{{global_rules}}

## Additional Instructions from Administrator

{{admin_instructions}}

## Prior Context from Other Documents

{{context}}

## Document Text

{{document_text}}

## Output Format

Return a single JSON object with one top-level array: `"entities"`. Do NOT include a "relationships" key — relationships come in Pass 2.

### Entity format

Each entity must have these fields:
- `"entity_type"`: "Party" or "Evidence"
- `"id"`: unique identifier — "party-001", "party-002", "evidence-001", "evidence-002", etc.
- `"label"`: short human-readable label (party name for Party, descriptive title for Evidence)
- `"properties"`: object with properties defined in the schema above
- `"verbatim_quote"`: for Evidence — exact text from the ANSWER portion of the document. For Party — null.

**CRITICAL: verbatim_quote goes at the TOP LEVEL of each entity, NOT inside properties.**

### Example entity (Party):
```json
{
  "entity_type": "Party",
  "id": "party-001",
  "label": "George Phillips",
  "properties": {
    "party_name": "George Phillips",
    "role": "respondent",
    "party_type": "person",
    "aliases": "Defendant Phillips, Phillips"
  },
  "verbatim_quote": null
}
```

### Example entity (Evidence — admission):
```json
{
  "entity_type": "Evidence",
  "id": "evidence-074",
  "label": "Phillips confirms Emil wanted $50,000 returned",
  "properties": {
    "title": "Phillips confirms Emil wanted $50,000 returned — contradicts gift narrative",
    "question": "Is it true that Mr. Awad indicated on the video that the $50,000 was not gifted to his daughters and that he desired to have that money returned to him?",
    "answer": "That is my recollection.",
    "paragraph": "Q74",
    "page_number": 22,
    "kind": "testimonial",
    "evidence_strength": "sworn_party_admission",
    "statement_type": "admission",
    "significance": "Phillips admits under oath that Emil Awad wanted the $50,000 returned — directly supports the complaint's conversion allegation and destroys the gift narrative",
    "weight": 10,
    "pattern_tags": "financial_misconduct"
  },
  "verbatim_quote": "That is my recollection."
}
```

### Example entity (Evidence — evasive):
```json
{
  "entity_type": "Evidence",
  "id": "evidence-041",
  "label": "Phillips deflects on auction cost overrun",
  "properties": {
    "title": "Phillips deflects to accounting records rather than confirming $6,000 auction loss",
    "question": "Is it true that the costs of the auction exceeded the assets realized by approximately $6,000.00?",
    "answer": "The accountings provide the best information regarding the costs and revenue generated by the auction.",
    "paragraph": "Q41",
    "page_number": 15,
    "kind": "testimonial",
    "evidence_strength": "sworn_party_evasion",
    "statement_type": "referral",
    "significance": "Phillips deflects to accounting records rather than directly confirming the $6,000 loss — the records themselves confirm it, making this referral an implicit admission",
    "weight": 6,
    "pattern_tags": "financial_misconduct,evasive_responses"
  },
  "verbatim_quote": "The accountings provide the best information regarding the costs and revenue generated by the auction."
}
```

Return ONLY the JSON object. No markdown fences, no explanation, no preamble.

## Completeness Checklist — Verify Before Returning

**Party checks:**
- [ ] Did I extract the respondent as the first Party?
- [ ] Did I extract the requesting party / plaintiff as the second Party?
- [ ] Did I extract EVERY person named in questions or answers?
- [ ] Did I extract EVERY organization named in questions or answers?
- [ ] Did I include attorneys, judges, witnesses, family members, accountants, auctioneers, and other professionals?
- [ ] Did I avoid extracting "Plaintiff"/"Defendant" without a name as Party entities?
- [ ] Did I avoid extracting court names, cities, or states as Party entities?

**Evidence checks:**
- [ ] Did I create an Evidence entity for EVERY numbered Q&A pair, from Q1 through the last question?
- [ ] Does every Evidence entity have verbatim_quote at the TOP LEVEL (not inside properties)?
- [ ] Is the verbatim_quote the exact text from the ANSWER portion (not the question)?
- [ ] Did I classify every statement_type correctly (admission, denial, evasive, partial_admission, objection, referral)?
- [ ] Did I set evidence_strength correctly based on statement_type?
- [ ] Did I write a descriptive title (not just "Q73") for every Evidence entity?
- [ ] Did I write significance explaining why this Q&A matters for the case?
- [ ] Did I assign pattern_tags where applicable?
- [ ] Did I include page_number for every Evidence entity?

**Completeness negative checks:**
- [ ] Did I accidentally SKIP an evasive or "see prior" response? (These MUST be extracted.)
- [ ] Did I accidentally SKIP a document production request? (These MUST be extracted.)
- [ ] Did I accidentally SKIP a one-word answer like "No" or "Yes"? (These are binding sworn statements.)
- [ ] Did I create any relationships? (I should NOT have — relationships come in Pass 2.)
- [ ] Did I extract text from the caption or signature block as Evidence? (I should NOT have.)
- [ ] Did I combine multiple Q&A pairs into a single Evidence entity? (Each numbered question must be separate.)

Return ONLY the JSON object with an "entities" array. No "relationships" key. No markdown fences, no explanation, no preamble.
