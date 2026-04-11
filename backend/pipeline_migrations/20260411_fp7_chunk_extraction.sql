-- FP-7: Per-chunk extraction tracking and pipeline fix.
--
-- 1. New table: extraction_chunks — stores per-chunk results for
--    the chunk-based extraction pipeline.
-- 2. New columns on extraction_runs — chunk-level statistics.
-- 3. Bug fix: extraction_chunks table enables per-chunk observability.

-- Per-chunk extraction results
CREATE TABLE IF NOT EXISTS extraction_chunks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    extraction_run_id INTEGER NOT NULL REFERENCES extraction_runs(id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    chunk_text TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    node_count INTEGER NOT NULL DEFAULT 0,
    relationship_count INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    input_tokens INTEGER,
    output_tokens INTEGER,
    duration_ms INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for querying chunks by run
CREATE INDEX IF NOT EXISTS idx_extraction_chunks_run
    ON extraction_chunks(extraction_run_id);

-- Chunk-level statistics on extraction_runs
ALTER TABLE extraction_runs ADD COLUMN IF NOT EXISTS chunk_count INTEGER;
ALTER TABLE extraction_runs ADD COLUMN IF NOT EXISTS chunks_succeeded INTEGER;
ALTER TABLE extraction_runs ADD COLUMN IF NOT EXISTS chunks_failed INTEGER;
ALTER TABLE extraction_runs ADD COLUMN IF NOT EXISTS chunks_pruned_nodes INTEGER;
ALTER TABLE extraction_runs ADD COLUMN IF NOT EXISTS chunks_pruned_relationships INTEGER;
