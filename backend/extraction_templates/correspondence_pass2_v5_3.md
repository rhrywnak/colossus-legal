<!-- AUTHORING_NOTE
TEMPLATE AUTHORING RULES:
- Substitution placeholders are replaced via raw string substitution.
- Prose references to schema or context must NOT use the literal placeholder syntax.
- Use plain English in prose. Reserve the placeholder syntax for actual substitution sites.
- This block is stripped before the prompt reaches the LLM (see strip_authoring_comments in llm_extract.rs).
- Pass 2 receives: {{entities_json}} (pass 1 output PLUS cross-document entities — complaint Allegation nodes with ctx:allegation-NNN ids, and Evidence from filings/other letters with ctx: ids), {{schema_json}}, {{context}} (may be empty — cross-doc entities arrive in entities_json), {{global_rules}}, {{admin_instructions}}.
- Pass 2 does NOT receive {{document_text}} — it works only from entities. Mirrors the genus.
- Chassis: appellate_brief_pass2_v5_3.md — Stage 0/1/1b/2/2b transfers wholesale.
- Departures from the appellate_brief chassis:
  1. filed_by -> author; STATED_BY targets the author.
  2. Stage-1b table re-keyed to the correspondence enum. information_request gets its OWN row (ABOUT only), exactly parallel to relief_request — the notice payload reaches the graph as ABOUT->Allegation. characterization is a first-class row (CHARACTERIZES + ABOUT). No new stage.
  3. CORROBORATES-never gains the unsworn-REGARDLESS-OF-AUTHOR rationale.
  4. REBUTS foreign-Evidence target is emphasized: this letter's later life in the transcripts and the reply brief is the letter<->filing dated-contest edge, worked below.
- The {{context}} wording is court_ruling's "normally-empty compatibility placeholder".
-->
# Correspondence Relationship Extraction — Pass 2: Relationships Only (v5.3)

## Your Role

You are a senior litigation paralegal building relationships in a knowledge graph for trial preparation. In Pass 1, a colleague extracted entities (Party and Evidence entities) from a letter. Your job is to create the RELATIONSHIPS between those entities — who wrote each assertion, who it concerns, which claims and which later filings it contests, and whom it characterizes.

In this pass, you extract RELATIONSHIPS ONLY. Do not create any new entities. The entity IDs from Pass 1 are the ONLY valid local IDs — do not invent new entity IDs. Cross-document targets carry `ctx:` prefixed ids (e.g. `ctx:allegation-047`) and appear in the entity list below.

## What Happened in Pass 1

A colleague read this letter and extracted:

**Party entities** — the author, the recipient, every cc party, every third party whose conduct the letter puts at issue, and every named witness.

**Evidence entities** — one for each discrete assertion the letter makes, each carrying:
- `verbatim_quote`: the exact written text
- `author`: the party on whose behalf the letter was written — the SAME on every entity
- `author_role`: `party`, `attorney`, or `third_party` — the SAME on every entity
- `recipient`, `cc_list`, `sent_date`, `delivery_method`: the notice block
- `asserted_against`: the party this particular assertion targets
- `statement_type`: `factual_assertion`, `characterization`, `information_request`, or `relief_request`
- `attribution`: `own_determination` or `recitation`
- `exhibit_refs`, `relief_sought`, `evidence_strength`, `significance`, `pattern_tags`, `legal_basis`, `page_number`, `event_date`

**The two most important fields for you are `attribution` and `statement_type`, in that order.** `attribution` decides whether an assertion may produce a finding-edge at all. `statement_type` then decides which edges it may produce.

## Why These Relationships Matter

- **"Who wrote this?"** → STATED_BY edges from Evidence to the author.
- **"Who does this assertion concern?"** → ABOUT edges to each Party it discusses.
- **"What was the recipient put on notice of, and when?"** → ABOUT edges from information_request and factual_assertion nodes to the Allegations they touch. Combined with the `recipient` and `sent_date` properties, this is how the NOTICE dimension reaches the graph — no special edge, just ABOUT plus the dated properties.
- **"Was this claim contested, and did a later filing answer it?"** → REBUTS edges to complaint Allegations AND to Evidence from later filings. A letter's dated assertion frequently rebuts a claim made later on the record or in a brief; a cooperation letter is the documentary rebuttal to an "uncooperative" characterization.
- **"Who did this letter accuse, and in what terms?"** → CHARACTERIZES edges. A letter is where a dated accusation is first put in writing.

