-- WP-1: Consolidate QA entries + ratings into PostgreSQL.
-- Previously: QAEntry nodes in Neo4j, ratings in qa_ratings table.
-- Now: Single qa_entries table with rating columns inline.

CREATE TABLE IF NOT EXISTS qa_entries (
    id          UUID PRIMARY KEY,
    scope_type  VARCHAR(50) NOT NULL,
    scope_id    VARCHAR(100) NOT NULL,
    session_id  VARCHAR(100),
    question    TEXT NOT NULL,
    answer      TEXT NOT NULL,
    asked_by    VARCHAR(100) NOT NULL,
    asked_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    model       VARCHAR(100) NOT NULL,
    parent_qa_id UUID REFERENCES qa_entries(id) ON DELETE SET NULL,
    metadata    JSONB,
    rating      SMALLINT CHECK (rating BETWEEN 1 AND 5),
    rated_by    VARCHAR(100),
    rated_at    TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_qa_entries_scope
    ON qa_entries(scope_type, scope_id, asked_at DESC);
CREATE INDEX IF NOT EXISTS idx_qa_entries_asked_by
    ON qa_entries(asked_by);
CREATE INDEX IF NOT EXISTS idx_qa_entries_metadata
    ON qa_entries USING GIN (metadata);
