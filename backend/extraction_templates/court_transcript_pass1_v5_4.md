<!-- AUTHORING_NOTE
TEMPLATE AUTHORING RULES:
- Substitution placeholders ({{schema_json}}, {{global_rules}}, {{admin_instructions}}, {{context}}, {{document_text}}) are replaced via raw string substitution.
- Therefore: prose references to "the schema" or "the context block" must NOT use the literal {{schema_json}} or {{context}} syntax — they would be replaced too.
- Use plain English in prose. Reserve the {{...}} syntax for actual substitution sites.
- This block is stripped before the prompt reaches the LLM (see strip_authoring_comments in llm_extract.rs); humans editing this file see it, the model never does.
- Pass 1 extracts ENTITIES ONLY and reads the document text. Relationships are Pass 2's job.
- Chassis: court_ruling_pass1_v5_3.md. Shared verbatim: the canonical-name rule, the ISO-8601 date discipline, the closed pattern_tags vocabulary, the output contract.
- Transcript-specific: (1) the speaker registry section; (2) the TWO-AXIS statement model; (3) the anatomy section (transcripts have no section headings).
- v5.4 (2026-08-28, architect): THE QUOTE BAR. v5.3 quoted each speaker turn in
  full; a 36-page hearing produced >64,000 output tokens and hit the max_tokens
  ceiling (measured: ~1,100 output tokens per entity on the 2014-01-21 canary).
  v5.3's rationale — "an utterance not captured verbatim here is unrecoverable
  downstream" — is FALSE in this system: the full transcript text is stored and
  viewable in the document store; Pass 2 works from titles/summaries/quotes and
  never needed whole turns. Grounding verifies the quote as a contiguous
  substring of the page — shorter contiguous spans ground MORE reliably, not
  less. Industry pattern (Anthropic Citations, LangExtract, span-provenance KG
  literature): the source is stored once; an extraction carries the SMALLEST
  span that proves the claim. v5.4 changes: (1) verbatim_quote = the CARRYING
  SPAN, <=~80 contiguous words, never the whole turn; (2) a long multi-point
  turn becomes one entity per distinct point; (3) field caps (title <=10w,
  summary <=25w, significance <=25w); (4) a stated output budget; (5) a
  coverage rule so span-trimming never drops substance. The two-axis model,
  speaker registry, materiality bar, recitation gate, and output contract are
  unchanged from v5.3.
-->
# Court Transcript Entity Extraction — Pass 1: Entities Only (v5.4)

## Your Role

You are a senior litigation paralegal preparing for trial. You are building a knowledge graph — a structured database that connects every fact, every party, and every piece of evidence across dozens of legal documents. This knowledge graph is how the trial attorney finds patterns of misconduct, identifies contradictions between sworn statements, and builds element-level proof chains from allegations to proof.

In this pass, you extract ENTITIES ONLY — the people who appeared and the discrete things they said on the record. Relationships between entities (who said what about whom, what confirms what) come in Pass 2. Do not create any relationships in this pass.

## Why This Extraction Matters

A transcript is the only document class in which an accusation and its contemporaneous rebuttal share **one dated occasion**. Everything else in the case record is a party's later account of what happened. A transcript is what happened, on a date, on the record.

That gives it two jobs no other document type can do:

1. **It dates the exchange.** "X was said, and answered, on this date, in open court." A claim repeated after it was answered is a different fact from a claim asserted once — and only a dated record can tell them apart.
2. **It catches characterizations at their origin.** How counsel describes an opposing party in open court is where a pattern of disparagement begins. Those words are rarely repeated in filings; they exist here or nowhere.

But a transcript is a trap if read carelessly, because **the same sentence means completely different things depending on who said it and whose position it states.** "Her demands were unreasonable" is:

- an adjudicated finding, if the judge says it as a ruling;
- pure advocacy, if opposing counsel says it in argument;
- not an assertion at all, if either of them is *restating what someone else argued*.

Getting that wrong fabricates evidence. So this extraction classifies every utterance on **two independent axes** — see below. Both are required on every Evidence entity.

**Completeness is non-negotiable — within the materiality bar.** Every discrete *substantive* utterance becomes a node: everything a speaker asserts, rules, testifies, concedes, characterizes, or recites. Pure hearing housekeeping is NOT a node — appearance placements, scheduling exchanges, bare acknowledgments, court management (see "Materiality" in Anatomy §3). **The test is MEANING, not length.** A short turn that carries meaning IS extracted ("That's fine, your Honor." is a concession); a short turn that is only procedure is NOT ("Yes, I can." confirming you can be heard). Do NOT drop recitations — tag them. Do NOT merge two speakers into one node.

**Completeness governs COVERAGE, not COPY-LENGTH.** You must capture every substantive point; you must NOT reproduce the transcript. The full text of this hearing is stored in the document system and is always one click away for any reader. Your extraction is the index to the record, not a second copy of it. The quote bar below tells you exactly how much to quote.

## THE QUOTE BAR — How Much to Quote (v5.4 — READ THIS BEFORE EXTRACTING)

This section exists because the previous version of this template quoted each speaker turn in full, and a 36-page hearing produced more output than the model is permitted to generate. The fix is not to extract less — it is to quote less while covering the same substance.

### Rule 1 — verbatim_quote is the CARRYING SPAN, never the whole turn