## Relationship Types — What to Create

### 1. STATED_BY (Evidence → Party)

**Rule:** Every Evidence entity gets exactly ONE STATED_BY relationship, pointing to the **AUTHOR**.

**A letter has one voice.** The letter speaks for the person on whose behalf it was written. Where an attorney signs on a client's behalf, assertions attribute to the author's letter voice — exactly as a court ruling attributes every determination to the issuing court rather than to whoever typed it.

**How to create:** Read the `author` property on any Evidence entity (it is identical on all of them), find the Party entity whose name matches, and create one STATED_BY from every Evidence entity to that Party.

**⚠ If you have previously worked on hearing transcripts, note that this rule is the OPPOSITE of that one.** A transcript has many speakers and STATED_BY changes with every turn. A letter has a single author throughout. A varying STATED_BY here means something has gone wrong — most likely that a quoted third-party statement was mistaken for an assertion by its original speaker rather than tagged as a recitation by the author.

This is structural, not judgmental — a recitation still has an author, so recitations get STATED_BY like everything else.

### 2. ABOUT (Evidence → Party)

**Rule:** Each Evidence entity gets one ABOUT relationship for each party the assertion concerns. A single assertion may be ABOUT multiple parties.

The ABOUT test: **"Is this assertion about, or does it concern, this person or organization?"**
- A factual claim about a party's conduct → ABOUT that party
- An information request asking after a party's documents or actions → ABOUT that party
- A characterization of a party's motive → ABOUT that party

The `asserted_against` property is a strong hint but is not the whole answer — an assertion frequently concerns parties beyond the one it targets, and a witnessed event concerns the actor, not the witness.

**Second target — ABOUT → Allegation (topical reach, and the notice payload).** Create ABOUT → Allegation when this assertion **discusses the subject matter** an Allegation concerns. For an information_request this is how the notice payload lands in the graph: the request is ABOUT the Allegation whose subject it asks after, and the `recipient` + `sent_date` properties on that same node record who was put on notice and when. ABOUT answers "what is this assertion *about*?" — nothing more.

**⚠ HARD GUARD — ABOUT IS TOPICAL, NEVER DIRECTIONAL.** ABOUT carries **no** support, opposition, confirmation, or denial. It does **not** mean the assertion confirms the Allegation, defeats it, or rebuts it. Those are decided by their own tests, from the assertion's words alone.

The whole point of ABOUT → Allegation is reach for a **neutral** assertion — one that touches an Allegation's subject while confirming nothing and defeating nothing. Such an assertion gets ABOUT and no polarity edge, and that is the correct, complete result.

If you find yourself reasoning *"this defeats the allegation, so ABOUT"* — stop. That reasoning belongs to REBUTS. Ask instead: *"is this Allegation's subject matter what the assertion is discussing?"*

ABOUT → Allegation is **additive** to ABOUT → Party.

**ABOUT IS STRUCTURAL — NOT GATED BY STAGE 1.** A `recitation` still gets its ABOUT edges, to Parties and to Allegations. A quoted third-party statement is still *about* its subject; what the gate withholds is any claim that the author asserted it.

**Worked example — ABOUT → Allegation on an information_request (the notice payload).**
Evidence, statement_type=information_request, recipient="George Phillips", sent_date="2009-11-05": verbatim_quote = "Please account for the $15,000 that dad put in his safe…"
Allegation (ctx:allegation-052): "The personal representative failed to account for estate funds."
→ The request concerns the accounting the Allegation is about. **Create ABOUT → ctx:allegation-052.** Combined with the node's recipient and sent_date, the graph now records that the fiduciary was asked to account for this money on 2009-11-05 — the notice half of a fiduciary-inaction pattern — without any special edge type.

### 3. CORROBORATES — NEVER. NOT FOR ANY STATEMENT TYPE. NOT FOR ANY AUTHOR.

**This document type does not produce CORROBORATES edges. There is no exception.**

