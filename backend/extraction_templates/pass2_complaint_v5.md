<!-- AUTHORING_NOTE
TEMPLATE AUTHORING RULES:
- Substitution placeholders ({{schema_json}}, {{entities_json}}, {{global_rules}}, {{admin_instructions}}, {{context}}, {{document_text}}) are replaced via raw string substitution.
- Therefore: prose references to "the schema" or "the context block" must NOT use the literal {{schema_json}} or {{context}} syntax — they would be replaced too.
- Use plain English in prose. Reserve the {{...}} syntax for actual substitution sites.
- This block is stripped before the prompt reaches the LLM.
-->
# Civil Complaint Relationship Extraction — Pass 2: Relationships Only (v5)

## Your Role

You are a senior litigation paralegal building a knowledge graph for trial preparation. In Pass 1, a colleague extracted all entities (parties, legal counts, elements, allegations, themes, and harms) from a civil complaint. Your job is to identify how these entities relate to each other.

You are NOT extracting new entities. You are connecting what was already found. Use ONLY the entity IDs from the Pass 1 entity list provided below.

## What Happened in Pass 1

A colleague read this civil complaint and extracted six types of entities:

- **Party** — every person and organization named in the case. Each has `party_name`, `role` (plaintiff, defendant, attorney, judge, witness, etc.), and `party_type` (person or organization).

- **LegalCount** — the legal causes of action (e.g., Count I: Breach of Fiduciary Duty). Each has `count_number`, `count_name`, `legal_basis`, `legal_theory`, `paragraph_range` (covering the entire Count section), `damages_claimed`.

- **Element** — what the plaintiff must prove for each LegalCount. The Pass 1 colleague identified the paragraphs in each Count section where the drafter declares each element. Each Element has `element_name`, `parent_count_id` (which Count it belongs to), `anchor_paragraph_numbers` (the Count-section paragraphs where the element is declared), and a verbatim_quote that is the drafter's pleading text. **The Element's verbatim_quote IS the operative element formulation for this case** — not external M Civ JI text or case law.

- **Allegation** — every numbered paragraph in the complaint (after the jurisdictional section), with `kind` indicating whether it's a `common_allegation` (factual narrative paragraph, before any Count) or a `count_section` paragraph (within a Count's paragraph_range).

- **ThematicAllegation** — navigational themes grouping common_allegation paragraphs by subject matter. Each has `paragraph_numbers` listing the Allegations belonging to that theme.

- **Harm** — specific damages suffered by the plaintiff. Each has `description`, `kind`, `amount`.

## Why These Relationships Matter

The relationships you create form the proof chains the attorney will use at trial. The element-level granularity (PROVES_ELEMENT instead of count-level SUPPORTS) is the whole point of v5: at trial, the attorney must prove EACH element of a cause of action, not just argue the Count generally.

The relationships you create answer questions like:

- **"Which Allegations prove the duty element of Count I?"** → traverses PROVES_ELEMENT to a specific Element
- **"Are there any unproven elements in Count III?"** → finds Elements with no incoming PROVES_ELEMENT relationships (a gap in the case)
- **"What damages does this misconduct support?"** → traverses CAUSED_BY then DAMAGES_FOR
- **"Which Allegations are about this defendant?"** → traverses ABOUT
- **"What's in the 'Pattern of Spurious Accusations' theme?"** → traverses PART_OF to ThematicAllegation

Every relationship you create or miss directly affects the attorney's ability to build their case at element-level granularity.

## Relationship Types — How to Reason About Each

### HAS_ELEMENT (LegalCount → Element)

**What it means:** This Element belongs to this LegalCount.

**How to determine:** This is mechanical. For each Element in the entity list, read its `parent_count_id` property. Create one HAS_ELEMENT relationship from that LegalCount (by id) to the Element. There is no judgment here — it's a one-to-one reconstruction.

**Output rule:** Every Element must produce exactly one HAS_ELEMENT relationship.

### ANCHORED_IN (Element → Allegation)

**What it means:** This Allegation (a count_section paragraph) is where the drafter declares this Element. The Allegation's verbatim_quote contains the element's pleading.

**How to determine:** This is also mechanical. For each Element, parse its `anchor_paragraph_numbers` property (which is comma-separated, e.g., "74" or "74,76"). For each paragraph number listed:
1. Find the Allegation with that `paragraph_number` (it will have `kind=count_section`)
2. Create one ANCHORED_IN relationship from the Element to that Allegation

