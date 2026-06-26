-- create_scenario_responses_tables: Create scenario responses tables
--
-- Created: 2026-06-26 13:50:22
-- Target: pipeline database (colossus_legal_v2)
--
-- The scenario RESPONSES model (task 1.6) — three FK-chained tables that complete
-- the Phase-1 minimal slice (1.1 scenarios + 1.2 scenario_fact_refs + 0.2 graph
-- human-fact write path + this):
--
--   scenario_responses        — Marie's prepared answer(s) to a scenario.
--   response_items            — a response may itemize (per-attorney, per-instance);
--                               ordered text rows under a response.
--   response_item_fact_refs   — m:n link from a response item to the GRAPH node ids
--                               of the evidence it rests on.
--
-- Same tag-not-copy discipline as scenario_fact_refs: facts are referenced by graph
-- node id and read LIVE from the graph at compose time — quotes/citations/fact text
-- are NEVER copied into these tables. The referenced facts are human-authored graph
-- nodes (0.2 path) and/or system-extracted nodes — both by id, neither copied.
--
-- Ownership cascade runs the WHOLE chain back to 1.1's table: deleting a scenario
-- wipes its responses, their items, and those items' fact refs. (Owned-child
-- cascade precedent: pipeline_events, extraction_chunks, scenario_fact_refs.)
-- Parent declared first so each FK target exists when its child is created.

-- ───────────────────────────────────────────────────────────────
-- scenario_responses — a prepared answer to a scenario
-- ───────────────────────────────────────────────────────────────
CREATE TABLE scenario_responses (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scenario_id  UUID NOT NULL REFERENCES scenarios(scenario_id) ON DELETE CASCADE,
    label        TEXT,
    text         TEXT NOT NULL,
    -- CHECK (not a Postgres enum): the scenario family's status/origin vocabularies
    -- are small, stable, and system-owned, so internal consistency with 1.1's
    -- direction/status CHECKs matters more than evolvability here.
    status       TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'ready')),
    origin       TEXT NOT NULL DEFAULT 'human'  CHECK (origin IN ('human', 'suggested')),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_scenario_responses_scenario ON scenario_responses (scenario_id);

COMMENT ON TABLE scenario_responses IS
    'A prepared answer to a scenario (task 1.6). Holds the response text and its '
    'status/origin — NO case content (no quotes, citations, or fact text). The '
    'evidence a response rests on is referenced by graph node id on its items '
    '(response_item_fact_refs), read live from the graph at compose time.';

COMMENT ON COLUMN scenario_responses.status IS
    'draft | ready. CHECK-constrained (small, system-owned vocabulary), mirroring '
    'the 1.1 scenarios.status pattern.';

COMMENT ON COLUMN scenario_responses.origin IS
    'human (Marie authored it) | suggested (system-proposed, pending adoption). '
    'App-state response provenance — a different axis from graph-node provenance.';

-- ───────────────────────────────────────────────────────────────
-- response_items — ordered itemization under a response
-- ───────────────────────────────────────────────────────────────
CREATE TABLE response_items (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    response_id  UUID NOT NULL REFERENCES scenario_responses(id) ON DELETE CASCADE,
    item_index   INTEGER NOT NULL,
    text         TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_response_items_response ON response_items (response_id, item_index);

COMMENT ON TABLE response_items IS
    'Ordered text rows under a scenario_response (per-attorney, per-instance '
    'itemization). Holds NO case content — evidence is referenced by graph node id '
    'in response_item_fact_refs.';

COMMENT ON COLUMN response_items.item_index IS
    'Zero-based order of this item within its response (mirrors '
    'extraction_chunks.chunk_index). Not the reserved word "order".';

-- ───────────────────────────────────────────────────────────────
-- response_item_fact_refs — m:n link: item ↔ graph node ids of its evidence
-- ───────────────────────────────────────────────────────────────
CREATE TABLE response_item_fact_refs (
    response_item_id  UUID NOT NULL REFERENCES response_items(id) ON DELETE CASCADE,
    graph_node_id     TEXT NOT NULL,
    note              TEXT,
    tagged_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (response_item_id, graph_node_id)
);

COMMENT ON TABLE response_item_fact_refs IS
    'Many-to-many link from a response item to the graph node ids of the evidence it '
    'rests on. Tag-not-copy: the graph_node_id is a pointer into Neo4j; the fact''s '
    'quote/citation are read live from the graph at compose time, NEVER stored here. '
    'The same graph node may back items across many responses. A fact''s cross-Count '
    'reach lives on its graph edges, never in this table. Holds NO case content.';

COMMENT ON COLUMN response_item_fact_refs.graph_node_id IS
    'The Neo4j node id (the graph node''s id property). Plain TEXT, NO foreign key — '
    'it points into Neo4j, which Postgres cannot reference. May name a human-authored '
    'node (0.2 path) or a system-extracted node — both by id, neither copied.';

COMMENT ON COLUMN response_item_fact_refs.note IS
    'Optional free-text note about why this evidence backs the item. No case content.';
