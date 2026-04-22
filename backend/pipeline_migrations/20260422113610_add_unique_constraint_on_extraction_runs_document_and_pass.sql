-- add_unique_constraint_on_extraction_runs_document_and_pass
--
-- Created: 2026-04-22 11:36:10
-- Target: pipeline database
--
-- Context (PIPELINE_CODEBASE_AUDIT.md §2.2 / R5)
-- -----------------------------------------------
-- A document should have at most one extraction_runs row per pass_number.
-- Today that invariant is upheld only by cleanup.rs running DELETE
-- statements before re-insertion. If cleanup is skipped (the post-R2
-- re-process path doesn't call it) or partially fails, a retry creates a
-- second row under a new synthetic id and both rows survive with their
-- own item/relationship/chunk children. This enforces the invariant at
-- the DB level so insert_extraction_run can upsert on conflict.
--
-- Pre-constraint dedupe
-- ---------------------
-- DEV/PROD may already carry duplicates from past partial-failure runs.
-- The ALTER TABLE ADD CONSTRAINT would fail if so. Keep the latest
-- (highest id) row for each (document_id, pass_number) pair and
-- cascade-delete the losers in FK-safe order:
--   review_edit_history -> extraction_relationships
--                       -> extraction_items
--                       -> extraction_chunks
--                       -> extraction_runs
-- No ON DELETE CASCADE is set on the underlying FKs, so the cascade is
-- written explicitly.
--
-- On a clean database every `losers` CTE is empty and the DELETEs are
-- no-ops, so this migration is safe to re-run.

WITH ranked AS (
    SELECT id,
           ROW_NUMBER() OVER (
               PARTITION BY document_id, pass_number
               ORDER BY id DESC
           ) AS rn
    FROM extraction_runs
),
losers AS (
    SELECT id FROM ranked WHERE rn > 1
)
DELETE FROM review_edit_history
WHERE item_id IN (
    SELECT id FROM extraction_items WHERE run_id IN (SELECT id FROM losers)
);

WITH ranked AS (
    SELECT id,
           ROW_NUMBER() OVER (
               PARTITION BY document_id, pass_number
               ORDER BY id DESC
           ) AS rn
    FROM extraction_runs
),
losers AS (
    SELECT id FROM ranked WHERE rn > 1
)
DELETE FROM extraction_relationships
WHERE run_id IN (SELECT id FROM losers);

WITH ranked AS (
    SELECT id,
           ROW_NUMBER() OVER (
               PARTITION BY document_id, pass_number
               ORDER BY id DESC
           ) AS rn
    FROM extraction_runs
),
losers AS (
    SELECT id FROM ranked WHERE rn > 1
)
DELETE FROM extraction_items
WHERE run_id IN (SELECT id FROM losers);

WITH ranked AS (
    SELECT id,
           ROW_NUMBER() OVER (
               PARTITION BY document_id, pass_number
               ORDER BY id DESC
           ) AS rn
    FROM extraction_runs
),
losers AS (
    SELECT id FROM ranked WHERE rn > 1
)
DELETE FROM extraction_chunks
WHERE extraction_run_id IN (SELECT id FROM losers);

WITH ranked AS (
    SELECT id,
           ROW_NUMBER() OVER (
               PARTITION BY document_id, pass_number
               ORDER BY id DESC
           ) AS rn
    FROM extraction_runs
)
DELETE FROM extraction_runs
WHERE id IN (SELECT id FROM ranked WHERE rn > 1);

-- Enforce the invariant going forward. insert_extraction_run will upsert
-- on this constraint (ON CONFLICT (document_id, pass_number) DO UPDATE).
ALTER TABLE extraction_runs
    ADD CONSTRAINT extraction_runs_doc_pass_unique UNIQUE (document_id, pass_number);