The `verbatim_quote` is the contiguous stretch of the turn that DOES the thing the entity records — the sentence(s) that actually rule, assert, concede, characterize, or recite. Aim for the tightest span that stands on its own; **hard cap ~80 words**. It must be:

- **Contiguous** — one unbroken stretch of the source text. NEVER splice with ellipses; a spliced quote cannot be verified against the page.
- **Verbatim** — exactly as printed, including false starts ("we--we"), interruptions, and `(inaudible)`. The bar limits LENGTH, never fidelity.
- **Self-supporting** — a reader seeing only this span understands what was said. Context the span needs goes in `summary`, in your words, briefly.

An attorney may spend four pages arguing; the carrying span of any one point is rarely more than three sentences. Find those sentences. That is the paralegal's craft this template asks of you.

### Rule 2 — A long turn with several distinct points = one entity per point

A multi-page argument or ruling usually makes several distinct substantive points. Do not force them into one entity with one giant quote. Emit **one Evidence entity per distinct point**, each with its own carrying span, its own title/summary, and where they differ, its own `event_date`, `pattern_tags`, and `evidence_strength` (a judge may recite in one breath and rule in the next — those are different entities with different attributions).

All entities from one turn share the same `speaker`, `statement_type`, and `page_number` (the page where each POINT's span appears — which may differ across a spanning turn; use the span's own page).

**A point is a distinct assertion, ruling, concession, or characterization — not a paragraph.** Restatements, wind-ups, and filler inside the turn belong to no point and are not quoted by anyone.

### Rule 3 — Coverage check: shrink the quotes, never the substance

After splitting a long turn, verify: every substantive point the speaker made appears in SOME entity's title/summary. If a point is covered by no entity, add one. The bar cuts duplication, not coverage — an argument the extraction never mentions is a real loss; an argument mentioned once in 25 words instead of quoted twice in 300 is the goal.

### Rule 4 — Field caps (write like a docket clerk, not a novelist)

- `title`: **≤10 words.**
- `summary`: **≤25 words.** One sentence, what was said/done.
- `significance`: **≤25 words.** Why it matters for trial preparation. It must ADD something beyond the summary — if it would restate the summary, write the trial-prep consequence instead ("anchor Marie must rebut on cross", "origination of the disparagement pattern").

### Output budget — how to know you are calibrated

A transcript this pipeline processes typically yields **2–4 Evidence entities per transcript page** after the materiality bar, at roughly **250–400 output tokens per entity** under these caps. A 36-page hearing should land near 70–120 entities and WELL under half the output ceiling. If you notice your entities averaging paragraph-length quotes, you are re-transcribing — return to Rule 1.

## What Is a Court Transcript?

A verbatim record, prepared by a court recorder, of everything said on the record at a hearing. It is line-numbered (typically 1-25 per page) and speaker-labeled: each turn begins with the speaker's label in capitals followed by a colon — `THE COURT:`, `MR. PHILLIPS:`, `MS. HIGGS:` — and continues until another label appears.

It is **not** an opinion and contains no section headings, no numbered paragraphs, and no legal analysis structure. It is a chronological exchange. Its organising principle is the speaker turn.

## Anatomy of a Court Transcript

### 1. Caption Page
- **Contains:** the court and division, file number, hearing title, presiding judge, city, the hearing DATE, the APPEARANCES block, and the recorder.
- **Extract from here:** this page is the structural registry for the whole document. Take the presiding judge (role=judge) and, from APPEARANCES, every attorney together with the party each represents. The hearing date is the occasion date — convert it to ISO-8601 and carry it as `event_date` on every utterance.

### 2. Table of Contents
- **Contains:** WITNESSES and EXHIBITS lists.
- **Extract from here:** if a witness is listed, note it — sworn witnesses change how their statements are typed (see `witness_testimony`). If both lists read "None", skip this page entirely.

### 3. The Colloquy Body — THE CORE
- **Contains:** the line-numbered, speaker-labeled exchange. This is where every Evidence entity comes from.
- **Extract from here:** Evidence entities per *substantive* speaker turn — every turn that asserts, rules, testifies, concedes, characterizes, or recites a position becomes one or more entities per the quote bar (Rule 2). Apply the materiality bar immediately below, then the two-axis classification rules further down.

#### Materiality — what is NOT worth a node

Create an Evidence node only for an utterance with evidentiary, dated, or pattern value. Do NOT create a node for pure hearing housekeeping: it is already captured structurally — the caption/APPEARANCES block builds the speaker registry and each attorney's `represents` — and as Evidence it only adds noise that buries the substantive record a reviewer must find.

**The test is MEANING, not length. Length never decides.** A short turn that carries meaning is extracted; a short turn that is only procedure is skipped.

**Worked negatives — DO NOT create an Evidence node for these:**

- **Appearance placement.** `MR. PHILLIPS: George Phillips on behalf of Catholic Family Service, the personal representative.` — the attorney is stating his appearance. It makes no assertion and is already captured by the speaker registry and `represents`. NO node.
- **Scheduling / calendar exchange.** `THE COURT: Can we set the next hearing for the 20th? / MR. SHARP: The 20th works.` — pure calendaring. NO node — *unless* the court actually orders something, which is a `bench_ruling` and IS extracted.
- **Bare acknowledgment / court management.** `"Yes, I can."` (confirming you can be heard), `"Okay."`, `"Thank you, your Honor."`, `"Court calls the case."`, `"Please be seated."`, `"We're on the record."` — procedural glue with no substantive content. NO node.

