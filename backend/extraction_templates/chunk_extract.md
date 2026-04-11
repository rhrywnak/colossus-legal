You are an expert at extracting structured information from text.

Extract all entities and relationships from the text below.
Use ONLY the entity types and relationship types defined in the schema.
For each entity, include all relevant properties defined in the schema.

## Schema

{{schema_json}}

## Rules

- Assign a unique ID string to each entity (e.g., "0", "1", "2").
- Reuse IDs when defining relationships between entities.
- Respect the allowed relationship patterns in the schema.
- Extract EVERY entity you can identify, even if similar entities may exist elsewhere.
- If a property value is not clearly stated in the text, omit it.
- For entities with a verbatim_quote property, copy the EXACT text from the document — do not paraphrase.
- Do not invent information that is not in the text.

{{examples}}

## Text to analyze

{{chunk_text}}