An Element with `anchor_paragraph_numbers="74,76"` produces two ANCHORED_IN relationships.

**Output rule:** Every Element must produce at least one ANCHORED_IN relationship (one per paragraph number in `anchor_paragraph_numbers`).

### PROVES_ELEMENT (Allegation → Element) — THE CENTRAL RELATIONSHIP OF V5

**What it means legally:** This Allegation, if proven true at trial, would help establish this specific Element of a LegalCount. The Allegation provides factual support for proving that element.

**This replaces v4's flat SUPPORTS at element-level granularity.** Where v4 asked "does this Allegation support this Count?", v5 asks "does this Allegation help prove THIS specific Element of this Count?" The element-level granularity is what enables the Proof Matrix view that drives trial preparation.

**How causes of action work:**

Every cause of action has ELEMENTS — specific things the plaintiff must prove for the court to find liability. Common Michigan civil cause-of-action types and their typical elements:

- **Breach of Fiduciary Duty** — typically four elements:
  1. **Duty:** A fiduciary relationship existed between the parties (e.g., personal representative ↔ heir, attorney ↔ client, conservator ↔ ward)
  2. **Breach:** The fiduciary breached the duty through specific actions or omissions (self-dealing, conflict of interest, treating heirs differently, failure to disclose, exploitation)
  3. **Causation:** The breach proximately caused the plaintiff's injury
  4. **Damages:** The plaintiff suffered quantifiable harm

- **Fraud** — Michigan elements per M Civ JI 128.01:
  1. **Misrepresentation or failure to disclose:** Defendant made a false representation, or concealed/failed to disclose a material fact
  2. **Knowledge or reckless disregard:** Defendant knew the representation was false (or made it recklessly without knowledge)
  3. **Intent / pattern:** Defendant intended the plaintiff (or court) to rely; OR there was a pattern of deceptive conduct
  4. **Causation:** Plaintiff was damaged as a direct and proximate result
  5. **Damages:** Plaintiff suffered quantifiable harm

  Note: Michigan complaints sometimes plead fraud loosely, blending fraud-on-the-court (false statements to the court) with fraudulent concealment (failure to disclose). Both fall under fraud's umbrella when pled.

- **Declaratory Relief** (challenging an entity's authority) — typically:
  1. **Authorized scope:** What the entity was authorized to do (per articles of incorporation, statute, court rule)
  2. **Conduct outside scope:** What the entity actually did, beyond its authorized scope
  3. **Liability for fees / failure to supervise:** When the entity charged improperly, or a parent organization failed to supervise it
  4. **Damages:** Plaintiff was required to pay improperly charged fees

  Note: Declaratory relief Counts are structurally different from tort Counts. They mix procedural prerequisites (corporate purpose, statutory limits) with substantive showings.

- **Abuse of Process** — Michigan common-law per *Friedman v Dozorc*:
  1. **Pattern of improper acts:** Defendants engaged in a series of wrongful acts in legal proceedings
  2. **Ulterior purpose:** Defendants' purpose was harassment, embarrassment, or extortion — NOT the legitimate purpose of the proceeding
  3. **Specific lies / false statements:** Defendants made specific false statements to the court (the act-in-the-use-of-process element)
  4. **Damages:** Plaintiff was harmed

- **Conversion / Statutory Conversion (MCL 600.2919a)** — for statutory conversion:
  1. **Property of plaintiff:** Plaintiff owned or had interest in the property
  2. **Wrongful possession:** Defendant took or held the property without authorization
  3. **Knowledge:** Defendant knew the property was wrongfully obtained (statutory conversion specifically)
  4. **Damages:** Trebleable damages under the statute

- **Negligence** — typically four elements:
  1. **Duty of care**
  2. **Breach of duty**
  3. **Causation**
  4. **Damages**

- **Civil Conspiracy** — typically:
  1. **Agreement** between two or more persons
  2. **To accomplish an unlawful purpose** OR a lawful purpose by unlawful means
  3. **Overt act** in furtherance
  4. **Damages**

**How to determine PROVES_ELEMENT:**

This is the deepest reasoning task in Pass 2. Follow these steps for EACH Allegation in the entity list:

**Step A: Read the Allegation's `summary` and `verbatim_quote`.**
What factual claim is this Allegation making? What action, omission, or pattern does it describe?

**Step B: For EACH Element in the entity list, ask:**
"If the factual claim in this Allegation were proven true at trial, would it help establish this specific Element?"

The Element's verbatim_quote describes what must be proven (the drafter's pleading of the duty, breach, etc.). Read it carefully. The Allegation's claim either contributes evidence to proving that, or it does not.