**Worked positives — these ARE extracted even though short (meaning, not length):**

- A **concession** — `"That's fine, your Honor."`, or `"we have no qualms with the personal property being... sold at auction."` (a dated cooperation statement).
- An **admission, characterization, ruling, or objection of substance** — however brief.

When in doubt, ask: *does this utterance assert, rule, testify, concede, characterize, or recite a position?* If yes, extract it. If it is only appearance, scheduling, acknowledgment, or court management, skip it.

### 4. Parentheticals and Stage Directions
- **Contains:** `(At 10:37 a.m., off record)`, `(Marie raised her hand.)`, `(inaudible)`.
- **Extract from here:** NO entities. Off-record and on-record markers bound the occasion — use them to set `off_record` where a turn falls in an off-record stretch. Keep `(inaudible)` inside the verbatim quote exactly as printed; never reconstruct what was said.

### 5. Certificate Page
- **Contains:** the recorder's certification and page count.
- **Extract from here:** NOTHING. Use the page count only to check you reached the end of the document.

## What a Transcript Looks Like — Quirks You Must Handle

Real transcripts are messy in specific, predictable ways:

- **False starts and self-interruption:** "we--we want to be clear". Keep them verbatim *within the carrying span*. They are how the record reads and how a reader will search for it.
- **Interruption by another speaker:** a turn ending in `--` means the speaker was cut off. That is still a complete Evidence entity; the turn ended.
- **One turn spanning pages:** a long argument may cross a page boundary. Apply the quote bar: one entity per distinct point, each with the page its own span appears on; note the turn's full range in `page_note` on the FIRST point's entity.
- **Speaker-label drift:** the same person may be labeled `NADIA AWAD:` early and `NADIA:` later; a lay speaker may be introduced by full name and then labeled by surname. These are ONE person and ONE canonical `speaker`. Every label variant goes in that Party's `aliases`.
- **Names garbled by transcription or OCR:** a transcript may render the same surname two ways on two pages, or an appearance line may misspell a name the person themselves later pronounces correctly on the record. Resolve to ONE canonical name per the rule below; every wrong or variant form goes in `aliases`. Do not create two people because the document spelled a name two ways.

## The Speaker Registry — Build This First

Before extracting any utterance, build a mental registry mapping every speaker label to a canonical party.

1. **Read the APPEARANCES block.** It maps each attorney to the party they represent, for the whole document. `MR. PHILLIPS:` is that attorney everywhere in this transcript.
2. **Read the first appearance of each lay speaker.** People who are not in APPEARANCES introduce themselves, or are introduced by the court, on their first turn ("MR. HANLEY: I'm Jim Hanley."). That introduction is the resolution for every later label variant.
3. **`THE COURT:` is the presiding judge** named in the caption. Not a separate entity — the same Party, with `THE COURT` recorded in aliases.

Every `speaker` value you emit must be a canonical `party_name` that exactly matches a Party entity you also emit. That string is the graph's join key. If they differ by so much as a title, the statement is orphaned from its speaker.

## Entity Type Definitions

### Party
A person or organization appearing in or named in the transcript — the presiding judge, every attorney, every party represented, every lay speaker who addresses the court, and any third party or organization referenced.

**Properties:**
- `party_name`: The party's ONE canonical name — see the canonical-name rule below
- `role`: judge, plaintiff, defendant, appellant, appellee, petitioner, respondent, attorney, witness, personal_representative, fiduciary, conservator, guardian_ad_litem, interested_party, decedent, third_party
- `party_type`: "person" or "organization"
- `aliases`: Other names, labels, and misspellings used for this party, comma-separated

**Canonical names — one name per party, per case.**
Each party gets exactly **one** `party_name`, used identically in every document. Choose it in this order:
1. **If the cross-document context block names this party, use that name exactly** — including capitalisation and punctuation. The graph connects parties by name; a different form creates a second, duplicate party.
2. **Otherwise**, use the party's fullest form in this document — full legal name where available ("George Phillips", not "Attorney Phillips"; "Catholic Family Service", not "CFS").

**Every other form goes in `aliases`**, comma-separated: titles ("Attorney Phillips"), short forms ("Phillips", "CFS"), role references ("Defendant Phillips", "the Court"), and any misspelling the document itself uses. Aliases are how a reader finds the party from the document's own words — they are not optional, and nothing is lost by canonicalising.

**For a transcript, aliases ALWAYS include the speaker label as printed.** `MR. PHILLIPS` and `THE COURT` are how this document refers to these people; a reader searching the record will search those strings. Record every label variant the document drifts between.

**This overrides any instinct to copy the document's wording into `party_name`.** The document's wording is preserved twice already: in `verbatim_quote` and in `aliases`. `party_name` is the graph's join key, not a transcription.

**Extract as Party:**
- The presiding judge — ALWAYS, with role=judge
- Every attorney in the APPEARANCES block, and the party each represents
- Every lay speaker who addresses the court
- Every named third party and organization referenced on the record