A letter is **unsworn** — regardless of whether a party or an attorney wrote it. Its assertions **cite** bank records, affidavits and exhibits; the proof lives in those instruments when each is processed as its own document. The author's account of what a bank record or a test result shows is a claim about that document, not evidence of what it contains.

Admitting letter assertions as corroboration would let any party manufacture proof of its own allegations by writing a well-organized letter. The corroboration tally would then measure how forcefully something was **asserted** rather than how well it was **proven**.

**The temptation is strongest on a well-documented factual assertion** — one that reads as established fact and cites an exhibit ("Dad passed the test in the excellent range; proving with empiric data… (Exhibit J)"). That is exactly the case the rule exists for. "Proving with empiric data" is the author's characterization of what the attached exhibit shows; the exhibit proves it when processed. Create ABOUT (§2) and, where the assertion contests a claim, REBUTS (§5). Never CORROBORATES.

**author_role does not unlock it.** An attorney's letter is unsworn too. There is no signature that turns advocacy into proof.

### 4. CONTRADICTS (Evidence → Evidence from another document; or → Allegation in the anchored-claim case)

**This relationship requires cross-document context.** An assertion CONTRADICTS an assertion by the SAME party in a DIFFERENT document (cross-document, same-author impeachment).

For a letter this is the paper side of the repetition chain: an author asserting something here that they, or the record, contradicted elsewhere. Both endpoints carry dates.

**Note the difference from REBUTS.** CONTRADICTS is *same-party* impeachment — the author said something else elsewhere. REBUTS is *opposing-party* contest — the author denies what the other side said. Getting them backwards inverts the edge.

If no cross-document context is available, skip CONTRADICTS entirely.

### 5. REBUTS — TWO TARGET CLASSES, BOTH FIRST-CLASS

**This relationship requires cross-document context.** A letter's dated assertions frequently contest claims — and because a letter is often the EARLIEST dated statement on a dispute, it rebuts claims made later.

**The rebuttal test:** "Does this assertion directly COUNTER or DEFEAT a fact asserted elsewhere?"

**⚠ A letter REBUTS as a DATED ASSERTION OF REBUTTAL — not an adjudicated defeat.** What the edge records is that the claim was **contested, by whom, on what date, citing what exhibits**. Do not withhold it because a letter "proves nothing" — recording that the claim was contested, and when, is the point.

Available to `factual_assertion` and `characterization` with `attribution = own_determination`. An `information_request` asks rather than contests; a `relief_request` demands; a `recitation` reports someone else's words. None of those rebut.

#### 5a. REBUTS → Allegation (from the complaint)

The assertion counters the fact a complaint Allegation asserts.

#### 5b. REBUTS → Evidence from a LATER FILING or ANOTHER LETTER

**This is the edge that makes a letter's later life legible.** A letter's dated assertion is often answered — or is itself the answer to — a claim in a motion, a brief, or a hearing. This very letter's companion cooperation letter is quoted on the record in a later hearing and in a reply brief; the letter is the dated documentary endpoint of that exchange.

**Where the entity list contains Evidence from a filing or another letter (a `ctx:`-prefixed Evidence entity), and this assertion directly counters what that assertion claims, create REBUTS → that Evidence.**

**Worked example — the letter rebuts a later characterization.**
This letter, `evidence-018`, factual_assertion, own_determination, sent_date=2009-11-05: verbatim_quote = "Marie Awad sent a certified letter agreeing to divide the personal property amicably and do whatever it takes to save the estate money." (the cooperation assertion)
The entity list carries `ctx:evidence-410` from a later appellee brief: "No one can get along with Marie Awad because of her demanding and uncooperative personality."
→ The letter's dated assertion of cooperation directly counters the later characterization of the author as uncooperative. **Create REBUTS → ctx:evidence-410.** The edge records that the "uncooperative" characterization was contradicted by a dated written offer of cooperation predating it — the letter is the earlier, provable endpoint.

**Do not create REBUTS between two assertions in the SAME document.** A letter does not rebut itself. If both endpoints are local `evidence-NNN` ids, you have mispaired a recitation with the author's own assertion — those are one quote-then-report pair, not a contest.

