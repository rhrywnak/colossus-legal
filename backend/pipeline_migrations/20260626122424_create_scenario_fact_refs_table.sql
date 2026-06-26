-- create_scenario_fact_refs_table: Create scenario_fact_refs table
--
-- Created: 2026-06-26 12:24:25
-- Target: pipeline database (colossus_legal_v2)
--
-- The THIN reference table linking a scenario (scenarios, task 1.1) to graph
-- node ids, recording the ROLE each referenced fact plays in THAT scenario. It
-- holds NO case content (no quotes, speakers, citations, or fact text) — facts
-- are read live from the graph by graph_node_id at compose time.
--
-- The same graph_node_id may appear under different scenario_ids with different
-- roles: role is a property of the REFERENCE, not of the fact. That cross-scenario
-- sharing with per-scenario roles is the whole point — a fact is shared, never owned.
--
-- Composite PK (scenario_id, graph_node_id): a fact appears once per scenario.
-- Mirrors document_text's composite-PK shape (NOT authored_relationships'
-- surrogate id + UNIQUE constraint). scenario_id is a real Postgres FK with
-- ON DELETE CASCADE (a ref is owned by its scenario — same as
-- extraction_chunks -> extraction_runs); graph_node_id is plain TEXT with NO FK
-- (it points into Neo4j, which Postgres cannot reference).

CREATE TABLE scenario_fact_refs (
    scenario_id            UUID NOT NULL REFERENCES scenarios(scenario_id) ON DELETE CASCADE,
    graph_node_id          TEXT NOT NULL,
    role_in_this_scenario  TEXT,
    confirmed              BOOLEAN NOT NULL DEFAULT FALSE,
    note                   TEXT,
    tagged_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (scenario_id, graph_node_id)
);

COMMENT ON TABLE scenario_fact_refs IS
    'Thin reference table: links a scenario to graph node ids and records the ROLE '
    'each referenced fact plays in THAT scenario. Holds NO case content (no quotes, '
    'speakers, citations, or fact text) — facts are read live from the graph by '
    'graph_node_id at compose time. The same graph_node_id may appear under '
    'different scenario_ids with different roles: role is a property of the '
    'REFERENCE, not of the fact. A fact is shared across scenarios, never owned.';

COMMENT ON COLUMN scenario_fact_refs.graph_node_id IS
    'The Neo4j node id (the graph node''s id property). Plain TEXT, NO foreign key — '
    'it points into Neo4j, which Postgres cannot reference.';

COMMENT ON COLUMN scenario_fact_refs.role_in_this_scenario IS
    'The role this fact plays in this scenario. Nullable until assigned. The role '
    'vocabulary is owned by the task-1.3 code lookup (deliberately evolvable) — '
    'intentionally NOT a DB CHECK or enum, so widening the set needs no migration. '
    'Validated in code when 1.3 lands.';

COMMENT ON COLUMN scenario_fact_refs.confirmed IS
    'Whether a human has confirmed this fact belongs in this scenario (default false).';

COMMENT ON COLUMN scenario_fact_refs.note IS
    'Optional free-text note about why this fact is referenced here. No case content.';
