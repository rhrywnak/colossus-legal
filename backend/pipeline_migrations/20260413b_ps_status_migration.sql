-- Pipeline Simplification: migrate old statuses to new 5-status model.
-- This is a one-way migration. Old statuses are replaced.

-- UPLOADED and TEXT_EXTRACTED → NEW (uploaded, text available, not yet processed)
UPDATE documents SET status = 'NEW' WHERE status IN ('UPLOADED', 'TEXT_EXTRACTED');

-- EXTRACTION_FAILED → FAILED
UPDATE documents SET status = 'FAILED' WHERE status = 'EXTRACTION_FAILED';

-- EXTRACTED with 0 items → FAILED (extraction ran but produced nothing)
UPDATE documents SET status = 'FAILED',
    error_message = 'Previous extraction produced 0 entities',
    error_suggestion = 'Try re-processing with different settings'
WHERE status = 'EXTRACTED'
    AND id NOT IN (SELECT DISTINCT document_id FROM extraction_items);

-- EXTRACTED with items > 0 → COMPLETED (extraction worked, just wasn't reviewed/ingested)
UPDATE documents SET status = 'COMPLETED'
WHERE status = 'EXTRACTED'
    AND id IN (SELECT DISTINCT document_id FROM extraction_items);

-- VERIFIED, REVIEWED → COMPLETED
UPDATE documents SET status = 'COMPLETED' WHERE status IN ('VERIFIED', 'REVIEWED');

-- INGESTED, INDEXED → COMPLETED
UPDATE documents SET status = 'COMPLETED' WHERE status IN ('INGESTED', 'INDEXED');

-- PUBLISHED → COMPLETED
UPDATE documents SET status = 'COMPLETED' WHERE status = 'PUBLISHED';
