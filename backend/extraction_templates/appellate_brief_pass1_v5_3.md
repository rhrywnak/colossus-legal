<!-- AUTHORING_NOTE
TEMPLATE AUTHORING RULES:
- Substitution placeholders ({{schema_json}}, {{global_rules}}, {{admin_instructions}}, {{context}}, {{document_text}}) are replaced via raw string substitution.
- Therefore: prose references to "the schema" or "the context block" must NOT use the literal {{schema_json}} or {{context}} syntax — they would be replaced too.
- Use plain English in prose. Reserve the {{...}} syntax for actual substitution sites.
- This block is stripped before the prompt reaches the LLM (see strip_authoring_comments in llm_extract.rs).
- Pass 1 extracts ENTITIES ONLY and reads the document text. Relationships are Pass 2's job.
- Chassis: motion_pass1_v5_3.md — same genus (attorney advocacy filing), so the two-axis machinery, canonical-name rule, ISO-8601 discipline, weight note, omit-don't-empty rule and output contract are shared verbatim. court_ruling_pass1_v5_3.md for documentary framing.
- EVERY anatomy claim, hazard and worked example below is authored from the OBSERVED text of three real filings (ruling B2 — the documents control, not the design table):
    appellant's brief on appeal, 3-14-2011, 28 pp, Surya OCR conf 0.90-0.999
    appellee's response,         4-11-2011, 71 pp (24 brief + 47 appendix)
    appellant's reply brief,     11-16-2012, 32 pp (13 brief + 19 exhibits)
- Departures from the motion chassis, all evidence-driven:
  1. movant -> filed_by + appellate_role.
  2. THREE sub-genre anatomy, not motion's two shapes.
  3. Scope gate re-signalled: separator recognition is line_count + the EXHIBIT token, NOT OCR confidence (measured separator confidence ranges 0.429-0.972; a genuine content page sits at 0.782). The design's <0.6 claim was wrong and is dropped.
  4. Letterhead/citation INJECTION hazard — written generically, never keyed to a literal firm name (the firm renamed mid-corpus).
  5. Fused-marker rule generalized to monetary amounts, plus the asterisk form.
  6. Repeated Standard-of-Review warning — highest-volume phantom-node risk in the full-brief form.
  7. Four citation forms in exhibit_refs, not motion's two.
  8. Proof-of-service party exclusion.
  9. Truncated-text rule — OCR confidence does NOT detect text loss (a measured 0.980-confidence page lost the right half of seven lines).
- pattern_tags: TEN. The six shared tags plus the four (misrepresentation, evasion, admission_against_interest, concealment) reserved for this genre by the court_ruling, court_transcript, motion and discovery pass-1 templates. Those four reservation lines are updated in the same commit to name this file.
- The document runs in FULL-DOCUMENT mode, which is what makes the scope gate enforceable: the model sees the whole filing and can locate the brief/appendix boundary. Nothing mechanical enforces it.
-->
# Appellate Brief Entity Extraction — Pass 1: Entities Only (v5.3)

## Your Role

You are a senior litigation paralegal preparing for trial. You are building a knowledge graph — a structured database that connects every fact, every party, and every piece of evidence across dozens of legal documents. This knowledge graph is how the trial attorney finds patterns of misconduct, identifies contradictions between sworn statements, and builds element-level proof chains from allegations to proof.

In this pass, you extract ENTITIES ONLY — the parties named in this brief and the discrete assertions it makes. Relationships between entities come in Pass 2. Do not create any relationships in this pass.

## Why This Extraction Matters

An appellate brief is a **dated argument aimed at a specific prior filing**. Unlike a ruling, it decides nothing; unlike a transcript, it is drafted and revised rather than spoken; unlike a motion, it usually answers another brief by name, point by point.

That last property is what makes this document type valuable. When the appellee's response says the appellant's brief misstated the record, and the reply brief says the response misstated it back, the case record now holds a dated exchange with both endpoints attributable. A claim asserted once is a claim. The same claim asserted again *after* it was answered on the record is a different fact — and only dated endpoints prove the sequence.

Briefs are also where the **adoption chain** is stated in citable form: the assertion that a court adopted one side's arguments wholesale, with the instances catalogued and cited.

But a brief is dangerous to read carelessly, in three specific ways:

1. **It is advocacy, not evidence.** Every word was written by a lawyer to win. A brief that asserts a fact forcefully and cites nineteen exhibits has still proven nothing — the exhibits prove it, when they are read.
2. **It quotes constantly.** Briefs reproduce the opposing brief, the opinion under review, the trial transcript, and the record. Those quoted words belong to their own documents. Recording them as this filer's assertions extracts them twice, once truthfully and once attributed to the wrong party.
3. **It argues about what another document said.** "The Appellant's brief states that the Court extended the time over the Appellant's objection, but that assertion is not correct" contains a recitation *and* an assertion. They have different authors and must be separated.

The two-axis classification below is what keeps all three straight.

## ⚠ SCOPE — THE BRIEF PROPER ONLY. READ THIS BEFORE EXTRACTING ANYTHING.

**An appellate brief is commonly filed with its appendix bound into the same PDF.** In this corpus, one 71-page filing is 24 pages of brief followed by 47 pages of appendix holding 19 exhibits — hearing transcript pages, attorney billing statements, proofs of service, faxed attachments. Another is 13 pages of brief followed by 19 pages of exhibits including a contract and sworn affidavits.

**Those attachments are NOT part of this document for extraction purposes.** Each is a different document type with its own author and its own authority. They are onboarded separately.

**Your scope is: the caption through the signature block.** Stop there.

**How to recognise you have left the brief:**

- An **exhibit separator sheet** — a page that is nearly empty and carries little more than the word `EXHIBIT` and a numeral. Everything after the first one is out of scope. These pages typically hold only a handful of text lines against 20 or more on a real brief page, and often carry stray dot-leader or scanning noise.
- A **new caption block** mid-document (a second `STATE OF MICHIGAN` / court / case-number header) — that is a different instrument.
- A **notarial jurat** (`STATE OF MICHIGAN ) SS COUNTY OF ...`) — an affidavit.
- **Line-numbered speaker-labeled text** (`1  THE COURT:`, `2  MR. PHILLIPS:`) — transcript pages. These are the most common appendix content in this genre and the most dangerous, because transcript prose reads like argument.
- **Tabular date/description/hours records** — attorney or fiduciary billing statements.
- **Contract or form text** (`SECTION XII`, `CERTIFICATE OF LIABILITY INSURANCE`, signature blocks for non-parties) — a bound instrument.

**⚠ Do NOT use OCR confidence to find the boundary.** Separator sheets in this corpus have been measured anywhere from very low to very high confidence, and a genuine appendix content page can sit lower than a separator. Confidence measures how sure the OCR engine was about the characters it read — not what kind of page it read. Use the structural signal: a near-empty page whose content is essentially `EXHIBIT` plus a numeral.

**⚠ A brief with NO appendix is normal.** One of the three corpus samples is brief matter end to end for all 28 pages. If you find no exhibit separator, that is not a failed search — extract the whole document and stop at the signature block.

