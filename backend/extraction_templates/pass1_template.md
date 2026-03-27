# Document extraction — Pass 1 (exhaustive)

You are extracting structured information from a legal document. Your goal is to identify every entity and relationship described in the document, with exact verbatim quotes from the text.

## Schema — what to extract

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

Return a single JSON object with two arrays: "entities" and "relationships".

Each entity must have:
- "entity_type": matching a type from the schema above
- "id": a unique identifier (e.g. "party-001", "allegation-001")
- "label": a short human-readable label
- "properties": an object with the properties defined in the schema
- "verbatim_quote": exact text from the document (where applicable)

Each relationship must have:
- "relationship_type": matching a type from the schema above
- "from_entity": the id of the source entity
- "to_entity": the id of the target entity

Return ONLY the JSON object. No markdown fences, no explanation, no preamble.