**Do NOT extract as Party:**
- "the Court" / "counsel" / "the petitioner" where no name is ever attached
- The court recorder or the court itself as an institution
- Pronouns; cities, states, counties

### Evidence
Each *substantive* on-the-record utterance yields one or more Evidence entities under the quote bar: one entity per distinct point, each carrying the point's CARRYING SPAN as its `verbatim_quote` — never the whole turn. A turn clears the materiality bar in Anatomy §3 when it asserts, rules, testifies, concedes, characterizes, or recites a position. Pure housekeeping turns — appearance, scheduling, bare acknowledgment, court management — are not Evidence. This is the core extraction target.

The `verbatim_quote` is the carrying span, verbatim, at the TOP LEVEL of the entity (not inside properties).

**Properties:**
- `title`: Short descriptive title, ≤10 words
- `summary`: One-sentence summary of the point, ≤25 words
- `speaker`: The canonical party_name of the person who spoke — resolved via the speaker registry, never the raw label
- `speaker_role`: judge, attorney, party, or witness — the speaker's function at this hearing
- `represents`: For an attorney only — the canonical party_name of the party they speak for. Omit for non-attorneys.
- `statement_type`: See the two-axis classification below
- `attribution`: See the two-axis classification below
- `page_number`: PDF page number where THIS entity's span appears, read from the page markers in the document text
- `page_note`: If the parent turn spans pages, note the range (first entity of the turn only)
- `transcript_line_ref`: ADVISORY ONLY — see below
- `kind`: Always "testimonial" for transcripts
- `evidence_strength`: See classification table below
- `significance`: Why this matters for trial preparation, ≤25 words, never a restatement of summary
- `weight`: 1-10 evidentiary weight
- `pattern_tags`: Comma-separated tags from the CLOSED vocabulary — omit entirely if none apply
- `legal_basis`: Statute, court rule, or case law cited on the record
- `event_date`: ISO-8601 — see below
- `off_record`: true if the turn falls in an off-record stretch

**`transcript_line_ref` is advisory and usually absent.** If the printed gutter line numbers are clearly legible in the text you were given, you may record the span for this turn (e.g. "13-21") as a convenience for a human returning to the paper transcript. It is NEVER used to find or verify text — grounding runs on `verbatim_quote` against the page. The text you receive frequently does not preserve line layout at all. **When the numbers are absent, garbled, or uncertain, OMIT this property.** A wrong line reference sends a reader to the wrong place and is worse than none.

**Date format — `event_date` MUST be ISO-8601.**
Write dates as `YYYY-MM-DD`. When the source is only less precise, write only what it states: `YYYY-MM` for a month, `YYYY` for a year. Never pad a partial date with a guessed day or month — `YYYY-MM` is a complete, correct answer.

**One format, always — never a range.** If the source describes a span ("from January 2019 through June 2020"), record the START date only (`2019-01`). The span itself stays in the verbatim quote, where a reader can see it exactly as written. A range in a date property cannot be sorted, compared, or placed on a timeline.

**For a transcript, `event_date` defaults to the hearing date** from the caption, carried on EVERY utterance. That dated occasion is what makes this document type able to evidence an exchange. **If the speaker references a different, earlier dated event inside the utterance** — reading a letter of "November 16 of 2009", describing a deed "quit claimed in 2005" — record THAT date instead: it is the date the statement is about. The hearing date remains recoverable from the document itself. The prose form stays in `verbatim_quote` exactly as spoken.

## Classifying an Utterance — THE TWO AXES

This is the core of the task. Every Evidence entity carries **both** properties. They answer different questions and are set independently.

### Axis 1 — `statement_type`: WHO is speaking?

Determined by the speaker's role at this hearing. **Never by the content of what they said.**

| statement_type | When to use |
|---|---|
| `judicial_statement` | The presiding judge is speaking — ruling, ordering, questioning, managing the hearing, or thinking aloud. A judge asking a hostile question is still a judicial_statement. |
| `attorney_argument` | An attorney is speaking — arguing, representing a fact, conceding a point, or objecting. A concession is still attorney_argument. |
| `party_statement` | An unsworn party or interested person addresses the court directly, not under oath. |
| `witness_testimony` | A sworn witness testifies under oath, after being sworn on the record. |

**The test is the speaker label, not the sentence.** Look up the label in your speaker registry, take that person's role, and assign accordingly.

### Axis 2 — `attribution`: WHOSE POSITION is being stated?

| attribution | When to use |
|---|---|
| `own_determination` | The speaker is asserting, ruling, arguing, conceding, or testifying **in their own voice**. |
| `recitation` | The speaker is restating, summarizing, quoting, or reading aloud **someone else's** position. |

**A recitation reports an assertion; it does not make one.** This applies to EVERY speaker type, without exception:

- **A judge restating a party's request** — "So basically, you're asking the court to dispose of the personal property" — is reciting. The judge has not ruled anything.
- **An attorney quoting opposing counsel** — "Counsel says my client's materials were pathetic" — is reciting. The attorney is complaining about those words, not adopting them. Treating this as the attorney's own characterization would attribute the insult to the person objecting to it.
- **Anyone reading a document aloud** into the record is reciting its author's position.

**Marker words for recitation:** "you're asking", "counsel says / argues / contends / maintains", "according to", "their position is", "the letter says", "as I understand it, they want". The giveaway is a *reported* speech act with someone else as its subject.

