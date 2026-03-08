-- Ratings live in PostgreSQL, not Neo4j.
-- Neo4j stores graph facts. PostgreSQL stores analytical/feedback data.

CREATE TABLE IF NOT EXISTS qa_ratings (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    qa_id       TEXT NOT NULL,
    rated_by    TEXT NOT NULL,
    rating      SMALLINT NOT NULL CHECK (rating >= 1 AND rating <= 5),
    model       TEXT NOT NULL,
    scope_type  TEXT NOT NULL,
    scope_id    TEXT NOT NULL,
    rated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_qa_ratings_qa_id_rated_by UNIQUE (qa_id, rated_by)
);

CREATE INDEX IF NOT EXISTS idx_qa_ratings_qa_id    ON qa_ratings(qa_id);
CREATE INDEX IF NOT EXISTS idx_qa_ratings_rated_by ON qa_ratings(rated_by);
CREATE INDEX IF NOT EXISTS idx_qa_ratings_model    ON qa_ratings(model);
CREATE INDEX IF NOT EXISTS idx_qa_ratings_rated_at ON qa_ratings(rated_at);