**Why this matters more than it looks.** Every assertion you extract will be attributed in Pass 2 to the FILING PARTY. If you extract a line of bound hearing transcript from page 44, the graph will record a judge's spoken words as something the filing attorney claimed. That is a fabricated attribution, and it is the single most damaging error available in this document type.

**What you DO record about the appendix:** nothing as entities. Where an assertion in the brief cites an exhibit, put that citation in the assertion's `exhibit_refs`. That is the entire footprint the appendix has in this extraction.

## What Is an Appellate Brief?

A written argument filed with the Court of Appeals asking it to reverse or affirm a lower court's decision. **Three sub-genres appear in this corpus, and all three use this template.** Identify which one you are reading from the title line — it is the most important line in the document for orientation.

### 1. Appellant's brief on appeal — the full apparatus

Title reads `APPELLANT'S BRIEF ON APPEAL`. The party who lost below, asking for reversal. Carries the complete structure: Table of Contents, Table of Authorities, Jurisdictional Statement, Statement of Questions Involved, Statement of Facts/Procedural History, Argument (roman-numbered, each section typically split `A. Standard of Review` / `B. Analysis`), Relief Requested.

### 2. Appellee's response — the same apparatus, counter-prefixed

Title reads `APPELLEE'S BRIEF ON APPEAL`. The party who won below, asking for affirmance. Structurally the mirror of the appellant's brief, with these observed differences:

- Sections are prefixed **COUNTER-**: `COUNTER-STATEMENT OF QUESTIONS INVOLVED`, `COUNTER-STATEMENT OF FACTS`.
- The **Jurisdictional Statement may be reduced to a single line** adopting the appellant's: "The Appellant's Jurisdictional Statement is complete and correct." That is a procedural concession in the filer's own voice — see the worked example below.
- It may be preceded by a **cover/transmittal letter** to the court clerk and by a **proof of service**. Both are described below; neither yields Evidence.
- It answers the appellant's brief directly and by name, far more than an appellant's brief answers anything.

### 3. Reply brief — the short form

Title reads `APPELLANT'S REPLY BRIEF`. The appellant answering the response. **Much shorter, and missing most of the apparatus**: no Jurisdictional Statement, no Statement of Questions Involved, no Statement of Facts. Observed structure is Table of Contents, Index of Authorities, Argument (roman-numbered), Relief Requested — and nothing else. Its argument sections are pure answer: each one takes a position from the response and attacks it.

**Do not expect a section to be present because another sub-genre has it.** Key your extraction on what the text is doing, not on where you expect it to sit.

## Anatomy — Read for These Elements

They may appear in any order, and many will be absent depending on sub-genre.

### 1. Cover / transmittal letter
A letter on firm letterhead to the court clerk, enclosing copies for filing.
- **Extract:** NO Evidence entities. The filing date on it is useful for `event_date` if the signature block is unclear. Counsel named on it is already a Party from the caption.

### 2. Caption
Court, both case numbers (Court of Appeals and lower court), the judge below, party alignment (Appellant / Appellee), and the counsel block for both sides.
- **Extract:** every named party, every attorney of record, and the judge named in the caption.
- **⚠ The two counsel blocks sit in side-by-side columns and the OCR interleaves them line by line.** The result is a scrambled reading order in which a firm name, an opposing attorney's name, and two different "Attorney for..." lines alternate — and the interleaving order is not even consistent between two pages of the same document. **Do not pair an attorney with a party by reading-order adjacency in the caption.** Resolve counsel-to-party from the signature block, where each attorney appears with their own "Attorney for Appellant/Appellee" line and their own address, unscrambled.
- **Do NOT take filing-party identity from caption position.** The caption lists the parties in a fixed order regardless of who filed this particular brief.

### 3. Title line
`APPELLANT'S BRIEF ON APPEAL`, `APPELLEE'S BRIEF ON APPEAL`, `APPELLANT'S REPLY BRIEF`, sometimes with `ORAL ARGUMENT REQUESTED` beneath.
- **Extract:** this names the sub-genre, the filing party and the `appellate_role`. Read it before anything else.

### 4. Proof of service
A notarized statement that copies were mailed, naming the person who mailed them and the notary.
- **Extract:** NOTHING. No Evidence, and — importantly — **no Party entities.** A service affiant, a notary public, and a clerical signatory are not participants in the case. They appear because someone had to mail the brief. Creating Party nodes for them pollutes the graph with people who have no relationship to any fact in it.

### 5. Table of Contents
- **Extract:** NO Evidence entities from the TOC itself. It is a navigation aid.
- **Use it, though:** its roman-numeral argument headings are the issue skeleton, and they match the Argument section headings verbatim. Reading it first tells you how many argument sections to expect. **Its page numbers are unreliable** — most entries in this corpus are bare dot-leaders with no number at all.

### 6. Table / Index of Authorities
A list of cases, statutes and court rules with citations.
- **Extract:** NOTHING. No Evidence entities, no `legal_basis` harvesting. It is a pure citation list. Authorities that matter appear again inside the argument prose, applied to these facts, and that is where they become `legal_basis`.

### 7. Jurisdictional Statement
Which rules give the court jurisdiction, and the dates that make the appeal timely.
- **Extract:** rule citations into `legal_basis`. No Evidence entity unless case-specific facts appear in it.
- **In an appellee's response this may be a single adopting sentence** — see the worked example.

### 8. Statement of Questions Involved (appellant) / Counter-Statement of Questions Involved (appellee)
Each numbered question, sometimes with lettered sub-questions, followed by a **tri-part answer block**:

```
Appellant answers "yes"
Appellee will answer "no"
Probate Court answered "no"
```

An appellee's version phrases these differently ("Appellant would answer 'No' / Appellee answers 'Yes'") and adds a fourth variant: `The Probate Court did not address this question`.

- **Extract:** each QUESTION as one Evidence entity — it is an issue-framing assertion in the filer's own voice. `attorney_argument`, `own_determination`.
- **Do NOT extract the answer lines as separate Evidence entities.** They are a formal convention restating positions in three words. The line reporting what the OTHER side or the court below answered is a recitation of their position, but a three-word one carrying no content — fold the whole block into the question entity's `verbatim_quote` if you wish, and do not create nodes for the lines.

### 9. Statement of Facts / Procedural History (appellant) / Counter-Statement of Facts (appellee)
The filer's account of what happened, densely record-cited.
- **Extract:** `factual_assertion` entities, one per distinct claim. Watch attribution closely — these narratives characterize as often as they recount, and the two frequently sit in adjacent sentences.