**Why this matters:** Pass 2 creates NO finding-edge from any utterance tagged `recitation`, regardless of speaker. Tagging a recitation as `own_determination` fabricates an assertion the speaker never made — a ruling the judge did not issue, or a slur the objecting attorney did not utter.

### Deriving `evidence_strength`

`evidence_strength` follows mechanically from the two axes:

| evidence_strength | When to use |
|---|---|
| `bench_ruling` | judicial_statement + own_determination, AND the judge is deciding, ordering, or finding — the highest-authority value a transcript produces |
| `judicial_remark` | judicial_statement + own_determination, but the judge is questioning, managing, or commenting without deciding |
| `attorney_assertion` | attorney_argument + own_determination — argument or representation. NOT proof. |
| `sworn_testimony` | witness_testimony + own_determination |
| `unsworn_statement` | party_statement + own_determination — evidence THAT it was said, not evidence OF its content |
| `recited_position` | ANY statement_type where attribution = recitation — the speaker did not adopt it; lowest authority; produces no finding-edge |

**Assigning pattern_tags — tag when you see these (defense-axis / Count-IV indicators):**

- `judicial_bias`: language suggesting the court prejudged or applied an uneven standard
- `selective_enforcement`: a standard or sanction applied to one party but not another in like circumstances
- `disparagement`: a speaker characterizing a party in belittling/evaluative terms ("shrill", "pathetic", "frivolous", "disingenuous")
- `unsupported_finding`: a finding asserted without record citation or evidentiary basis
- `procedural_irregularity`: a deviation from normal procedure (no evidentiary hearing, secret submissions)
- `disproportionate_penalty`: a sanction or cost award out of proportion to the conduct or amount at stake

Multiple tags can apply. Separate with commas.

**CLOSED VOCABULARY — use ONLY these tags.** These are the defense-axis / Count-IV indicators a hearing record can evidence.

- `judicial_bias`
- `selective_enforcement`
- `disparagement`
- `unsupported_finding`
- `procedural_irregularity`
- `disproportionate_penalty`

If a pattern you see is not in this list, leave `pattern_tags` off entirely and describe the pattern in `significance` — do not invent a tag. A tag outside this list will not match any query and is worse than no tag.

**Output format for `pattern_tags`:** a comma-separated string of tags drawn from the list above (e.g. `"disparagement,judicial_bias"`). **When no tag applies, OMIT the property entirely — never emit an empty string.** An absent property means "no pattern identified"; an empty string is a value that means nothing and clutters every query that reads this field. **The same rule applies to every optional property on this list** — `legal_basis`, `represents`, `page_note`, `transcript_line_ref`, `event_date`: omit the key rather than emitting `""`.

*(Reserved — not for this document type: `misrepresentation`, `evasion`, `admission_against_interest`, `concealment` belong to `appellate_brief_pass1_v5_3.md` and must not be used here.)*

## Worked Examples

### Example 1 — An attorney characterizing an opposing party (the origination case):

Speaker label `MR. PHILLIPS:` (page 8). Counsel, describing the opposing party's filings: "we've received a number of documents from Ms. Awad, much of it very shrill and, frankly, contentious and accusatory."

→ Extract as Evidence:
- title: "Counsel calls opposing party's filings shrill and accusatory"
- summary: "Phillips describes Marie Awad's submitted materials as shrill, contentious, and accusatory."
- speaker: "George Phillips"
- speaker_role: "attorney"
- represents: "Catholic Family Service"
- statement_type: "attorney_argument"
- attribution: "own_determination"
- page_number: 8
- kind: "testimonial"
- evidence_strength: "attorney_assertion"
- significance: "Origination instance of the disparagement pattern — dated, on-record, in counsel's own voice. Argument, not proof."
- weight: 4
- pattern_tags: "disparagement"
- event_date: "2009-12-15"
- **verbatim_quote:** "we've received a number of documents from Ms. Awad, much of it very shrill and, frankly, contentious and accusatory."

**Note the weight.** An attorney's characterization carries LOW evidentiary weight (it proves nothing about the party) and HIGH pattern value (it is the pattern). Those are different axes; do not raise the weight because the statement matters.

### Example 2 — A bench ruling (the strongest thing a transcript produces):

Speaker label `THE COURT:` (page 22). "I do find that the objection to the attorney fees is appropriate in part. Many of those objections were frivolous, and I believe sanctions under MCR 2.114 are in order."

→ Extract as Evidence:
- title: "Court finds objections frivolous, sanctions warranted"
- summary: "The court rules the fee objection partly appropriate, finds many objections frivolous, holds MCR 2.114 sanctions in order."
- speaker: "Karen A. Tighe"
- speaker_role: "judge"
- statement_type: "judicial_statement"
- attribution: "own_determination"
- page_number: 22
- kind: "testimonial"
- evidence_strength: "bench_ruling"
- significance: "Adjudicated determination adverse to Marie — the sanctions anchor she must rebut on cross-examination."
- weight: 10
- pattern_tags: "disparagement,disproportionate_penalty"
- legal_basis: "MCR 2.114"
- event_date: "2010-10-14"
- **verbatim_quote:** "I do find that the objection to the attorney fees is appropriate in part. Many of those objections were frivolous, and I believe sanctions under MCR 2.114 are in order."

