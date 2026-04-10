# Complaint Extraction — Exhaustive Structural Analysis

You are extracting structured information from a **legal complaint** — the document that initiates a civil lawsuit. Your job is to identify every party, every cause of action (legal count), every factual allegation, and every claimed harm, with precise references to the document text.

## Why this matters

The complaint defines the structural skeleton of the entire case. Every other document in this case (discovery responses, affidavits, motions, court rulings) will be analyzed in relation to what you extract here. Missing a party, a count, or an allegation means that evidence linking to it will be orphaned. Completeness is critical.

## Extraction strategy

Work through the complaint systematically in this order:

### Step 1: Identify ALL parties
Read the opening paragraphs and caption. Extract every person and organization named as a plaintiff, defendant, or third party. Use their full legal name exactly as it appears in the document. Determine whether each is a person or an organization.

### Step 2: Identify ALL legal counts (causes of action)
Scan the complaint for section headings that identify causes of action. These are typically labeled "COUNT I", "COUNT II", "FIRST CAUSE OF ACTION", or similar. Extract every count with its number and legal basis. Note the paragraph range for each count.

### Step 3: Extract EVERY factual allegation
Go through the complaint paragraph by paragraph. Each numbered paragraph that makes a factual claim is a separate ComplaintAllegation. Do not skip any. Do not group multiple paragraphs together. For each allegation:
- Record the paragraph number
- Write a one-sentence summary
- Copy the verbatim text exactly as it appears in the document
- Determine which legal count(s) this allegation supports
- Determine which party this allegation is about

### Step 4: Identify claimed harms
Look for descriptions of damages, injuries, or losses. These may appear in a separate damages section or scattered throughout the allegations. For each harm:
- Classify the type (financial, emotional, procedural, reputational, physical)
- Note any specific dollar amounts
- List the paragraph numbers that establish this harm (provenance)

## Schema — entity types, properties, and relationships

{{schema_json}}

## Extraction rules

{{global_rules}}

## Additional instructions from administrator

{{admin_instructions}}

## Prior context from other documents

{{context}}

## Document text

{{document_text}}

## Output format

Return a single JSON object with two top-level arrays: `"entities"` and `"relationships"`.

### Entity format

Each entity must have these fields:
- "entity_type": matching a type from the schema (Party, LegalCount, ComplaintAllegation, or Harm)
- "id": unique identifier (party-001, count-001, allegation-001, harm-001)
- "label": short human-readable label
- "properties": object with properties defined in the schema
- "verbatim_quote": exact text from the document (REQUIRED for ComplaintAllegation, null for others)

For Harm entities, include a "provenance" array:
```json
"provenance": [
  {"ref_type": "paragraph", "ref": "47", "quote_snippet": "failed to account for..."},
  {"ref_type": "paragraph", "ref": "52", "quote_snippet": "estate funds were..."}
]
```

### Relationship format

Each relationship must reference entity IDs:
```json
{"relationship_type": "SUPPORTS", "from_entity": "allegation-012", "to_entity": "count-001"}
```

### Completeness checklist

Before returning your output, verify:
- Did I extract EVERY party named in the complaint?
- Did I extract EVERY legal count (cause of action)?
- Did I extract EVERY numbered paragraph as a separate allegation?
- Does every allegation have an exact verbatim_quote from the document?
- Does every allegation link to at least one legal count via SUPPORTS?
- Does every allegation identify which party it is ABOUT?
- Does every Harm have a provenance array linking to supporting paragraphs?

Return ONLY the JSON object. No markdown fences, no explanation, no preamble.