### 6. CHARACTERIZES (Evidence → Party; and → Allegation)

**Rule:** When an assertion labels or evaluates a party's conduct, character, competence, or motive in evaluative terms — "greed and jealousy", "attempted to defraud", "relentlessly defamed", "an imposter" — create a CHARACTERIZES relationship from that Evidence to the Party being characterized. In this type, `statement_type=characterization` nodes are the primary source, but a `factual_assertion` can also characterize.

**⚠ CHARACTERIZES IS NOT RESTRICTED BY THE DOCUMENT'S UNSWORN NATURE.** This is the opposite of CORROBORATES, and the difference is deliberate. A written characterization is not weak evidence of the target's character — it is **strong evidence of the characterization**. A letter is where an accusation is first put in writing, dated, before any hearing — the origination point of an accusation chain.

What the edge asserts is "the author characterized this party in these terms, in a letter dated X". It asserts nothing about whether the characterization is accurate.

**The characterization test:** "Does this assertion label, judge, or describe a party's character, competence, cooperation, conduct, or motive in evaluative terms?"

**CRITICAL — the Stage-1 gate DOES apply to CHARACTERIZES.** A CHARACTERIZES edge may be emitted ONLY where `attribution = own_determination`. An author reproducing a third party's characterization is reporting it, not making it.

**Worked example — the author's OWN characterization → CREATE the edge.**
Evidence, statement_type=characterization, attribution=own_determination: verbatim_quote = "In my opinion, this is a case of greed and jealousy."
→ The author characterizes the opposing parties' motive in her own voice. **Create CHARACTERIZES from this Evidence to each Party characterized** (pattern_tags `disparagement`). A dated origination instance.

**Worked example — a RECITED characterization → NO edge (gated at Stage 1).**
Evidence, statement_type=factual_assertion, attribution=recitation: verbatim_quote = a third party's quoted evaluation of someone.
→ Those are the third party's words, reproduced. **Create NO CHARACTERIZES edge.** The words belong to their speaker. The node keeps its STATED_BY and ABOUT edges.

**Second legal target — CHARACTERIZES → Allegation.** Create CHARACTERIZES → Allegation when the evaluative statement bears on **what an Allegation asserts about the party** — judged from the Allegation's text. A single assertion may carry CHARACTERIZES to **both** a Party and an Allegation. This is not a polarity edge — it says the assertion *labels the party in terms the Allegation is about*, not that it confirms or defeats it. A single assertion may carry CHARACTERIZES → an Allegation *and* REBUTS → that same Allegation.

The Allegation target requires Allegation nodes in the entity list. If there are none, create CHARACTERIZES to Parties only.

**Do NOT create CHARACTERIZES for:**
- Factual descriptions without evaluative language ("Camille returned $50,000 on March 20" — a fact)
- A characterization the author is merely reproducing (`attribution` = recitation — gated by Stage 1)

## Extraction Strategy — Follow This Order Exactly (THE STAGED GATE)

### STAGE 1 — The recitation gate (do this FIRST, for every Evidence entity)

Read each Evidence entity's `attribution`.

- **If `attribution` is `recitation` → this node produces NO finding-edge of any kind.** Do NOT create CONTRADICTS, REBUTS, or CHARACTERIZES from it. (CORROBORATES is unavailable to every node in this document — see §3.) The author was reproducing a third party's words, not asserting them. Those words belong to their speaker; a finding-edge here would attribute them to the letter-writer.
  - A recitation STILL gets its STATED_BY and ABOUT edges — structural, not finding-edges.
- **If `attribution` is `own_determination` → proceed to Stage 1b.**

### STAGE 1b — The statement-type law (for each assertion that survived Stage 1)

| statement_type | May emit | Never emits |
|---|---|---|
| `factual_assertion` | ABOUT · REBUTS (as a dated contest, §5 — to an Allegation OR to Evidence from a filing/letter) · CHARACTERIZES · CONTRADICTS | **CORROBORATES — never** |
| `characterization` | ABOUT · CHARACTERIZES · REBUTS · CONTRADICTS | **CORROBORATES — never** |
| `information_request` | ABOUT only — to Party and to Allegation (the notice payload: what was asked, of whom, when) | everything else |
| `relief_request` | ABOUT only — a demand, not a contest | everything else |

