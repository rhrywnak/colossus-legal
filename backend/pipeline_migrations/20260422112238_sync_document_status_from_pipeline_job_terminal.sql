-- sync_document_status_from_pipeline_job_terminal: Sync document status from pipeline job terminal
--
-- Created: 2026-04-22 11:22:38
-- Target: pipeline database
--
-- Problem
-- -------
-- When a pipeline_jobs row reaches a terminal state (failed/cancelled/completed),
-- nothing updates the corresponding documents.status. The steps write progress
-- statuses (INGESTED, INDEXED, PUBLISHED) only on the happy path; the colossus-
-- pipeline StepRecorder trait has no job-terminal hook. Result: failed jobs leave
-- documents stuck at PROCESSING, the state_machine only exposes Cancel, and the
-- Cancel route is itself missing — Marie & Chuck cannot recover through the UI.
--
-- Fix
-- ---
-- AFTER UPDATE trigger on pipeline_jobs. When job_type = 'document_processing'
-- and status transitions to a terminal value, project that onto documents.status
-- using the status names the 5-state UI already expects (FAILED/CANCELLED) plus
-- PUBLISHED for completed (safety net behind the step-layer writes; the 8→5-state
-- PS-B8 migration is Phase 5b scope).
--
-- Terminal mapping
-- ----------------
--   pipeline_jobs.status  ->  documents.status
--   failed                ->  'FAILED'
--   cancelled             ->  'CANCELLED'
--   completed             ->  'PUBLISHED'   (safety net; step writes STATUS_PUBLISHED first)
--
-- Guard: only fires on the status-change edge (OLD.status IS DISTINCT FROM
-- NEW.status) so mid-flight updates that don't touch status are ignored, and
-- so an idempotent re-insert of the same terminal state is a no-op.

CREATE OR REPLACE FUNCTION sync_document_status_from_pipeline_job() RETURNS trigger AS $$
BEGIN
    IF NEW.job_type = 'document_processing'
       AND NEW.status IN ('failed', 'cancelled', 'completed')
       AND (OLD.status IS DISTINCT FROM NEW.status) THEN
        UPDATE documents
        SET status = CASE NEW.status
                         WHEN 'failed'    THEN 'FAILED'
                         WHEN 'cancelled' THEN 'CANCELLED'
                         WHEN 'completed' THEN 'PUBLISHED'
                     END,
            updated_at = NOW()
        WHERE id = NEW.job_key;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Postgres < 14 doesn't support CREATE TRIGGER IF NOT EXISTS. Drop-then-create
-- is the pattern used elsewhere in this codebase (see pipeline_jobs_changed
-- trigger in colossus-pipeline's 001_create_pipeline_jobs.sql).
DROP TRIGGER IF EXISTS pipeline_jobs_sync_document_status ON pipeline_jobs;
CREATE TRIGGER pipeline_jobs_sync_document_status
    AFTER UPDATE ON pipeline_jobs
    FOR EACH ROW EXECUTE FUNCTION sync_document_status_from_pipeline_job();
