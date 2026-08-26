-- documents_phase_comment: the last reference to the retired timeline.json
--
-- Created: 2026-08-26
-- Target: pipeline database (colossus_legal_v2)
--
-- The R-2 leftover from CC_REPORT_TIMELINE_PHASE_B_v1, riding Phase C as ruled.
--
-- ## Why this is a migration and not an edit
--
-- `20260817150412_add_document_phase.sql` set this column's COMMENT, and that
-- migration is APPLIED. An applied migration is never edited — sqlx checksums
-- them, and a changed file makes the next boot refuse. So correcting a comment
-- is a new migration, which is exactly what a comment is for: it is data in the
-- catalogue, not source.
--
-- ## What was wrong with it
--
-- It named `frontend/public/data/timeline.json` twice — as the source of the
-- phase slugs and as the home of the display labels. That file was DELETED in
-- Phase B (v2.0.0-beta.411). The slugs are guarded by `domain::case_phase` and
-- by two SQL CHECKs; the labels live in `chronology_phases`, served by the
-- backend (design R15). A comment pointing an operator at a path that does not
-- exist is worse than no comment: it is a confident wrong answer, and psql
-- prints it on \d+ documents with the same authority as the column type.
--
-- ## Domain note — the phases stay INDEPENDENT
--
-- Design R13: an event's phase and its linked document's phase are independent,
-- and a divergence is allowed and visible, never auto-fixed. The new text says
-- so, because the previous text's "the same slugs the timeline uses" reads like
-- a promise that the two agree. They share a VOCABULARY; they do not share a
-- value.

COMMENT ON COLUMN documents.phase IS
    'Which phase of the case this document belongs to: estate | probate | appeals | civil_lawsuit. The same four slugs as domain::case_phase, chronology_phases.id and chronology_events.phase, guarded here by documents_phase_valid. Display labels live in chronology_phases, served by the backend — never in this column and never in a file. NULL means nobody has said yet; it is never required. A document''s phase and a chronology event''s phase are INDEPENDENT (design R13): a divergence between them is allowed and visible, and nothing ever silently corrects one store from the other.';

-- ⚑ ROW-COUNT ASSERTION (CLAUDE.md 25a).
--
-- COMMENT ON is silent about a column that does not exist? No — it errors. What
-- it IS silent about is having written nothing an operator will ever read,
-- because the comment could equally have landed on a column nobody looks at.
-- This asserts the END STATE the migration exists to produce: the column has a
-- comment, that comment no longer names the retired file, and it does name the
-- table the labels actually live in.
DO $$
DECLARE
    current_comment TEXT;
BEGIN
    SELECT col_description(c.oid, a.attnum) INTO current_comment
      FROM pg_class c
      JOIN pg_namespace n ON n.oid = c.relnamespace
      JOIN pg_attribute a ON a.attrelid = c.oid
     WHERE c.relname = 'documents'
       AND a.attname = 'phase'
       AND a.attnum > 0
       AND NOT a.attisdropped
       AND n.nspname = current_schema();

    IF current_comment IS NULL THEN
        RAISE EXCEPTION
            'documents.phase has no comment after this migration ran; the COMMENT ON did not land';
    END IF;
    IF current_comment LIKE '%timeline.json%' THEN
        RAISE EXCEPTION
            'documents.phase still names the retired timeline.json: %', current_comment;
    END IF;
    IF current_comment NOT LIKE '%chronology_phases%' THEN
        RAISE EXCEPTION
            'documents.phase does not name chronology_phases, so it points an operator nowhere: %', current_comment;
    END IF;
END $$;