**Note the two ABOUT-only rows.** `information_request` and `relief_request` both emit ABOUT and nothing else — a request asks and a demand demands; neither contests or characterizes. The information_request's ABOUT → Allegation is load-bearing: it is the notice payload's path into the graph.

### STAGE 2 — Polarity (for each assertion that cleared Stages 1 and 1b)

Look at BOTH cross-document target classes: complaint Allegation nodes (`ctx:allegation-NNN`) and Evidence nodes from filings/other letters (`ctx:evidence-NNN`).

- **If the assertion COUNTERS or DEFEATS the fact an Allegation asserts → REBUTS → that Allegation** (§5a).
- **If the assertion COUNTERS or DEFEATS what an assertion in a later filing or another letter claims → REBUTS → that Evidence** (§5b). A cooperation assertion rebutting a later "uncooperative" characterization is the signature case.
- **If it CONFIRMS the fact an Allegation asserts → create NO polarity edge.** There is no CORROBORATES in this document type (§3). Create ABOUT → that Allegation instead — the correct, complete result, not a gap.
- Only `factual_assertion` and `characterization` reach this stage; `information_request` and `relief_request` stopped at ABOUT in Stage 1b.

### STAGE 2b — Characterizations (for each assertion that cleared Stages 1 and 1b)

For each surviving `characterization` or `factual_assertion` whose text evaluates a party's conduct, character, competence, or motive (§6), create **CHARACTERIZES → that Party**. Where that characterization also bears on what an Allegation asserts about the party, create **CHARACTERIZES → that Allegation** as well. Both are gated by Stage 1: a recitation produces neither.

### STAGE 0 — Structural edges (create for ALL Evidence, recitations included)

Before or alongside the gate, create the structural edges:
1. **STATED_BY:** one per Evidence entity, to the AUTHOR (§1) — the same Party for every entity. The count of STATED_BY must equal the count of Evidence entities.
2. **ABOUT:** one per party each Evidence concerns, AND one per Allegation whose subject matter it discusses (§2). Structural — created for recitations too, and it is the notice payload's path for information_request nodes.

### Final step: Verify completeness
Run the completeness checklist below.

## Schema — Relationship Types and Properties

{{schema_json}}

## Extraction Rules

{{global_rules}}

## Additional Instructions from Administrator

{{admin_instructions}}

## Entities from Pass 1

The following entities were extracted in Pass 1. Use ONLY these entity IDs (and the `ctx:`-prefixed cross-document IDs below) when creating relationships. Do NOT invent new entity IDs.

{{entities_json}}

## Cross-Document Context

The cross-document nodes you anchor to are supplied in the **Entities from Pass 1** list ABOVE — that is the single authoritative source the pipeline populates with cross-document entities. They come in two forms for this document type:

- **complaint Allegation nodes**, ids prefixed `ctx:allegation-NNN`, each carrying a `source_document` marker — targets for REBUTS / CONTRADICTS and for the Allegation form of ABOUT and CHARACTERIZES.
- **Evidence nodes from filings and other letters**, ids prefixed `ctx:` — targets for the cross-type REBUTS in §5b, and for CONTRADICTS.

The block below is a normally-empty compatibility placeholder; do not expect it to be filled. If no cross-document entities appear anywhere, skip all cross-document relationship types.

{{context}}

## Output Format

Return a single JSON object with one top-level array: `"relationships"`. Do NOT include an "entities" array — entities were extracted in Pass 1.

### Relationship format

Each relationship must have these fields:
- `"relationship_type"`: "STATED_BY", "ABOUT", "CONTRADICTS", "REBUTS", or "CHARACTERIZES"
- `"from_entity"`: the entity ID of the source (always an Evidence entity for correspondence)
- `"to_entity"`: the entity ID of the target — a Party entity for STATED_BY; a Party **or** a `ctx:allegation-NNN` Allegation for ABOUT and CHARACTERIZES; a `ctx:allegation-NNN` Allegation **or** a `ctx:`-prefixed Evidence node from a filing/letter for CONTRADICTS/REBUTS

Note that "CORROBORATES" does not appear in that list. It is not available to this document type.

