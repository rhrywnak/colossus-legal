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

## Required output format

Return a JSON object with exactly two arrays: "entities" and "relationships".

Each entity MUST have these fields:
- "id": unique string identifier (e.g., "0", "1", "2")
- "entity_type": the entity type name from the schema (e.g., "Party", "LegalCount")
- "properties": an object containing the extracted properties
- "verbatim_quote": the exact text from the document that supports this entity (if applicable, otherwise omit)

Example entity:
{"id": "0", "entity_type": "Party", "properties": {"party_name": "John Smith", "role": "plaintiff"}, "verbatim_quote": "Plaintiff John Smith filed..."}

Each relationship MUST have these fields:
- "relationship_type": the relationship type from the schema (e.g., "FILED_BY", "SUPPORTS")
- "from_entity": the id of the source entity
- "to_entity": the id of the target entity
- "properties": an object (can be empty {})

Example relationship:
{"relationship_type": "FILED_BY", "from_entity": "0", "to_entity": "1", "properties": {}}

Return ONLY the JSON object. No explanation, no markdown fences, no commentary.

## Text to analyze

{{chunk_text}}
