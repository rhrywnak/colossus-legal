-- Pipeline Simplification: new columns for process endpoint
-- Adds progress tracking, error details, and cancellation support

-- Progress tracking columns
ALTER TABLE documents ADD COLUMN IF NOT EXISTS processing_step TEXT;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS processing_step_label TEXT;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS chunks_total INTEGER DEFAULT 0;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS chunks_processed INTEGER DEFAULT 0;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS entities_found INTEGER DEFAULT 0;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS percent_complete INTEGER DEFAULT 0;

-- Error detail columns
ALTER TABLE documents ADD COLUMN IF NOT EXISTS failed_step TEXT;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS failed_chunk INTEGER;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS error_message TEXT;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS error_suggestion TEXT;

-- Cancellation
ALTER TABLE documents ADD COLUMN IF NOT EXISTS is_cancelled BOOLEAN NOT NULL DEFAULT FALSE;

-- Auto-write tracking
ALTER TABLE documents ADD COLUMN IF NOT EXISTS entities_written INTEGER DEFAULT 0;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS entities_flagged INTEGER DEFAULT 0;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS relationships_written INTEGER DEFAULT 0;

-- Graph status for extraction items (grounded → written, ungrounded → flagged)
ALTER TABLE extraction_items ADD COLUMN IF NOT EXISTS graph_status TEXT DEFAULT 'pending';
-- Values: 'pending' (not yet processed), 'written' (in Neo4j), 'flagged' (ungrounded, skipped)
