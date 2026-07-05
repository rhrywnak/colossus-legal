-- refresh_scenarios_definition_column_comment: refresh scenarios.definition column comment
--
-- Created: 2026-07-05 15:35:20
-- Target: pipeline database (colossus_legal_v2)
--
-- Documentation-only. The COMMENT ON COLUMN for scenarios.definition set by
-- 20260626115557_create_scenarios_table.sql described the ORIGINAL v1 definition
-- shape (8 keys incl. the now-retired seed_phrases[], anti_seed_phrases[], notes).
-- D1 rebuilt the definition to schema_v 2; this refreshes ONLY that column comment
-- to match. No column, constraint, index, or data is changed.

COMMENT ON COLUMN scenarios.definition IS
    'Authored body as JSONB, schema_v 2 (attack_text, attack_meaning, target, '
    'wielders[], schema_v). target is a party node id from the subjects vocab, '
    'not free text; wielders[] is a list of {party_id, actor_role} objects, not '
    'strings. Retired in D1 (schema_v 1 -> 2): seed_phrases[], anti_seed_phrases[], '
    'notes. Internal shape is NOT modeled as columns; it is validated at the serde '
    'parse boundary (ScenarioDefinition DTO — loud Err on {} / v1 / unknown '
    'actor_role), not at render time.';
