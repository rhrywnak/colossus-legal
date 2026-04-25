# Global Extraction Rules — DATA_MODEL_v4

These rules apply to ALL document types and both extraction passes.

## Verbatim quote rules
- Every verbatim_quote MUST be an exact substring of the document text
- Do NOT paraphrase, summarize, or clean up the text
- Include surrounding context if the quote is ambiguous out of context
- If the text contains OCR artifacts, copy them exactly

## Page number rules
- page_number is REQUIRED on every Evidence node
- Use the page number as printed on the document, not the PDF page index
- If no page numbers are printed, use the PDF page index starting from 1

## Entity ID rules
- Use descriptive IDs: party-phillips, evidence-phillips-q73, allegation-005, count-001, harm-001
- For Evidence from discovery: evidence-{respondent}-q{number}
- For Evidence from affidavits: evidence-{affiant}-{topic}
- For Evidence from briefs: evidence-{author}-coa-{topic}
- For Evidence from rulings: evidence-{court}-{topic}
- IDs must be unique within the document extraction

## Party extraction rules
- Extract EVERY named individual and organization
- Use full legal names as they appear in the document
- Set party_type to "person" or "organization"
- Include role in the case (plaintiff, defendant, attorney, judge, etc.)
- The same person may have multiple roles — use the most specific role

## Relationship rules
- Every Evidence node MUST have exactly one STATED_BY relationship
- Every Evidence node MUST have at least one ABOUT relationship when applicable
- Every Evidence node MUST have one CONTAINED_IN relationship (created by the pipeline, not extracted)
- Do NOT create relationships to entity types not defined in the schema
- Use entity IDs from the provided entity list, not invented IDs

## What NOT to extract
- Do NOT extract purely procedural content (e.g., "This matter comes before the Court...")
- Do NOT extract boilerplate legal language unless it contains case-specific content
- Do NOT extract caption blocks, signature lines, or certificate of service
- Do NOT create entities for concepts — only for specific factual content with quotes

## Quality standards
- Prefer MORE entities over fewer — litigation requires exhaustive extraction
- When in doubt about whether something is substantive, extract it
- Set weight higher for admissions by opposing parties than for routine procedural facts
- Flag potential bias indicators with appropriate pattern_tags