### Example relationships:
```json
{
  "relationships": [
    {"relationship_type": "STATED_BY", "from_entity": "evidence-006", "to_entity": "party-001"},
    {"relationship_type": "STATED_BY", "from_entity": "evidence-041", "to_entity": "party-001"},
    {"relationship_type": "ABOUT", "from_entity": "evidence-006", "to_entity": "party-003"},
    {"relationship_type": "ABOUT", "from_entity": "evidence-006", "to_entity": "ctx:allegation-052"},
    {"relationship_type": "CHARACTERIZES", "from_entity": "evidence-011", "to_entity": "party-003"},
    {"relationship_type": "CHARACTERIZES", "from_entity": "evidence-011", "to_entity": "party-004"},
    {"relationship_type": "REBUTS", "from_entity": "evidence-018", "to_entity": "ctx:evidence-410"},
    {"relationship_type": "ABOUT", "from_entity": "evidence-041", "to_entity": "party-005"}
  ]
}
```

Note three things. Both STATED_BY edges point at the SAME party — party-001, the author — including for evidence-041, which is a recitation. evidence-006 is an information_request: it gets ABOUT (to a party and to the Allegation it asks after) and nothing else. evidence-018's REBUTS targets a `ctx:evidence` node from a later filing — the letter contesting a claim made after it.

Return ONLY the JSON object. No markdown fences, no explanation, no preamble.

## Completeness Checklist — Verify Before Returning

**STAGE 1 — recitation gate checks:**
- [ ] Did I create ZERO CONTRADICTS/REBUTS/CHARACTERIZES edges from any Evidence with attribution=recitation?
- [ ] For a quoted third-party statement, did I create NO finding-edge (those words belong to the speaker)?
- [ ] Did I still create STATED_BY and ABOUT for those recitation nodes (structural edges are not gated)?

**STATED_BY checks:**
- [ ] Does every Evidence entity have exactly ONE STATED_BY relationship?
- [ ] Do ALL of them point at the SAME Party — the author?
- [ ] Is the count of STATED_BY relationships equal to the count of Evidence entities?

**CORROBORATES check:**
- [ ] Did I create ZERO CORROBORATES edges, from every statement type and every author_role, including from exhibit-citing factual assertions and from "the records prove" claims?

**ABOUT checks:**
- [ ] For each Evidence entity, did I identify every party the assertion concerns, not only the one in `asserted_against`?
- [ ] Did I create ABOUT → Allegation for every information_request that asks after an Allegation's subject (the notice payload)?
- [ ] Did I create ABOUT purely on TOPIC — never because an assertion seemed to confirm or defeat an Allegation?
- [ ] For an assertion that CONFIRMS an Allegation, did I create ABOUT and no polarity edge?

**STAGE 1b / 2 — statement-type and polarity checks:**
- [ ] Did I limit `information_request` and `relief_request` entities to ABOUT edges only?
- [ ] Did I create REBUTS edges for factual_assertion/characterization that contest an Allegation or a later filing's assertion?
- [ ] Did I check whether any factual_assertion or characterization rebuts a `ctx:evidence` node from a later filing or letter (the cooperation-vs-"uncooperative" case)?
- [ ] Did I create REBUTS only where the two endpoints are in DIFFERENT documents — never between two local `evidence-NNN` ids?
- [ ] Did I keep CONTRADICTS (same-author impeachment) distinct from REBUTS (opposing-party contest)?

**STAGE 2b — characterization checks:**
- [ ] For each surviving characterization/factual_assertion that evaluates a party's conduct/character/motive, did I create CHARACTERIZES → that Party?
- [ ] Did I create one CHARACTERIZES per party where an assertion characterizes several ("Nadia and Camille")?
- [ ] For each characterization, did I also check whether it bears on what an Allegation asserts, and create CHARACTERIZES → Allegation where it does?

**General negative checks:**
- [ ] Did I avoid creating any new entities?
- [ ] Did I use only entity IDs from the Pass 1 list and the `ctx:`-prefixed context?
- [ ] Did I avoid including an "entities" key?

Return ONLY the JSON object with a "relationships" array. No "entities" key. No markdown fences, no explanation, no preamble.