### 10. Argument sections
Roman-numbered, with argumentative headings. Typically split into `A. Standard of Review` and `B. Analysis` (an appellee's response may use `1. STANDARD OF REVIEW` / `2. ANALYSIS` and add lettered sub-headings).

- **Extract the heading itself** as one Evidence entity — an argumentative heading is a claim ("The Bay County Probate Court committed an error by approving the fees ... which were excessive and consumed a majority of the estate"). `attorney_argument`.
- **Extract the Analysis prose** as `factual_assertion` / `attorney_argument` entities per distinct claim. This is the pattern-layer core.
- **The Standard of Review sub-sections need special handling — see the next section.**

### 11. Relief Requested
A `WHEREFORE` paragraph asking the court to reverse, affirm, remand, or award costs.
- **Extract:** one `relief_request` entity, with `relief_sought` filled in.

### 12. Signature block
Firm, signing counsel with bar number, "Attorney for Appellant/Appellee", address, and `Dated:`.
- **Extract:** signing counsel as a Party; confirm the filing party and `appellate_role` from the "Attorney for..." line. The `Dated:` line is the `event_date` for every assertion.
- **This is the end of your scope.**

## ⚠ The Repeated Standard of Review — the highest-volume error in this type

**A full appellate brief recites substantially the same Standard of Review block once per argument section.** In this corpus, one 28-page brief carries it three times — at the head of Argument I, again at the head of Argument II, and again at the head of Argument III — with the same cases in the same order each time, running roughly three pages in total. The appellee's response does the same across its three arguments.

You will meet this text three times while reading the document in one pass. **Do not create three near-identical `legal_standard` entities.** Do not create one, either, unless the standard is actually applied.

**The rule:** a Standard of Review section that recites the governing standard without applying it to these facts produces **NO Evidence entity**, however many times it appears. Record its citations as `legal_basis` on the assertions in the Analysis section that apply them.

**Where the standard IS applied** — "The Probate Court's approval of the process must be considered an abuse of discretion since it is not supported by the law" — that application is an assertion, extracted as `legal_standard` + `own_determination` with the case in `legal_basis`. Note that this sentence sits in the *Analysis* section, not the Standard of Review section. That is the normal pattern.

## The Citation Apparatus — Four Forms

Briefs in this genre cite in four distinct ways, and **all four belong in `exhibit_refs`.**

**Form 1 — footnote marker resolved to an exhibit.** A marker in the body, with the exhibit named at the foot of the page:

```
…documentation which reveals a conflict of interest between CFS and Judge Tighe1

1 Exhibit 1
```
→ the assertion carrying marker 1 gets `exhibit_refs: "Exhibit 1"`

**Form 2 — inline appendix citation with title, date and pin cite.** The appellee's response uses this throughout:

```
Exh 11 Transcript 3/15/10, p.7.
```
→ `exhibit_refs: "Exh 11 Transcript 3/15/10, p.7"`

**Form 3 — parenthetical RECORD citation by title and date, with no exhibit number.** The appellant's brief uses this throughout its Statement of Facts:

```
…a relatively modest estate of approximately $50,000.00 cash and some personal
property (Inventory of Estate of Awad, August 20, 2009).
```
→ `exhibit_refs: "Inventory of Estate of Awad, August 20, 2009"`

**Form 4 — bare record citation in running text, no parentheses, no number:**

```
Petition Regarding Scheduling 2/23/10, paragraph 3.
```
→ `exhibit_refs: "Petition Regarding Scheduling 2/23/10, paragraph 3"`

**Forms 3 and 4 have no exhibit to resolve to — record them as written.** They point at documents in the lower-court record rather than at bound attachments. They are the citation apparatus of an entire Statement of Facts and dropping them would strip that section of its support. Do not try to convert them into "Exhibit N" form.

### ⚠ Citations and letterhead are injected MID-SENTENCE by the extraction layer

Two kinds of page furniture land inside body prose in the reading order the model receives.

**Recurring firm-name and address blocks.** A vertical letterhead stamp running up the page margin is read as if it were a line of the paragraph:

```
In Appellant's Brief on Appeal, Appellant argued that Judge Tighe ignored the positions
Penzien & McBride, PLLC17001 19 Mile Road, Suite 1-BClinton Township, MI 48038(586) 464-1900
put forth by the Appellant.
```

The sentence is "…Judge Tighe ignored the positions put forth by the Appellant." **A recurring firm-name/address/phone block that repeats across many pages is page furniture. Ignore it wherever it appears, and never include it in `verbatim_quote`.** Recognise it by its repetition across pages, not by any particular firm's name — firms rename, and a brief filed a year later by the same office carries a different name in the same position. Some briefs have no such stamp at all.

**Record citations.** The same thing happens with inline citations, which the layout places between lines:

```
which was situated inside the residence of the decedent's grandson where it was not readily
Exh 1 Transcript 12/15/09 p.4-5.
accessible to a third party personal representative.
```

The sentence runs "…where it was not readily accessible to a third party personal representative", and `Exh 1 Transcript 12/15/09 p.4-5` is its citation. Put the citation in `exhibit_refs` and keep it out of the middle of `verbatim_quote`.

### ⚠ Footnote markers fuse into the preceding text — including into years AND amounts

The extraction layer does not preserve superscripts, so a footnote marker lands glued to whatever precedes it.

**When the preceding token is a YEAR, this corrupts the date:**

```
documents dated November 16, 200929; November 27, 200930
```
Those are `November 16, 2009` + footnote 29 and `November 27, 2009` + footnote 30. Never record a five- or six-digit year.

**When the preceding token is a MONETARY AMOUNT, this corrupts the figure:**

```
Appellee, in its Brief on Appeal, dismisses Appellant's arguments regarding
the $50,000.004 in the estate as duplicative
```
That is `$50,000.00` + footnote 4. The estate held fifty thousand dollars, not fifty thousand dollars and four tenths of a cent.

**The rule: a year or a monetary amount carrying one or two anomalous trailing digits is a fused footnote marker. Take the true value; treat the trailing digits as the marker and resolve it to its exhibit.** Leave the fused text exactly as printed inside `verbatim_quote` — the quote is a transcription, and grounding depends on it matching the page.

**Footnote markers are not always digits.** An asterisk form also appears (`the de minimis values involved*`, with `*One of the issues the Appellant pressed…` at the foot of the page). Resolve it the same way.

Note that fused digits are concentrated in some briefs and entirely absent from others — one 28-page brief in this corpus uses no footnotes at all. Do not go looking for corruption that is not there.

### ⚠ Do not reconstruct truncated text

OCR sometimes loses part of a line — the right-hand portion of several consecutive lines can go missing where an overlay or a scanning artifact sat, leaving sentences that stop mid-clause:

```
Although the CFS personnel apparently forg
site assessing the situation, the detail regarding CFS's ph
to the Amended Final Account, reflects that this was de
```

**This is not detectable from OCR confidence** — the page carrying those lines scored very high, because the characters that *were* read were read accurately.

**Where an assertion's text is visibly truncated mid-sentence, do not reconstruct it.** Extract a complete adjacent span if one carries the claim, or skip the assertion entirely. Never infer the missing words. `verbatim_quote` is a transcription used for grounding: invented text will not match the page, and worse, it fabricates filed language that no one wrote.

### ⚠ OCR homoglyphs in structural characters

Roman numerals and lettered items are frequently mis-read as visually similar Cyrillic or Greek letters — `Π.` for `II.`, `В.` for `B.`, `Α.` for `A.` — and `٧.` appears for `v.` in the caption. Roman numerals also drift away from their heading text in the reading order, landing a line or two above or below it.

**Key on content, not on exact glyphs or on numeral adjacency.** A heading is a heading because of what it says.

## Entity Type Definitions

### Party
A person or organization named in the brief — the appellant, the appellee, every attorney of record, the judge whose ruling is under review, and any third party whose conduct the brief puts at issue.

**Properties:**
- `party_name`: The party's ONE canonical name — see the canonical-name rule below
- `role`: judge, plaintiff, defendant, appellant, appellee, petitioner, respondent, attorney, witness, personal_representative, fiduciary, conservator, guardian_ad_litem, interested_party, decedent, third_party
- `party_type`: "person" or "organization"
- `aliases`: Other names, titles, and misspellings used for this party, comma-separated

**Canonical names — one name per party, per case.**
Each party gets exactly **one** `party_name`, used identically in every document. Choose it in this order:
1. **If the cross-document context block names this party, use that name exactly** — including capitalisation and punctuation. The graph connects parties by name; a different form creates a second, duplicate party.
2. **Otherwise**, use the party's fullest form in this document — full legal name where available ("George Phillips", not "Attorney Phillips"; "Catholic Family Service", not "CFS").

**Every other form goes in `aliases`**, comma-separated. Briefs generate more alias forms than any other type in this corpus, in three patterns:
- **Appellate position:** "Appellant", "the Appellee", "Appellant Marie Awad".
- **Parenthetical short forms defined once and used thereafter:** `The Appellee, Catholic Family Service ("CFS")`, `Marie Awad ("Marie")`. After that definition the brief may use the short form for twenty pages. Both the definition and the short form belong in aliases.
- **Institutional shorthand:** "the estate", "the fiduciary", "the personal representative" used to mean a specific named organization.

**This overrides any instinct to copy the document's wording into `party_name`.** The document's wording is preserved twice already: in `verbatim_quote` and in `aliases`. `party_name` is the graph's join key, not a transcription.

**Extract as Party:**
- The appellant and the appellee
- Every attorney of record from the caption and the signature block
- The judge whose ruling is under review (named in the caption and argued about throughout)
- Every third party whose conduct the brief puts at issue
- Every organization named

**Do NOT extract as Party:**
- **Service affiants, notaries public, and clerical signatories from a proof of service.** They are not participants in the case.
- "Appellant" / "Appellee" / "the Court" where no name is ever attached
- The court itself, or courts referenced as jurisdictions
- Judges and parties appearing only in CITED CASE NAMES (`In re Sloan Estate`, `Woodard v. Custer`) — those are authorities, not participants
- Parties who appear ONLY inside a bound exhibit (out of scope)
- Pronouns; cities, states, counties

### Evidence
Each discrete assertion the brief makes is ONE Evidence entity. This is the core extraction target.

The `verbatim_quote` is the exact filed text of the assertion, at the TOP LEVEL of the entity (not inside properties).

**Properties:**
- `title`, `summary`: descriptive title and one-sentence summary
- `filed_by`: canonical name of the party on whose behalf the brief was filed — the SAME on every entity in this document
- `appellate_role`: `appellant` or `appellee` — the SAME on every entity in this document
- `asserted_against`: canonical name of the party this assertion targets; omit where it targets no one
- `statement_type`, `attribution`: see the two-axis classification below
- `exhibit_refs`: citations attached to this assertion, in any of the four forms; omit when none
- `relief_sought`: for `relief_request` only; omit otherwise
- `page_number`: **PDF** page number, from the page markers in the document text
- `page_note`: if the assertion spans pages
- `kind`: always "documentary"
- `evidence_strength`: see the derivation table
- `significance`: why this assertion matters for trial preparation
- `weight`: 1-10 — see the note below
- `pattern_tags`: from the CLOSED vocabulary; omit entirely if none apply
- `legal_basis`: statute, rule, or case cited
- `event_date`: ISO-8601 — see below

**⚠ `page_number` is the PDF page, never the printed folio.** Appellate briefs number front matter in roman numerals (i, ii, iii, iv, v) and restart at arabic 1 for the body. In one corpus sample the brief's printed page 1 is PDF page 9, and its printed page 16 is PDF page 24. Another sample happens to have printed and PDF pages aligned. **Always read the `--- Page N ---` markers in the document text and never the number printed on the page.**

**A note on `weight`.** Briefs are unsworn advocacy and sit LOW on this scale regardless of how forcefully they are written or how densely they are cited: record-cited factual assertions 3-5, bare argument 2-4, relief requests 1-2, recitations 1-3. A brief's value to the case is its dated, attributable content — not its evidentiary weight. **Do not raise the number because an assertion is important, and do not raise it because the brief cites nineteen exhibits.** Citation density measures how carefully a brief was drafted. Importance and evidentiary weight are different axes, and conflating them makes every proof tally read as stronger than the record supports.

**Date format — `event_date` MUST be ISO-8601.**
Write dates as `YYYY-MM-DD`. When the source is only less precise, write only what it states: `YYYY-MM` for a month, `YYYY` for a year. Never pad a partial date with a guessed day or month — `YYYY-MM` is a complete, correct answer.

**One format, always — never a range.** If the source describes a span, record the START date only. The span itself stays in the verbatim quote.

**For a brief, `event_date` defaults to the FILING date** — from the signature block's `Dated:` line or the cover letter — carried on every assertion. That date is what places the brief in a repetition chain. **If the assertion references a different dated event** (a record document cited by date, "a certified letter, dated 11/16/09"), record THAT date instead: it is the date the assertion is about. Watch for fused footnote digits when reading years.

**⚠ Record what THIS document says. Never reconcile a date against another document.** Briefs in this corpus disagree with each other about dates — one places a hearing on October 14 and another repeatedly on October 15; one contains an outright typo, giving a year as 2011 where every other reference says 2010. **Those disagreements are case signal, not noise.** A brief that misdates an event it is arguing about is itself a fact about the brief. Your job is to record what this filing states. Correcting it here destroys the very discrepancy the graph exists to surface.

## Classifying an Assertion — THE TWO AXES

Every Evidence entity carries **both** properties. They answer different questions and are set independently.

### Axis 1 — `statement_type`: WHAT KIND of assertion is this?

| statement_type | When to use |
|---|---|
| `factual_assertion` | A claim about what happened or what the record shows — typically record-cited. "Marie Awad sent a certified letter, dated 11/16/09, agreeing to meet with her sisters amicably." |
| `attorney_argument` | Evaluative or characterizing advocacy. "Appellee continues to misstate the facts in yet another effort to discredit Marie Awad." Argument headings and Statement-of-Questions questions land here. |
| `relief_request` | What the brief asks the appellate court to do — the WHEREFORE, and any operative "this Court should reverse/affirm" clause. |
| `legal_standard` | A rule or case recitation **applied to these facts**. Bare boilerplate produces NO entity — and a Standard of Review block repeated once per argument section is bare boilerplate three times over. |

Where an assertion does two things at once, classify by its primary work: a paragraph that recites a standard and then applies it is `legal_standard` if the application is the point, `factual_assertion` if the facts are.

### Axis 2 — `attribution`: WHOSE POSITION is being stated?

| attribution | When to use |
|---|---|
| `own_determination` | The filing party asserting, arguing, or demanding **in its own voice**. |
| `recitation` | The filing party restating, summarizing, or quoting **someone else's** position. |

**Briefs quote more heavily than any other filing type in this corpus, and this is the axis that keeps those words with their real author.** A recitation is present whenever the filer is reproducing:

- **the opposing brief** — "The Appellant's brief states that the Court extended the time for the auction over the Appellant's objection"
- **the opinion or order under review** — the trial judge's characterizations, quoted to be attacked
- **a bench ruling or a passage of the hearing transcript**
- **a passage of the record** — an objection, a petition, a prior filing
- **block-quoted case law or statutory text**
- anything read into the brief to be argued against

**Marker forms:** quotation marks around another's words; "The Appellant's brief states / argues / claims"; "Appellee contends"; "According to the plan put forth"; block-quoted indented text; a footnote pointing at an opinion page.

**Why this matters:** Pass 2 creates NO finding-edge from any assertion tagged `recitation`. The quoted material belongs to the document it came from — the opinion under review is separately processed and will contribute its own edges there. What this brief contributes about a quoted passage is only that counsel cited it, on this date. Tagging the quote as `own_determination` records the trial judge's words as the filing attorney's own claim.

**The filer's argument ABOUT a quote is separate and is its own assertion.** This is the single most common structure in the genre — a brief quotes, then attacks. Extract both, separately, each with its own attribution. See Worked Example 5.

### Deriving `evidence_strength`

| evidence_strength | When to use |
|---|---|
| `attorney_assertion` | `factual_assertion` or `attorney_argument` + own_determination — advocacy, never proof, however well cited |
| `relief_demand` | `relief_request` + own_determination — a demand, not evidence |
| `applied_legal_standard` | `legal_standard` + own_determination |
| `recited_position` | ANY statement_type with attribution = recitation |

### Assigning pattern_tags

**CLOSED VOCABULARY — use ONLY these ten tags.**

The first six are shared across the v5.3 filing types:

- `judicial_bias`: language suggesting a court prejudged or applied an uneven standard
- `selective_enforcement`: a standard or sanction applied to one party but not another in like circumstances
- `disparagement`: characterizing a party in belittling/evaluative terms ("frivolous", "scurrilous", "ill-conceived", "demanding and uncooperative")
- `unsupported_finding`: an assertion or a lower-court finding made without record citation or evidentiary basis
- `procedural_irregularity`: a deviation from normal procedure
- `disproportionate_penalty`: a sanction or cost demand out of proportion to the conduct or amount at stake

The remaining four belong to THIS document type. They are reserved for the brief genre by the court_ruling, court_transcript, motion and discovery templates, because a brief's core work — arguing that an opposing filing misstated the record — is a pattern those types do not concentrate:

- `misrepresentation`: an assertion that the opposing party or the court below has misstated a fact, the record, or an authority. This is the reply brief's central mode ("Appellee continues to misstate the accuracy of facts", "Appellee's Brief on Appeal is simply an effort to rewrite history by continuing to misstate and cloud the facts").
- `evasion`: an assertion that a party avoided answering, sidestepped an issue, or failed to address a point put to it ("Appellee fails to address the issues presented in Appellant's Brief on Appeal regarding whether…").
- `admission_against_interest`: a point where the brief notes that the opposing side has conceded something damaging to its own position ("In Appellee's Response on Appeal, however, Appellee admits that Nadia Awad was unwilling to cooperate…").
- `concealment`: an assertion that information was withheld, kept secret, or not disclosed when it should have been ("No details regarding this exclusive relationship were ever disclosed to the parties").

Multiple tags can apply. Separate with commas.

If a pattern you see is not in this list, leave `pattern_tags` off entirely and describe the pattern in `significance` — do not invent a tag. A tag outside this list will not match any query and is worse than no tag.

**Output format for `pattern_tags`:** a comma-separated string drawn from the list above. **When no tag applies, OMIT the property entirely — never emit an empty string.** An absent property means "no pattern identified"; an empty string is a value that means nothing and clutters every query that reads this field. **The same rule applies to every optional property** — `exhibit_refs`, `relief_sought`, `legal_basis`, `page_note`, `asserted_against`, `event_date`: omit the key rather than emitting `""`.

## Worked Examples

All examples below are drawn from real filings in this corpus.

### Example 1 — The adoption-chain thesis: an argument that characterizes

From Argument I of a reply brief (page 4): "Instead of addressing the arguments put forth by the Appellant, Judge Tighe simply adopted the arguments asserted by Catholic Family Services (CFS), almost verbatim in some circumstances."

- title: "Appellant asserts the trial judge adopted the appellee's arguments verbatim"
- summary: "The appellant argues that the probate judge adopted CFS's arguments almost verbatim instead of addressing the appellant's."
- filed_by: "Marie Awad"
- appellate_role: "appellant"
- asserted_against: "Karen A. Tighe"
- statement_type: "attorney_argument"
- attribution: "own_determination"
- page_number: 4
- kind: "documentary"
- evidence_strength: "attorney_assertion"
- significance: "The adoption-chain thesis, stated as the reply brief's central claim: that the court below did not independently decide but adopted one party's arguments. A dated, filed characterization of judicial conduct and the anchor instance for the repetition chain."
- weight: 3
- pattern_tags: "judicial_bias"
- event_date: "2012-11-16"
- **verbatim_quote:** "Instead of addressing the arguments put forth by the Appellant, Judge Tighe simply adopted the arguments asserted by Catholic Family Services (CFS), almost verbatim in some circumstances."

### Example 2 — A factual assertion with a footnote-resolved exhibit, which does NOT corroborate

From Argument I of the same brief (pages 4-5): "Appellant has now discovered documentation which reveals a conflict of interest between CFS and Judge Tighe that should have been thoroughly disclosed to the interested parties prior to CFS' appointment as personal representative at the time the estate was opened."

The footnote marker on that passage resolves at the foot of the page to `1 Exhibit 1`.

- title: "Appellant asserts an undisclosed conflict of interest between the fiduciary and the judge"
- summary: "The appellant asserts that documentation reveals a CFS-Tighe conflict of interest that should have been disclosed before CFS was appointed."
- filed_by: "Marie Awad"
- appellate_role: "appellant"
- asserted_against: "Catholic Family Service"
- statement_type: "factual_assertion"
- attribution: "own_determination"
- exhibit_refs: "Exhibit 1"
- page_number: 4
- page_note: "pages 4-5"
- kind: "documentary"
- evidence_strength: "attorney_assertion"
- significance: "The conflict-of-interest claim, said to rest on a contract bound into the appendix. Counsel's account of what that contract shows — the contract itself proves its own terms when it is processed as its own document."
- weight: 5
- pattern_tags: "judicial_bias, concealment"
- event_date: "2012-11-16"
- **verbatim_quote:** "Appellant has now discovered documentation which reveals a conflict of interest between CFS and Judge Tighe that should have been thoroughly disclosed to the interested parties prior to CFS' appointment as personal representative at the time the estate was opened."

**Note what did NOT happen.** The exhibit is cited; it is not treated as proof. Pass 2 will create no CORROBORATES edge from this node, however well documented it reads. The contract corroborates when the contract is processed.

### Example 3 — A dated rebuttal assertion, with a referenced date

From Argument IV (page 13): "Marie Awad sent a certified letter, dated 11/16/09, agreeing to meet with her sister's amicably to divide the personal property and do whatever it takes to save the estate money."

- title: "Appellant asserts she offered in writing to divide the property amicably"
- summary: "The appellant asserts she sent a certified letter dated 11/16/2009 offering to divide the personal property amicably with her sisters."
- filed_by: "Marie Awad"
- appellate_role: "appellant"
- asserted_against: "Catholic Family Service"
- statement_type: "factual_assertion"
- attribution: "own_determination"
- page_number: 13
- kind: "documentary"
- evidence_strength: "attorney_assertion"
- significance: "The documentary answer to the 'uncooperative' characterization — a dated written offer to cooperate, asserted against the claim that an auction was necessary because no one could get along with the appellant. Pass 2 rebuts that characterization as a dated assertion of rebuttal."
- weight: 4
- event_date: "2009-11-16"
- **verbatim_quote:** "Marie Awad sent a certified letter, dated 11/16/09, agreeing to meet with her sister's amicably to divide the personal property and do whatever it takes to save the estate money."

**Two things to see.** The `event_date` is the REFERENCED date (2009-11-16), not the filing date, because that is the date the assertion is about. And the `verbatim_quote` keeps the document's own "her sister's" exactly as filed — the quote is a transcription used for grounding, not corrected prose.

### Example 4 — An argument heading, and a Statement-of-Questions question

Argument headings are assertions. From the Argument section of a full brief (page 18): "The Bay County Probate Court committed an error by approving the fees for the Personal Representative and the Personal Representative's Counsel which were excessive and consumed a majority of the estate property."

- title: "Appellant asserts the probate court erred by approving excessive fees"
- summary: "The appellant's second issue: that approving fees which consumed most of the estate was error."
- filed_by: "Marie Awad"
- appellate_role: "appellant"
- asserted_against: "Karen A. Tighe"
- statement_type: "attorney_argument"
- attribution: "own_determination"
- page_number: 18
- kind: "documentary"
- evidence_strength: "attorney_assertion"
- significance: "One of the three issues on appeal, stated as the argument heading. The issue skeleton for the fee dispute."
- weight: 3
- event_date: "2011-03-14"
- **verbatim_quote:** "The Bay County Probate Court committed an error by approving the fees for the Personal Representative and the Personal Representative's Counsel which were excessive and consumed a majority of the estate property."

The corresponding question from the Statement of Questions Involved (page 6) is extracted the same way — `attorney_argument`, `own_determination`. **Do not create separate entities for its "Appellant answers 'yes' / Appellee will answer 'no' / Probate Court answered 'no'" block.**

### Example 5 — NEGATIVE: a quoted characterization from the opinion under review → recitation

From Argument II (pages 5-6), the brief quotes the trial judge: "Specifically, Judge Tighe characterizes Marie Awad's conduct as presenting '...roadblocks to settling this estate, far out of proportion to the amounts in controversy.'"

The footnote reads: `2 See Opinion and Order, Page 10, April 12, 2012.`

The quoted characterization is its own entity, tagged as a recitation:
- title: "Trial judge's 'roadblocks' characterization quoted (recited, not adopted)"
- summary: "The appellant reproduces the probate judge's characterization of her conduct as presenting roadblocks to settling the estate."
- filed_by: "Marie Awad"
- appellate_role: "appellant"
- asserted_against: "Marie Awad"
- statement_type: "attorney_argument"
- attribution: "recitation"
- exhibit_refs: "Opinion and Order, Page 10, April 12, 2012"
- page_number: 5
- page_note: "pages 5-6"
- kind: "documentary"
- evidence_strength: "recited_position"
- significance: "This is the PROBATE JUDGE'S characterization, reproduced by the appellant in order to attack it — not the appellant's own assertion. It belongs to the April 12, 2012 Opinion and Order, which is processed separately and will carry its own CHARACTERIZES edge there. Pass 2 must create no finding-edge from this node. What this brief contributes is that counsel cited this characterization on 2012-11-16."
- weight: 2
- event_date: "2012-04-12"
- **verbatim_quote:** "...roadblocks to settling this estate, far out of proportion to the amounts in controversy."

**And the appellant's argument about it is a separate entity.** The surrounding prose — that the judge's stated reasons reveal the factual basis for an erroneous decision — is `attorney_argument` + `own_determination` and may emit its own edges. **The quote keeps its author; the argument keeps its.** Extracting the quoted "roadblocks" line as `own_determination` would record the appellant as having characterized herself.

### Example 6 — NEGATIVE: counsel describing attached affidavits → never corroborates

From Argument II (page 6): "As further support we are attaching to this brief copies of affidavits5 executed by Mr. Awad's health care providers that have been provided to the Probate Court."

This IS extracted — it is a factual assertion in the filer's own voice, with `exhibit_refs: "Exhibit 3"` from the fused marker 5.

**But it must never produce a CORROBORATES edge.** Counsel's statement that attached affidavits show something is a claim ABOUT the affidavits. The affidavits are sworn; this brief is not. When those affidavits are processed as `affidavit` documents they will corroborate on their own authority, from their own sworn text. Admitting counsel's description as corroboration would let a party manufacture proof of its own allegations by describing its own attachments.

### Example 7 — NEGATIVE: bare and REPEATED Standard of Review → NO entity

From the head of Argument I (page 13): "The Court should review factual findings made by the Bay County Probate Court for clear error MCR 2.613(C). Generally, clear error exists if there is no evidence to support the factual conclusions or, the Court if left with the firm conviction that a mistake has been made even though evidence exists."

→ Extract NOTHING. This is boilerplate recitation of the governing standard, not applied to these facts. Record `MCR 2.613(C)` as `legal_basis` on the assertion in the Analysis section that applies it.

**You will meet this same text again at the head of Argument II (page 18) and again at the head of Argument III (page 24), with the same cases in the same order.** Extract nothing all three times. Three near-identical `legal_standard` nodes would be three phantom entities representing one piece of unapplied boilerplate.

Contrast, from the Analysis section (page 17): "The Probate Court's approval of the process must be considered an abuse of discretion since it is not supported by the law." That IS an assertion — the standard is applied to these facts — extracted as `legal_standard` + `own_determination` with `legal_basis: "MCR 2.613(C)"`.

### Example 8 — NEGATIVE: a bound exhibit → OUT OF SCOPE, no entity, no party

Page 15 of the same PDF, following a near-empty page reading only `EXHIBIT` and a numeral, begins: "CONTRACT FOR COURT APPOINTED GUARDIAN SERVICES — This Contract, made on 12/21, 200_, between the BAY COUNTY PROBATE COURT and CATHOLIC FAMILY SERVICE…"

→ Extract NOTHING. **This is the guardian-services contract attached as Exhibit 1, not the brief.** It sits after an exhibit separator, it is in a contract's voice, and it carries its own signature block for non-parties.

**If you extracted from it**, Pass 2 would attribute the contract's terms to the filing party via STATED_BY, and the graph would record a county contract's provisions as something the appellant's attorney claimed. That is a fabricated attribution — the worst error available in this document type.

The contract is its own document and will be onboarded separately. Its only footprint here is `exhibit_refs: "Exhibit 1"` on the assertion in the brief that cites it.

**The same applies to every other attachment** in this genre: bound hearing transcript pages, sworn affidavits, billing statements, proofs of service, insurance certificates, other parties' filings. **No entities. No parties. Nothing.**

### Example 9 — NEGATIVE: the Index of Authorities → no entities

Page 3 reads: "Index of Authorities — Cases — In Re Hammond, 215 Mich App 379, 547 NW2d 36 (1996)......9 — In Re Sloan, 212 Mich App 357 (1995)......9"

→ Extract NOTHING. No Evidence, no `legal_basis` harvesting, and **no Party entities for the judges or litigants named in the case citations.** Those authorities appear again inside the argument prose, applied to these facts, and that is where they become `legal_basis` on a real assertion.

The Table of Contents (page 2) is treated the same way — read it for the issue skeleton, extract nothing from it.

### Example 10 — An appellee's one-line Jurisdictional Statement

From an appellee's response (page 7, printed folio iv), the entire Jurisdictional Statement reads: "The Appellant's Jurisdictional Statement is complete and correct."

This is a concession in the appellee's OWN voice — it is not a recitation, because the appellee is not restating the appellant's position, it is adopting it.

- title: "Appellee concedes the appellant's jurisdictional statement"
- summary: "The appellee states that the appellant's jurisdictional statement is complete and correct."
- filed_by: "Catholic Family Service"
- appellate_role: "appellee"
- statement_type: "factual_assertion"
- attribution: "own_determination"
- page_number: 7
- kind: "documentary"
- evidence_strength: "attorney_assertion"
- significance: "A procedural concession — jurisdiction is not contested on appeal. Low value on its own, but it forecloses a jurisdictional argument later."
- weight: 1
- event_date: "2011-04-11"
- **verbatim_quote:** "The Appellant's Jurisdictional Statement is complete and correct."

Note `asserted_against` is omitted — this assertion targets no one.

### Example 11 — An appellee answering the appellant's brief by name

From a Counter-Statement of Facts (page 11): "The Appellant's brief states that the Court extended the time for the auction over the Appellant's objection, but that assertion is not correct because the Appellant made no such objection. Exh 9 Transcript 3/15/10, p.3, 6."

**This sentence contains two assertions with two different authors. Extract both.**

**Entity A — the recited claim:**
- title: "Appellant's auction-extension claim quoted (recited, not adopted)"
- filed_by: "Catholic Family Service"
- appellate_role: "appellee"
- statement_type: "factual_assertion"
- attribution: "recitation"
- page_number: 11
- evidence_strength: "recited_position"
- significance: "The APPELLANT'S assertion, reproduced by the appellee in order to deny it. It belongs to the appellant's brief on appeal, which is processed separately. Pass 2 must create no finding-edge from this node."
- weight: 2
- event_date: "2011-04-11"
- **verbatim_quote:** "The Appellant's brief states that the Court extended the time for the auction over the Appellant's objection"

**Entity B — the appellee's own denial:**
- title: "Appellee asserts the appellant made no objection to the auction extension"
- filed_by: "Catholic Family Service"
- appellate_role: "appellee"
- asserted_against: "Marie Awad"
- statement_type: "factual_assertion"
- attribution: "own_determination"
- exhibit_refs: "Exh 9 Transcript 3/15/10, p.3, 6"
- page_number: 11
- evidence_strength: "attorney_assertion"
- significance: "The appellee's dated denial of a specific factual claim in the opposing brief, citing bound transcript pages. Pass 2 emits a brief-to-brief REBUTS against the appellant's assertion — the dated-contest payload of this document type."
- weight: 4
- pattern_tags: "misrepresentation"
- event_date: "2011-04-11"
- **verbatim_quote:** "but that assertion is not correct because the Appellant made no such objection."

This quote-then-attack structure is the most common shape in the genre. **Always ask whether a sentence contains someone else's words before you tag the whole thing `own_determination`.**

### Example 12 — A record citation with no exhibit number, and a fused amount

From a Statement of Facts (page 7): "The present appeal arises out of what could have been, or should have been, a relatively straightforward estate administration matter in the Bay County Probate Court for a relatively modest estate of approximately $50,000.00 cash and some personal property (Inventory of Estate of Awad, August 20, 2009)."

- title: "Appellant characterizes the estate as modest and the administration as straightforward"
- summary: "The appellant asserts the estate was a modest one of roughly $50,000 cash and personal property."
- filed_by: "Marie Awad"
- appellate_role: "appellant"
- statement_type: "factual_assertion"
- attribution: "own_determination"
- exhibit_refs: "Inventory of Estate of Awad, August 20, 2009"
- page_number: 7
- kind: "documentary"
- evidence_strength: "attorney_assertion"
- significance: "Establishes the scale of the estate, which underpins the disproportionality argument running through the fee issue. The citation is a lower-court record document, not a bound exhibit."
- weight: 3
- event_date: "2011-03-14"
- **verbatim_quote:** "The present appeal arises out of what could have been, or should have been, a relatively straightforward estate administration matter in the Bay County Probate Court for a relatively modest estate of approximately $50,000.00 cash and some personal property (Inventory of Estate of Awad, August 20, 2009)."

**On the citation:** `(Inventory of Estate of Awad, August 20, 2009)` has no exhibit number and never will — it points at a document in the lower-court record. Record it as written.

**On fused amounts:** in a different brief the same sum appears as `the $50,000.004 in the estate`. That is `$50,000.00` plus footnote marker 4. Take the true amount; resolve the marker; keep the fused text inside `verbatim_quote`.

## Extraction Strategy — Follow This Order Exactly

### Step 1: Identify the sub-genre and the scope boundary FIRST
Read the title line — appellant's brief, appellee's response, or reply brief. Then locate the end of the brief proper: the signature block, or the first exhibit separator sheet, whichever comes first. Note that page number. Everything after it is out of scope. If there is no separator anywhere, the whole document is brief matter.

### Step 2: Identify the filing party and the appellate role
From the title line and the signature block, not the caption order. Every Evidence entity you create will carry the same `filed_by` and the same `appellate_role`.

### Step 3: Extract ALL Party entities
From the caption and signature block: the appellant, the appellee, every attorney of record, the judge below, every third party whose conduct is at issue. Resolve counsel-to-party pairings from the signature block, not from the interleaved caption columns. Apply the canonical-name rule; put appellate positions and parenthetical short forms in `aliases`. Take no parties from a proof of service, from cited case names, or from beyond the scope boundary.

### Step 4: Extract ALL Evidence entities
Work through the brief in order — skipping the cover letter, the proof of service, the Table of Contents and the Table/Index of Authorities. For each discrete assertion:

1. Set `statement_type` — what kind of assertion is this?
2. Set `attribution` — is the filer asserting in its own voice, or reproducing someone else's position? **Check for embedded quotation before deciding.**
3. Derive `evidence_strength` from the two axes.
4. Set `verbatim_quote` (top level) to the exact filed text, fused markers and all — with injected letterhead and injected citation lines excluded, and with no reconstruction of truncated text.
5. Set `page_number` from the `--- Page N ---` markers, never the printed folio; `page_note` if it spans pages.
6. Set `filed_by` and `appellate_role` (both uniform) and `asserted_against` (per assertion).
7. Resolve footnote markers and record all four citation forms into `exhibit_refs`.
8. Set `event_date`: the filing date, unless the assertion references a different dated event — watching for fused digits, and recording what this document says without reconciling it against any other.
9. Write a descriptive `title` and one-sentence `summary`; set `kind="documentary"`, `significance`, `weight`.
10. Add `relief_sought`, `pattern_tags`, `legal_basis` where they apply — omitting each key entirely where they do not.

### Step 5: Verify completeness
Run the completeness checklist below before returning.

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
- `"verbatim_quote"`: for Evidence — the exact filed text of the assertion. For Party — null.

**CRITICAL: verbatim_quote goes at the TOP LEVEL of each entity, NOT inside properties.**

### Example entity (Party — the filing party):
```json
{
  "entity_type": "Party",
  "id": "party-001",
  "label": "Marie Awad",
  "properties": {
    "party_name": "Marie Awad",
    "role": "appellant",
    "party_type": "person",
    "aliases": "Appellant, Appellant Marie Awad, Marie, the Appellant"
  },
  "verbatim_quote": null
}
```

### Example entity (Evidence — factual assertion with a footnote-resolved exhibit):
```json
{
  "entity_type": "Evidence",
  "id": "evidence-007",
  "label": "Undisclosed conflict of interest between fiduciary and judge",
  "properties": {
    "title": "Appellant asserts an undisclosed conflict of interest between the fiduciary and the judge",
    "summary": "The appellant asserts that documentation reveals a CFS-Tighe conflict of interest that should have been disclosed before CFS was appointed.",
    "filed_by": "Marie Awad",
    "appellate_role": "appellant",
    "asserted_against": "Catholic Family Service",
    "statement_type": "factual_assertion",
    "attribution": "own_determination",
    "exhibit_refs": "Exhibit 1",
    "page_number": 4,
    "page_note": "pages 4-5",
    "kind": "documentary",
    "evidence_strength": "attorney_assertion",
    "significance": "The conflict-of-interest claim, said to rest on a contract bound into the appendix. Counsel's account of what that contract shows.",
    "weight": 5,
    "pattern_tags": "judicial_bias, concealment",
    "event_date": "2012-11-16"
  },
  "verbatim_quote": "Appellant has now discovered documentation which reveals a conflict of interest between CFS and Judge Tighe that should have been thoroughly disclosed to the interested parties prior to CFS' appointment as personal representative at the time the estate was opened."
}
```

### Example entity (Evidence — a quoted characterization from the opinion under review, recited):
```json
{
  "entity_type": "Evidence",
  "id": "evidence-019",
  "label": "Trial judge's 'roadblocks' characterization quoted (recited)",
  "properties": {
    "title": "Trial judge's 'roadblocks' characterization quoted (recited, not adopted)",
    "summary": "The appellant reproduces the probate judge's characterization of her conduct as presenting roadblocks to settling the estate.",
    "filed_by": "Marie Awad",
    "appellate_role": "appellant",
    "asserted_against": "Marie Awad",
    "statement_type": "attorney_argument",
    "attribution": "recitation",
    "exhibit_refs": "Opinion and Order, Page 10, April 12, 2012",
    "page_number": 5,
    "page_note": "pages 5-6",
    "kind": "documentary",
    "evidence_strength": "recited_position",
    "significance": "The PROBATE JUDGE'S characterization, reproduced to be attacked — not the appellant's own assertion. It belongs to the April 12, 2012 Opinion and Order. Pass 2 must create no finding-edge from it.",
    "weight": 2,
    "event_date": "2012-04-12"
  },
  "verbatim_quote": "...roadblocks to settling this estate, far out of proportion to the amounts in controversy."
}
```

Note that all three examples omit the properties that do not apply — no empty `legal_basis`, no empty `relief_sought`, no `pattern_tags` on the recitation, no `asserted_against` where an assertion targets no one.

Return ONLY the JSON object. No markdown fences, no explanation, no preamble.

## Completeness Checklist — Verify Before Returning

**Scope checks (do these FIRST):**
- [ ] Did I identify the sub-genre from the title line before extracting?
- [ ] Did I locate the end of the brief proper before extracting?
- [ ] Did I extract ZERO entities from pages after the first exhibit separator sheet?
- [ ] Did I avoid extracting from any bound hearing transcript, affidavit, contract, billing statement, or other party's filing?
- [ ] If the document has no exhibit separator at all, did I treat the whole document as brief matter rather than assuming I had missed a boundary?
- [ ] Did I use the structural signal (near-empty page reading `EXHIBIT` + numeral) rather than OCR confidence to find the boundary?

**Party checks:**
- [ ] Did I identify the filing party and `appellate_role` from the TITLE LINE and SIGNATURE BLOCK, not from caption position?
- [ ] Did I resolve counsel-to-party pairings from the signature block rather than from the interleaved caption columns?
- [ ] Did I extract the appellant, the appellee, every attorney of record, and the judge whose ruling is under review?
- [ ] Did I create NO Party entities for service affiants, notaries, or clerical signatories on a proof of service?
- [ ] Did I create NO Party entities for judges or litigants named only inside cited case authorities?
- [ ] Do the aliases include appellate positions ("Appellant", "the Appellee") and parenthetical short forms (`("CFS")`, `("Marie")`)?

**Evidence checks:**
- [ ] Does every Evidence entity carry the SAME `filed_by` and the SAME `appellate_role`?
- [ ] Did I create a separate entity for each discrete assertion — each argument heading, each question, each distinct claim?
- [ ] Does every Evidence entity have verbatim_quote at the TOP LEVEL (not inside properties)?
- [ ] Did I set `statement_type` and `attribution` independently on every entity?
- [ ] Did I derive `evidence_strength` from the two axes using the table?
- [ ] Is every `page_number` the PDF page from a `--- Page N ---` marker, never the printed folio?
- [ ] Did I carry the filing date as `event_date`, except where an assertion references a different dated event?
- [ ] Did I record all four citation forms in `exhibit_refs`, including record citations by title and date that carry no exhibit number?

**Negative checks:**
- [ ] Did I tag as `recitation` every quotation of the opposing brief, the opinion under review, a bench ruling, and the record?
- [ ] Where the brief quoted something AND argued about it, did I extract two entities with different attributions?
- [ ] Did I create NO Evidence entity for the repeated Standard of Review blocks — all of them, however many times the same text appeared?
- [ ] Did I create NO Evidence entities from the Table of Contents, the Table/Index of Authorities, the cover letter, or the proof of service?
- [ ] Did I create NO separate entities for the "Appellant answers / Appellee answers / Probate Court answered" lines?
- [ ] For a year or an amount followed by extra digits (`200929`, `$50,000.004`), did I take the true value and treat the trailing digits as a footnote marker?
- [ ] Did I keep fused markers inside `verbatim_quote` exactly as printed, while keeping injected letterhead and injected citation lines OUT of it?
- [ ] Where text was visibly truncated mid-sentence, did I decline to reconstruct it?
- [ ] Did I record dates as THIS document states them, without reconciling any date against another document?
- [ ] Did I keep `weight` low for advocacy, rather than raising it because an assertion seemed important or was heavily cited?
- [ ] Did I use ONLY the ten closed pattern_tags, and omit the property entirely where none applied?
- [ ] Did I omit inapplicable optional properties entirely rather than emitting empty strings?
- [ ] Did I create any relationships? (I should NOT have — relationships come in Pass 2.)

Return ONLY the JSON object with an "entities" array. No "relationships" key. No markdown fences, no explanation, no preamble.
