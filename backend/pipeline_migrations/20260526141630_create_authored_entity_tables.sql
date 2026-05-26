-- Three-tier architecture: authored entity storage (Option A)
--
-- Tier 1 (authored): Elements, LegalCounts, and future human-created entities.
--   These are structural legal knowledge that humans bring to the system.
--   They are NOT extracted from documents by the pipeline.
--
-- Tier 3 (mapping): Relationships connecting authored entities (Tier 1) to
--   extracted entities (Tier 2) and to each other. Independently editable,
--   rebuildable without reprocessing documents.
--
-- Design decision: COLOSSUS_LEGAL_SESSION_TRANSITION_2026-05-25.md §8
-- No FK to extraction_items, extraction_runs, or documents.
-- entity_id is the stable string ID used as the Neo4j node 'id' property.

-- ============================================================
-- Table: authored_entities
-- ============================================================
-- Stores human-authored entities (Elements from Michigan law,
-- LegalCounts from the complaint structure, future authored types).
-- The canonical Element loader writes here; the ingest step reads
-- here and writes to Neo4j.

CREATE TABLE authored_entities (
    id              SERIAL PRIMARY KEY,
    case_slug       TEXT NOT NULL,
    entity_type     TEXT NOT NULL,
    entity_id       TEXT NOT NULL,
    item_data       JSONB NOT NULL,
    provenance      TEXT NOT NULL DEFAULT 'authored',
    created_by      TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT authored_entities_entity_id_unique UNIQUE (entity_id)
);

CREATE INDEX idx_authored_entities_case ON authored_entities (case_slug);
CREATE INDEX idx_authored_entities_type ON authored_entities (entity_type);

COMMENT ON TABLE authored_entities IS
    'Tier 1: human-authored entities (Elements, LegalCounts). '
    'No FK to extraction pipeline tables. entity_id is the Neo4j node id.';

COMMENT ON COLUMN authored_entities.case_slug IS
    'Case identifier (e.g. awad_v_catholic_family_service). Scopes entities to a case.';

COMMENT ON COLUMN authored_entities.entity_id IS
    'Stable string ID used as Neo4j node id property. Must be globally unique. '
    'For Elements: the YAML id (e.g. element-1-1). '
    'For LegalCounts: e.g. count-1, count-2.';

COMMENT ON COLUMN authored_entities.item_data IS
    'Full entity data as JSONB. Structure depends on entity_type. '
    'For Elements: {element_name, title, what_plaintiff_must_prove, '
    'controlling_authority, statutory_anchor, case_specific_notes, '
    'order_in_count, parent_count_number}. '
    'For LegalCounts: {count_number, count_name, burden_of_proof, '
    'controlling_authorities, m_civ_ji_reference, paragraph_range}.';

COMMENT ON COLUMN authored_entities.provenance IS
    'Origin: authored (human-created), canonical (from law library), '
    'or future values. Never extracted.';

-- ============================================================
-- Table: authored_relationships
-- ============================================================
-- Stores relationships where at least one endpoint is an authored
-- entity, OR relationships that are independently editable (Tier 3
-- mapping layer). Endpoints are referenced by entity_id strings,
-- not integer FKs — an endpoint can be in authored_entities OR
-- extraction_items.

CREATE TABLE authored_relationships (
    id                  SERIAL PRIMARY KEY,
    case_slug           TEXT NOT NULL,
    from_entity_id      TEXT NOT NULL,
    to_entity_id        TEXT NOT NULL,
    relationship_type   TEXT NOT NULL,
    properties          JSONB,
    provenance          TEXT NOT NULL DEFAULT 'authored',
    created_by          TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT authored_relationships_unique_edge
        UNIQUE (from_entity_id, to_entity_id, relationship_type)
);

CREATE INDEX idx_authored_relationships_case ON authored_relationships (case_slug);
CREATE INDEX idx_authored_relationships_type ON authored_relationships (relationship_type);
CREATE INDEX idx_authored_relationships_from ON authored_relationships (from_entity_id);
CREATE INDEX idx_authored_relationships_to ON authored_relationships (to_entity_id);

COMMENT ON TABLE authored_relationships IS
    'Tier 3: mapping layer. Relationships connecting authored entities '
    '(Tier 1) to extracted entities (Tier 2) and to each other. '
    'No FK to extraction_items. Endpoints referenced by entity_id strings.';

COMMENT ON COLUMN authored_relationships.from_entity_id IS
    'Source entity stable string ID. Can reference an authored_entities.entity_id '
    'OR the neo4j_node_id of an extraction_items row (the stable, content-derived '
    'id assigned at ingest and used as the Neo4j node id) — for cross-tier edges '
    'like PROVES_ELEMENT from an Allegation to an Element.';

COMMENT ON COLUMN authored_relationships.to_entity_id IS
    'Target entity stable string ID. Same cross-tier rules as from_entity_id.';

COMMENT ON COLUMN authored_relationships.provenance IS
    'Origin: authored (human-created via UI), canonical (from loader), '
    'mapped (from Element mapping step), or future values.';