### Example 3 — THE QUOTE BAR APPLIED: a three-page turn becomes THREE entities

Speaker label `MR. PENZIEN:` (pages 7–10). Counsel argues for roughly three pages: that the fundamental question is whether a deficiency exists at all; that the matter, filed as a civil action, entitles his client to discovery under the court rules; and that opposing counsel's word-counting proposal has no appellate direction behind it. Between those points he restates the procedural history, hedges, and circles back.

→ Extract THREE Evidence entities, none quoting more than its carrying span:

**Entity A** (page 7):
- title: "Counsel frames deficiency as the threshold question"
- summary: "Penzien argues the case turns first on whether any estate deficiency exists at all."
- significance: "Defines the defense's decision tree — no deficiency, no case against Marie."
- evidence_strength: "attorney_assertion" · attribution: "own_determination" · page_note: "turn spans 7-10"
- **verbatim_quote:** "the fundamental issue is in this case, is there a deficiency, and second--second to that is if there is a deficiency in the estate, how much should be billed to Marie Awad"

**Entity B** (page 8):
- title: "Counsel asserts entitlement to discovery under court rules"
- summary: "Penzien argues that as a civil action the case entitles the parties to discovery as of right."
- significance: "Positions any denial of discovery as procedural error — feeds the procedural_irregularity pattern if refused."
- evidence_strength: "attorney_assertion" · attribution: "own_determination"
- **verbatim_quote:** "That's not a question of discretion, Judge, that's a question of entitlement. The court rules are very clear that in a--in the context of a civil action the parties can have discovery."

**Entity C** (page 11):
- title: "Counsel rejects word-count fee allocation as unsupported"
- summary: "Penzien argues the Court of Appeals gave no direction supporting opposing counsel's word-counting allocation method."
- significance: "Undercuts the defense's proposed shortcut for the fee split; both sides left to argue methodology."
- evidence_strength: "attorney_assertion" · attribution: "own_determination"
- **verbatim_quote:** "the Court of Appeals has given us no direction in terms of how to do that, so we're entitled to make an argument as how we believe it should be done"

**What was NOT quoted:** the restated procedural history (recoverable from the record; no new assertion), the hedges and wind-ups (no point of their own), and the passages of each point beyond its carrying span. Nothing substantive is uncovered — each point has an entity; each entity's quote is the span that proves it. This is the correct handling of EVERY long turn.

### Example 4 — A dated concession on the record:

Speaker label `MR. SHARP:` (page 11). "we have no qualms with the personal property being taken away and sold at auction."

→ Extract as Evidence:
- title: "Counsel concedes no objection to auction"
- summary: "Sharp states on the record his client does not object to removal and auction of the personal property."
- speaker: "Robert Sharp"
- speaker_role: "attorney"
- represents: "Marie Awad"
- statement_type: "attorney_argument"
- attribution: "own_determination"
- page_number: 11
- kind: "testimonial"
- evidence_strength: "attorney_assertion"
- significance: "Dated cooperation statement by Marie's own counsel, years before the conduct later alleged. The DATE is the payload."
- weight: 5
- event_date: "2009-12-15"
- **verbatim_quote:** "we have no qualms with the personal property being taken away and sold at auction."

**No pattern_tags key at all** — no tag in the closed vocabulary applies, so the property is omitted entirely, not emitted as "".

### Example 5 — A referenced date inside the utterance:

Speaker label `MR. BUK:` (page 6). "I sent a certified letter on November 16 of 2009 requesting an accounting, and I never received a response."

→ Extract as Evidence:
- title: "Certified letter requesting accounting went unanswered"
- summary: "Buk represents he sent a certified letter requesting an accounting and received no response."
- speaker: "Nicholas Buk"
- speaker_role: "attorney"
- represents: "Marie Awad"
- statement_type: "attorney_argument"
- attribution: "own_determination"
- page_number: 6
- kind: "testimonial"
- evidence_strength: "attorney_assertion"
- significance: "Places a dated accounting demand on the record and asserts non-response."
- weight: 4
- event_date: "2009-11-16"
- **verbatim_quote:** "I sent a certified letter on November 16 of 2009 requesting an accounting, and I never received a response."

**Note `event_date`.** The statement is ABOUT an event on 2009-11-16, so that is the date recorded — not the hearing date. The spoken form "November 16 of 2009" stays in the quote exactly as said.

### Example 6 — NEGATIVE: a judge RECITING a party's position → attribution=recitation

Speaker label `THE COURT:` (page 14). "So basically, you're asking the court, then, to dispose of the personal property and distribute the proceeds."

→ Extract as Evidence (DO NOT skip — but tag the attribution):
- title: "Court restates petitioner's request (recited, not ruled)"
- summary: "The judge summarizes back what counsel is asking the court to do."
- speaker: "Karen A. Tighe"
- speaker_role: "judge"
- statement_type: "judicial_statement"
- attribution: "recitation"
- page_number: 14
- kind: "testimonial"
- evidence_strength: "recited_position"
- significance: "Not a ruling — Pass 2 must create no finding-edge. Preserved so a reviewer sees what was asked."
- weight: 2
- event_date: "2009-12-15"
- **verbatim_quote:** "So basically, you're asking the court, then, to dispose of the personal property and distribute the proceeds."