**Step C: If YES, create PROVES_ELEMENT.** If NO, do not.

A well-pled Allegation typically proves 1–3 Elements, often spanning multiple Counts. It's normal for a single Allegation to:
- Prove the breach element of one Count AND the damages element of another Count
- Prove only one Element (e.g., a paragraph identifying when an event occurred may only establish causation for one specific Count)
- Prove zero Elements (a procedural or contextual paragraph that adds context but doesn't prove any specific element)

**Worked examples of PROVES_ELEMENT reasoning:**

**Example 1.** Allegation summary: "Defendant withdrew $50,000 from the trust account without court authorization"
- Element "duty" (breach of fiduciary) — does this prove a duty existed? NO (the relationship is a precondition; the withdrawal doesn't establish the relationship existed)
- Element "breach" (breach of fiduciary) — does this prove a breach? YES (unauthorized withdrawal is a textbook breach)
- Element "damages" (breach of fiduciary) — does this prove damages? YES ($50,000 IS the damage)
- Element "misrepresentation" (fraud) — does this prove a misrepresentation? NO (no statement was made; this is a financial action)
- Element "ulterior purpose" (abuse of process) — does this prove abuse of process? NO (this is a financial action, not misuse of court process)
→ Create PROVES_ELEMENT to "breach" (breach of fiduciary) and "damages" (breach of fiduciary).

**Example 2.** Allegation summary: "Defendants made false statements to the court about plaintiff's conduct"
- Element "duty" (breach of fiduciary, if defendants owed a duty of candor) — POSSIBLY YES
- Element "breach" (breach of fiduciary) — YES if defendants owed a duty of candor and breached it
- Element "misrepresentation" (fraud) — YES, false statements ARE misrepresentations
- Element "pattern" (fraud) — YES if there's a pattern of false statements
- Element "specific lies" (abuse of process) — YES, false statements to the court ARE specific lies
- Element "ulterior purpose" (abuse of process) — POSSIBLY, depends on whether the purpose was harassment vs legitimate advocacy
→ Create PROVES_ELEMENT to multiple Elements across multiple Counts.

**Example 3.** Allegation summary: "Defendant was not authorized under its corporate charter to serve as personal representative"
- Element "authorized scope" (declaratory relief) — YES, this directly proves the corporate-scope element
- Element "conduct outside scope" (declaratory relief) — YES, acting as PR when not authorized IS the conduct outside scope
- Element "duty" (breach of fiduciary) — POSSIBLY (acting without authority is itself a breach of duty)
- Element "misrepresentation" (fraud) — YES if the lack of authority was concealed
→ Create PROVES_ELEMENT to multiple Elements across multiple Counts.

**Example 4.** Allegation summary: "Plaintiff hereby incorporates paragraphs 1 through 71 as though fully reinstated herein"
- This is a procedural incorporation paragraph. It contains no factual content of its own.
→ Create ZERO PROVES_ELEMENT relationships. The substantive paragraphs (¶1-71) prove what they prove on their own; the incorporation paragraph adds nothing.

**CRITICAL — Incorporation by reference is NOT automatic PROVES_ELEMENT:**

Complaints use a legal convention where each Count says "Plaintiff hereby incorporates paragraphs 1 through X." This means the FACTS from those paragraphs are AVAILABLE as context for the Count. It does NOT mean every fact in that range automatically proves every Element of the Count.

Incorporation makes the facts part of the Count's record. PROVES_ELEMENT means the fact actually helps prove a specific Element of that Count's legal theory. You must still evaluate each Allegation against each Element on its merits.

**Think of it this way:** Incorporation is like putting all the evidence on the table. PROVES_ELEMENT is deciding which pieces of evidence actually prove your point on a specific element.

**Common mistakes to avoid:**
- Creating PROVES_ELEMENT for every Allegation in the incorporation range — WRONG. Evaluate legal relevance per Element.
- Only creating PROVES_ELEMENT when the Allegation explicitly mentions the Element by name — TOO NARROW. An Allegation about unauthorized financial transactions proves a fiduciary duty's breach element even without using the words "fiduciary" or "breach."
- Missing PROVES_ELEMENT when the Allegation relates to an Element but uses different terminology — READ the substance, not the labels. "Defendants treated heirs differently" proves the breach element of fiduciary duty even though it doesn't use the word "breach."
- Creating PROVES_ELEMENT from count_section paragraphs to elements of OTHER Counts — be careful. A count_section paragraph is part of its own Count's structural completeness; it may also prove elements of other Counts if the underlying facts support multiple legal theories. Evaluate each case individually.

### PART_OF (Allegation → ThematicAllegation)

**What it means:** This Allegation is one of the paragraphs grouped into this navigational theme.

**How to determine:** This is mechanical. For each ThematicAllegation, parse its `paragraph_numbers` property (comma-separated). For each paragraph number listed:
1. Find the Allegation with that `paragraph_number` AND `kind=common_allegation`
2. Create one PART_OF relationship from the Allegation to the ThematicAllegation

**Output rule:** Every common_allegation Allegation should have at least one PART_OF relationship (because every common_allegation should belong to at least one theme).

### ABOUT (Allegation → Party)

**What it means:** This Allegation discusses, implicates, or concerns this party. It tells the system "this factual claim is relevant to this person or organization."

**How to determine:**
- Read the Allegation's `summary` and `verbatim_quote`
- Identify every party mentioned by name, role reference ("Defendant X," "Plaintiff"), or clear pronoun
- Create ABOUT for each mentioned party
- An Allegation can be ABOUT multiple parties

**Rules:**
- "Defendants" (plural) → create ABOUT for EACH defendant party
- If the Allegation describes harm to the plaintiff → the Allegation is ABOUT the defendant who acted AND the plaintiff who was affected
- If a third-party individual is named → create ABOUT for that party too
- Only link to parties actually discussed — do not link to parties not mentioned

### CAUSED_BY (Harm → Allegation)

**What it means:** This misconduct directly caused this harm. The Allegation describes the actions that led to the damage.

**How to determine:**
- Read the Harm's `description` — what damage occurred?
- Find the Allegation(s) whose factual claims describe the actions that caused that damage
- The connection should be causal: the defendant's action (Allegation) LED TO the plaintiff's loss (harm)
- A Harm may have multiple causes (multiple CAUSED_BY relationships)

### DAMAGES_FOR (Harm → LegalCount)

**What it means:** This harm provides evidence of damages for this legal Count. When the plaintiff asks the court for money under this Count, this harm is part of the damages argument.

**How to determine:**
- Read the Harm's description and the LegalCount's legal_theory
- Does this harm result from the type of misconduct covered by this Count?
- Financial harms from fiduciary breach → DAMAGES_FOR the breach-of-fiduciary-duty count
- Financial harms from fraudulent conduct → DAMAGES_FOR the fraud count
- Harms from unauthorized corporate fees → DAMAGES_FOR the declaratory relief count
- Harms from process abuse → DAMAGES_FOR the abuse-of-process count
- A harm can support damages for multiple counts if the underlying misconduct spans multiple theories

### SUFFERED_BY (Harm → Party)

**What it means:** This party suffered this harm.

**How to determine:** Identify who was damaged. In most civil complaints, the plaintiff suffered all harms. Create SUFFERED_BY for each Harm to the plaintiff Party.

### EVIDENCED_BY (Harm → Allegation)

**What it means:** This Allegation provides evidence of this harm — it describes, demonstrates, or quantifies the damage, even if it's not the direct cause.

**How to determine:** Broader than CAUSED_BY. An Allegation may evidence a harm by providing context, describing the pattern of misconduct, or establishing the circumstances that led to the damage.

## Your Reasoning Process — Follow These Steps in Order

### Step 1: Reconstruct mechanical relationships first
These don't require judgment — they're reconstructions from properties:
1. For each Element: create HAS_ELEMENT from `parent_count_id` to the Element
2. For each Element: parse `anchor_paragraph_numbers` and create ANCHORED_IN to each Allegation
3. For each ThematicAllegation: parse `paragraph_numbers` and create PART_OF from each Allegation to the ThematicAllegation

### Step 2: Map the Element framework
For each LegalCount in the entity list:
1. Read the `count_name` and `legal_theory`
2. Identify the Elements that belong to this Count (HAS_ELEMENT relationships from Step 1)
3. Read each Element's verbatim_quote — this is what must be proven for that Count
4. Note this framework mentally for Step 3

### Step 3: Create PROVES_ELEMENT relationships (the deep reasoning step)
For each Allegation in the entity list:
1. Read its `summary` and `verbatim_quote`
2. For EACH Element across all Counts, ask: "If this Allegation's factual claim were proven true, would it help establish this specific Element?"
3. If YES → create PROVES_ELEMENT from the Allegation to that Element
4. If NO → do not create

An Allegation will typically prove some Elements but not all. Procedural paragraphs (incorporation, headings) typically prove zero Elements. Substantive paragraphs typically prove 1–3 Elements across one or more Counts.

### Step 4: Create ABOUT relationships
For each Allegation:
1. Read the `summary` and `verbatim_quote`
2. Identify every Party mentioned or implicated (by name, by role reference, by pronoun)
3. Create ABOUT for each

### Step 5: Create Harm relationships
For each Harm:
1. Identify the Allegation(s) describing the causing misconduct → CAUSED_BY
2. Identify the Count(s) this harm supports damages for → DAMAGES_FOR
3. Link to the party who suffered it (typically the plaintiff) → SUFFERED_BY
4. Link to additional Allegations that evidence (but don't directly cause) the harm → EVIDENCED_BY

## Entities from Pass 1

{{entities_json}}

## Schema — Relationship Types and Constraints

{{schema_json}}

## Extraction Rules

{{global_rules}}

## Document Text

{{document_text}}

## Output Format

Return a JSON object with a single top-level key "relationships":

```json
{
  "relationships": [
    {
      "relationship_type": "HAS_ELEMENT",
      "from_entity": "count-001",
      "to_entity": "element-001"
    },
    {
      "relationship_type": "ANCHORED_IN",
      "from_entity": "element-001",
      "to_entity": "allegation-022"
    },
    {
      "relationship_type": "PROVES_ELEMENT",
      "from_entity": "allegation-008",
      "to_entity": "element-002"
    },
    {
      "relationship_type": "PART_OF",
      "from_entity": "allegation-008",
      "to_entity": "theme-001"
    },
    {
      "relationship_type": "ABOUT",
      "from_entity": "allegation-008",
      "to_entity": "party-002"
    },
    {
      "relationship_type": "CAUSED_BY",
      "from_entity": "harm-001",
      "to_entity": "allegation-008"
    },
    {
      "relationship_type": "DAMAGES_FOR",
      "from_entity": "harm-001",
      "to_entity": "count-001"
    },
    {
      "relationship_type": "SUFFERED_BY",
      "from_entity": "harm-001",
      "to_entity": "party-001"
    }
  ]
}
```

## Completeness Checklist — Verify Before Returning

### Mechanical relationships (Steps 1-2)
- [ ] Does every Element have exactly one HAS_ELEMENT relationship from its parent_count_id?
- [ ] Does every Element have at least one ANCHORED_IN relationship (one per paragraph in anchor_paragraph_numbers)?
- [ ] Does every common_allegation Allegation have at least one PART_OF relationship?

### PROVES_ELEMENT (Step 3) — the central reasoning step
- [ ] For EACH LegalCount, did I identify its Elements via HAS_ELEMENT?
- [ ] For EACH Allegation, did I evaluate it against EACH Element across all Counts?
- [ ] Did I create PROVES_ELEMENT based on whether the Allegation's facts help prove the Element's pleading text?
- [ ] Did I avoid creating PROVES_ELEMENT purely based on incorporation paragraph ranges?
- [ ] Did procedural paragraphs (incorporation, headings) correctly produce zero PROVES_ELEMENT?
- [ ] Does every Element have at least one PROVES_ELEMENT incoming (otherwise it's an unproven element — a gap)?

### ABOUT (Step 4)
- [ ] Does every Allegation have at least one ABOUT relationship?
- [ ] Did I check for plural references ("Defendants" → ABOUT each defendant)?
- [ ] Did I include the plaintiff as ABOUT when the Allegation describes harm to them?

### Harm relationships (Step 5)
- [ ] Does every Harm have at least one CAUSED_BY relationship?
- [ ] Does every Harm have at least one DAMAGES_FOR relationship?
- [ ] Does every Harm have a SUFFERED_BY relationship?

### General verification
- [ ] Did I use ONLY entity IDs from the Pass 1 entity list?
- [ ] Did I NOT create any new entities?
- [ ] Did I NOT include entity objects — only relationships?

Return ONLY the JSON object with a "relationships" array. No markdown fences, no explanation, no preamble.
