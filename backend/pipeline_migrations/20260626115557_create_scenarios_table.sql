-- create_scenarios_table: Create scenarios table
--
-- Created: 2026-06-26 11:55:57
-- Target: pipeline database (colossus_legal_v2)
--
-- A scenario is a saved LENS over the case graph (task 1.1). This table holds
-- ONLY its authored definition + the spine columns the system filters/joins on.
-- It holds NO case content (no quotes, citations, or facts) — those live in the
-- graph and are referenced by scenario_fact_refs (task 1.2), not here.
--
-- Spine columns are real SQL columns (queried/filtered). The churning authored
-- body lives in the `definition` jsonb so it can evolve without a migration; its
-- internal shape is validated at render time (task 1.5), never by this table.
--
-- Mirrors the spine+jsonb pattern of authored_entities
-- (20260526141630_create_authored_entity_tables.sql). No FK to extraction or
-- graph tables — string ids only, same no-FK discipline.

CREATE TABLE scenarios (
    scenario_id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name                   TEXT NOT NULL,
    direction              TEXT NOT NULL,
    status                 TEXT NOT NULL DEFAULT 'draft',
    case_slug              TEXT NOT NULL,
    feeds_count_id         TEXT,
    anchor_allegation_ids  TEXT[],
    definition             JSONB NOT NULL,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- CHECK (not a Postgres enum) so the value set is widened with an ALTER,
    -- not an enum migration — the same "no DB enum" discipline used elsewhere.
    CONSTRAINT scenarios_direction_check
        CHECK (direction IN ('offense', 'defense')),
    CONSTRAINT scenarios_status_check
        CHECK (status IN ('draft', 'needs_evidence', 'ready'))
);

CREATE INDEX idx_scenarios_case ON scenarios (case_slug);

COMMENT ON TABLE scenarios IS
    'Authored scenario definitions — a saved lens over the case graph. '
    'Spine columns are filtered/joined on; definition jsonb is the authored body. '
    'Holds NO case content (no quotes, citations, or facts) — those live in the '
    'graph and are referenced by scenario_fact_refs (task 1.2), not here.';

COMMENT ON COLUMN scenarios.direction IS
    'offense | defense. CHECK-constrained (not a Postgres enum) so the set is '
    'alterable without an enum migration.';

COMMENT ON COLUMN scenarios.status IS
    'draft | needs_evidence | ready. CHECK-constrained; defaults to draft.';

COMMENT ON COLUMN scenarios.case_slug IS
    'Case identifier (e.g. awad_v_catholic_family_service). Scopes scenarios to a case.';

COMMENT ON COLUMN scenarios.feeds_count_id IS
    'Optional LegalCount this scenario feeds (e.g. count-1). String id, no FK.';

COMMENT ON COLUMN scenarios.anchor_allegation_ids IS
    'Optional denormalized convenience array of allegation entity_ids this scenario '
    'anchors on. The authoritative anchors are scenario_fact_refs rows (task 1.2); '
    'this is a spine-level hint only, NOT the source of truth.';

COMMENT ON COLUMN scenarios.definition IS
    'Authored body as JSONB (attack_text, attack_meaning, wielders[], target, '
    'seed_phrases[], anti_seed_phrases[], notes, schema_v). Internal shape is NOT '
    'modeled as columns — it churns and is validated at render time (task 1.5).';