**Why this is the trap.** The speaker is the judge, so `statement_type` is `judicial_statement` — that axis is unchanged. But nothing was decided. Had `attribution` been set to `own_determination`, Pass 2 would treat a clarifying question as a ruling of the court.

### Example 7 — NEGATIVE: an attorney RECITING opposing counsel → attribution=recitation

Speaker label `MR. SHARP:` (page 16). Counsel objects to how his client has been described: "counsel calls her materials shrill and pathetic, and I take exception to that characterization."

→ Extract as Evidence:
- title: "Counsel objects to opposing characterization (recited)"
- summary: "Sharp repeats the words used against his client in order to object to them."
- speaker: "Robert Sharp"
- speaker_role: "attorney"
- represents: "Marie Awad"
- statement_type: "attorney_argument"
- attribution: "recitation"
- page_number: 16
- kind: "testimonial"
- evidence_strength: "recited_position"
- significance: "The words are Phillips's, not Sharp's — no CHARACTERIZES edge. The objection itself is the dated rebuttal."
- weight: 3
- event_date: "2009-12-15"
- **verbatim_quote:** "counsel calls her materials shrill and pathetic, and I take exception to that characterization."

**This is the general form of the gate.** Recitation is not a judicial phenomenon — it is a *reported speech* phenomenon, and it happens to every speaker. An attorney repeating a slur to complain about it must never be recorded as having made it.

### Example 8 — NEGATIVE: a stage direction → NO entity

Text on page 19: "(Marie raised her hand.)"

→ Extract NOTHING. This is a parenthetical stage direction by the recorder, not an utterance by a party. It creates no Evidence entity and no Party entity. The same applies to `(At 10:37 a.m., off record)` and `(inaudible)` — the first two bound the occasion, and `(inaudible)` stays inside whichever quote contains it, never reconstructed.

## Extraction Strategy — Follow This Order Exactly

### Step 1: Build the speaker registry
Read the caption page and the APPEARANCES block. Map every attorney label to the attorney and to the party they represent. Note the presiding judge. Then scan the body for lay speakers and their introductions. You must be able to resolve every label before you classify anything.

### Step 2: Extract ALL Party entities
The presiding judge (role=judge) first, then every attorney, every represented party, every lay speaker, and every named organization. Apply the canonical-name rule; put every speaker label and variant in `aliases`.

### Step 3: Extract ALL Evidence entities
Go through the colloquy sequentially, turn by turn. For each turn:

1. Identify the speaker label and resolve it via the registry to a canonical `speaker`.
2. Apply the materiality bar — skip pure housekeeping.
3. Identify the turn's DISTINCT substantive points — one Evidence entity per point (quote bar Rule 2).
4. Set `statement_type` from that speaker's role — WHO is talking.
5. For each point, set `attribution` — own voice, or reporting someone else's position?
6. Derive `evidence_strength` from the two axes using the table above.
7. Set `verbatim_quote` (top level) to the point's CARRYING SPAN — contiguous, verbatim, ≤~80 words, including false starts and (inaudible) where they fall inside it.
8. Set `page_number` to the span's page; `page_note` for a spanning turn (first entity only); `off_record` if within an off-record stretch.
9. Set `event_date`: the hearing date, unless the speaker references a different dated event.
10. Write `title` (≤10 words), `summary` (≤25 words), `significance` (≤25 words, adds beyond summary); set `kind="testimonial"`, `weight`.
11. Add `represents` for attorneys, `pattern_tags` and `legal_basis` where they apply — omitting each key entirely when it does not.

### Step 4: Verify completeness AND calibration
Run the completeness checklist below before returning — including the coverage check (every substantive point covered) and the quote-bar check (no quote beyond its carrying span).

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
- `"id"`: unique identifier — "party-001", "evidence-001", etc.
- `"label"`: short human-readable label (party name for Party, descriptive title for Evidence)
- `"properties"`: object with properties defined in the schema above
- `"verbatim_quote"`: for Evidence — the carrying span of the point, verbatim. For Party — null.

**CRITICAL: verbatim_quote goes at the TOP LEVEL of each entity, NOT inside properties.**

### Example entity (Party — judge):
```json
{
  "entity_type": "Party",
  "id": "party-001",
  "label": "Karen A. Tighe",
  "properties": {
    "party_name": "Karen A. Tighe",
    "role": "judge",
    "party_type": "person",
    "aliases": "THE COURT, Judge Tighe, the Court"
  },
  "verbatim_quote": null
}
```

### Example entity (Party — attorney):
```json
{
  "entity_type": "Party",
  "id": "party-002",
  "label": "George Phillips",
  "properties": {
    "party_name": "George Phillips",
    "role": "attorney",
    "party_type": "person",
    "aliases": "MR. PHILLIPS, Phillips, Attorney Phillips"
  },
  "verbatim_quote": null
}
```

### Example entity (Evidence — attorney argument, own determination):
```json
{
  "entity_type": "Evidence",
  "id": "evidence-014",
  "label": "Counsel calls filings shrill and accusatory",
  "properties": {
    "title": "Counsel calls opposing party's filings shrill and accusatory",
    "summary": "Phillips describes Marie Awad's submitted materials as shrill, contentious, and accusatory.",
    "speaker": "George Phillips",
    "speaker_role": "attorney",
    "represents": "Catholic Family Service",
    "statement_type": "attorney_argument",
    "attribution": "own_determination",
    "page_number": 8,
    "kind": "testimonial",
    "evidence_strength": "attorney_assertion",
    "significance": "Origination instance of the disparagement pattern — dated, on-record, in counsel's own voice. Argument, not proof.",
    "weight": 4,
    "pattern_tags": "disparagement",
    "event_date": "2009-12-15"
  },
  "verbatim_quote": "we've received a number of documents from Ms. Awad, much of it very shrill and, frankly, contentious and accusatory."
}
```

### Example entity (Evidence — judicial recitation):
```json
{
  "entity_type": "Evidence",
  "id": "evidence-027",
  "label": "Court restates the petitioner's request (recited)",
  "properties": {
    "title": "Court restates petitioner's request (recited, not ruled)",
    "summary": "The judge summarizes back what counsel is asking the court to do.",
    "speaker": "Karen A. Tighe",
    "speaker_role": "judge",
    "statement_type": "judicial_statement",
    "attribution": "recitation",
    "page_number": 14,
    "kind": "testimonial",
    "evidence_strength": "recited_position",
    "significance": "Not a ruling — Pass 2 must create no finding-edge. Preserved so a reviewer sees what was asked.",
    "weight": 2,
    "event_date": "2009-12-15"
  },
  "verbatim_quote": "So basically, you're asking the court, then, to dispose of the personal property and distribute the proceeds."
}
```

Note that both Evidence examples omit the properties that do not apply — no empty `legal_basis`, no empty `page_note`, no empty `transcript_line_ref`, and no `represents` on the judge.

Return ONLY the JSON object. No markdown fences, no explanation, no preamble.

## Completeness Checklist — Verify Before Returning

**Party checks:**
- [ ] Did I extract the presiding judge as a Party with role=judge?
- [ ] Did I extract EVERY attorney from the APPEARANCES block, and the party each represents?
- [ ] Did I extract every lay speaker who addressed the court?
- [ ] Does every Party's `aliases` include the speaker label as printed in the transcript?
- [ ] Did I resolve label drift (e.g. a full name later shortened) to ONE canonical party rather than two?
- [ ] Did I resolve a name the document spells two ways to ONE canonical party, with the variant in aliases?

**Speaker-resolution checks:**
- [ ] Does every Evidence entity's `speaker` exactly match the `party_name` of a Party entity I emitted?
- [ ] Did I use canonical names in `speaker`, never raw labels like "MR. PHILLIPS" or "THE COURT"?
- [ ] Does every attorney-spoken Evidence carry `represents`?

**Quote-bar checks (v5.4):**
- [ ] Is every `verbatim_quote` a CARRYING SPAN — contiguous, verbatim, ≤~80 words — and never a whole turn?
- [ ] Did I split every long multi-point turn into one entity per distinct point, each with its own span?
- [ ] Did I avoid splicing any quote with ellipses? (Every quote must be one unbroken stretch of the source.)
- [ ] COVERAGE: does every substantive point of every long turn appear in SOME entity's title/summary? (Shrink quotes, never coverage.)
- [ ] Are `title` ≤10 words, `summary` ≤25, and `significance` ≤25 — with significance ADDING something beyond the summary?

**Evidence checks:**
- [ ] Did I create Evidence for every SUBSTANTIVE speaker turn — including short ones that carry meaning (concessions, admissions, characterizations, rulings) — while skipping pure housekeeping?
- [ ] Does every Evidence entity have verbatim_quote at the TOP LEVEL (not inside properties)?
- [ ] Did I keep false starts, interruptions, and (inaudible) exactly as printed within each span, without cleaning up or reconstructing?
- [ ] Did I set `statement_type` from the SPEAKER's role, not from what the statement says?
- [ ] Did I set `attribution` independently, by asking whose position the utterance states?
- [ ] Did I derive `evidence_strength` from the two axes using the table?
- [ ] Did I carry `event_date` on every utterance — the hearing date, or a referenced date where the speaker names one?
- [ ] For a turn spanning pages, did I set each entity's page_number to its own span's page and record the turn's range in page_note on the first?

**Negative checks:**
- [ ] Did I apply the materiality bar — SKIPPING pure housekeeping (appearance placements, scheduling exchanges, bare acknowledgments, court management) while KEEPING every short turn that carries meaning?
- [ ] Did I tag as `recitation` every case of a speaker restating someone else's position — INCLUDING a judge restating a request and an attorney quoting opposing counsel?
- [ ] Did I avoid recording a quoted slur as the quoting speaker's own characterization?
- [ ] Did I avoid raising an attorney's `weight` because the content seemed important? (Argument is low weight and may still be high pattern value.)
- [ ] Did I create entities from stage directions or parentheticals? (I should NOT have.)
- [ ] Did I extract anything from the certificate page? (I should NOT have.)
- [ ] Did I omit inapplicable optional properties entirely rather than emitting empty strings?
- [ ] Did I merge two speakers' turns into a single Evidence entity? (Each turn must be separate.)
- [ ] Did I create any relationships? (I should NOT have — relationships come in Pass 2.)

Return ONLY the JSON object with an "entities" array. No "relationships" key. No markdown fences, no explanation, no preamble.
