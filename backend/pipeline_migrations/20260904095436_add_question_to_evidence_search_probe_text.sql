-- add_question_to_evidence_search_probe_text: the 367 discovery-response cards
-- become reachable by the request they answer.
--
-- Created: 2026-09-04 09:54:36
-- Target: pipeline database (colossus_legal_v2, applied at backend boot by the
--         runtime sqlx::migrate::Migrator over ./pipeline_migrations — NOT the
--         compile-time migrate! macro, which serves ./migrations and the MAIN
--         database. See database.rs::init_pools for the two-pool split.)
--
-- ## The defect
--
-- 367 Evidence nodes carry a `question` — the interrogatory or request for
-- admission the card answers. 99 of them have an answer-only `verbatim_quote`:
-- `Admitted.`, `Denied as untrue.`, `No.` The card DTO shows `question` and the
-- scan judge reads it, but neither retrieval half can see it. L1a's `probe_text`
-- is generated from quote + title + significance, so for those 99 the trigram
-- half is matching against the word "Admitted." and nothing else. Measured
-- 2026-09-01: six of the seven $50,000 admissions were reachable only through
-- `title`.
--
-- ## What this migration does, and why it is a drop-and-recreate
--
-- `probe_text` is GENERATED ALWAYS ... STORED. Postgres has no
-- `ALTER COLUMN ... SET EXPRESSION` for a stored generated column in the version
-- this project runs (17.7 — the syntax arrives in PG 18), so the only way to
-- change the expression is to drop the column and add it back. The trigram index
-- is built ON that column, so it goes and comes back with it. Both are recreated
-- in the same transaction the migrator wraps this file in, so there is no window
-- in which the table exists without its trigram half.
--
-- Dropping the column rewrites every row's stored value — which is the point:
-- existing mirror rows get a probe_text that includes their question with no
-- backfill step of any kind. Cheap today (the table is empty on DEV, measured
-- below) and correct whenever it is not.
--
-- ## Why `question` FIRST in the concatenation
--
-- No functional reason — trigram matching is order-independent over a flat
-- surface, exactly as L1a's comment says. It is written first because that is
-- the order the instruction specified, and because reading the column left to
-- right then tells the story of the card: the request, then the answer, then our
-- own commentary on it.
--
-- ## What this migration deliberately does NOT touch
--
-- `search_vector`, the FULL-TEXT half, is still generated from quote/title/
-- significance and remains blind to `question`. That is not an oversight and it
-- is not fixed here: adding a fourth field to a weighted tsvector is a ranking
-- change (it needs a weight letter, and 'D' is the only one left), and ranking
-- is a decision, not a mechanical edit. Recorded in
-- CC_REPORT_QUESTION_IN_RETRIEVAL_v1 as a ruling for Roman with the exact
-- one-line change ready.
--
-- ## MERGE ORDER — this file REQUIRES L1a
--
-- `evidence_search` is created by
-- `20260901083845_evidence_search_table.sql` on branch
-- `feat/evidence-search-l1a`, which is NOT merged. Measured on DEV 2026-09-04,
-- read-only: `to_regclass('public.evidence_search')` is NULL, `pg_trgm` is not
-- installed, and version 20260901083845 is absent from `_sqlx_migrations`
-- (114 applied, latest 20260902091005). So L1a has never run there.
--
-- sqlx 0.8.6's `Migrator::run_direct` applies every migration whose version is
-- absent from `_sqlx_migrations`, in version order, and does NOT refuse one
-- whose version sorts below an already-applied migration — it only errors when
-- an APPLIED migration is missing from disk (`VersionMissing`). So L1a's
-- 20260901083845 will still apply on DEV even though 20260902091005 already has,
-- and because 20260904095436 sorts after it, ONE boot creates the table and then
-- alters it, in that order, with no manual step.
--
-- The preflight below refuses loudly if this file is reached without L1a rather
-- than degrading. A guard that quietly no-opped would be far worse: the migrator
-- records the version as applied either way, so a silent skip would mean this
-- migration NEVER runs again once L1a lands, and the trigram half would stay
-- blind forever with nothing in any log to say why.
--
-- ## FORWARD-ONLY
--
-- Same as L1a: this repo has no down files and no down convention. A bad forward
-- migration is corrected by a further forward migration. The manual undo is:
--
--     DROP INDEX IF EXISTS idx_evidence_search_probe_trgm;
--     ALTER TABLE evidence_search DROP COLUMN probe_text;
--     ALTER TABLE evidence_search ADD COLUMN probe_text TEXT GENERATED ALWAYS AS (
--         coalesce(quote, '') || ' ' || coalesce(title, '') || ' ' ||
--         coalesce(significance, '')
--     ) STORED;
--     CREATE INDEX idx_evidence_search_probe_trgm
--         ON evidence_search USING GIN (probe_text gin_trgm_ops);
--     ALTER TABLE evidence_search DROP COLUMN question;

-- Preflight. See "MERGE ORDER" above for why this raises rather than skips.
DO $$
BEGIN
    IF to_regclass('public.evidence_search') IS NULL THEN
        RAISE EXCEPTION
            'evidence_search does not exist. This migration extends the table created by '
            '20260901083845_evidence_search_table.sql on branch feat/evidence-search-l1a, '
            'which is not merged. Merge L1a before this branch — sqlx applies both in one '
            'boot, in version order, because 20260901083845 sorts before 20260904095436.';
    END IF;
END $$;

-- The request the card answers. Nullable, and for the same reason `title` and
-- `significance` are: 842 of the 1,209 Evidence nodes have no question at all,
-- and NULL says "the graph had none" while '' would say "somebody stored an
-- empty one". `coalesce` in the generated expression flattens both to nothing.
--
-- IF NOT EXISTS so a re-run over a hand-patched database is a no-op rather than
-- a failure; the assertion block at the bottom is what makes that safe, because
-- a no-op ADD leaves the shape unchecked and the block checks it.
ALTER TABLE evidence_search ADD COLUMN IF NOT EXISTS question TEXT;

COMMENT ON COLUMN evidence_search.question IS
    'The interrogatory or request for admission this Evidence answers, mirrored from the graph node''s `question` property. 367 of 1209 nodes carry one; 99 of those have an answer-only quote ("Admitted.") and are unreachable by any lexical search without it. Nullable: NULL means the graph had none, distinct from an empty string.';

-- Dropping the generated column would cascade to this index on its own. It is
-- dropped explicitly anyway, so the file states its own intent and so the
-- recreate below is visibly paired with a drop rather than appearing to create
-- an index that already exists.
DROP INDEX IF EXISTS idx_evidence_search_probe_trgm;

ALTER TABLE evidence_search DROP COLUMN IF EXISTS probe_text;

-- The same flat, unweighted matching surface L1a documented, plus `question`.
-- Single spaces between fields so a trigram cannot bridge the end of one field
-- into the start of the next.
ALTER TABLE evidence_search ADD COLUMN probe_text TEXT GENERATED ALWAYS AS (
    coalesce(question, '') || ' ' || coalesce(quote, '') || ' ' ||
    coalesce(title, '') || ' ' || coalesce(significance, '')
) STORED;

COMMENT ON COLUMN evidence_search.probe_text IS
    'Generated (never triggered, so it cannot drift): question, quote, title and significance concatenated flat, space-separated. The trigram matching surface. Flat and unweighted because trigram has no weighting to give. `question` was added 2026-09-04: 99 of 1209 cards have an answer-only quote ("Admitted.", "Denied as untrue.") and were unreachable by the trigram half without the request text. NOTE: search_vector, the full-text half, does NOT yet carry question.';

CREATE INDEX IF NOT EXISTS idx_evidence_search_probe_trgm
    ON evidence_search USING GIN (probe_text gin_trgm_ops);

COMMENT ON INDEX idx_evidence_search_probe_trgm IS
    'Trigram half of the lexical gather. Covers probe_text = question + quote + title + significance. Everything L1a measured about what this index can and cannot do still holds: it separates "$50,000" from a bare "50,000", it does NOT separate "$50,000" from "$500,000.00", it reaches substrings ("Milste" -> "Milster"), and it does not reach OCR transpositions at Postgres''s default word_similarity threshold.';

-- This migration seeds no rows, so Rule 25a's row-count assertion does not
-- apply. What needs asserting is the SHAPE, for the reason L1a's own block gives:
-- every statement above is `IF NOT EXISTS` / `IF EXISTS`, and those are silent
-- no-ops against a pre-existing object. Without this block a database whose
-- `probe_text` had been hand-rebuilt without `question` would pass this
-- migration and stay blind, with nothing anywhere to say so.
DO $$
DECLARE
    generated_kind   TEXT;
    generation_expr  TEXT;
    question_type    TEXT;
    index_count      INTEGER;
BEGIN
    SELECT data_type INTO question_type
      FROM information_schema.columns
     WHERE table_name = 'evidence_search' AND column_name = 'question';
    IF question_type IS DISTINCT FROM 'text' THEN
        RAISE EXCEPTION
            'evidence_search.question is % rather than text — the mirror cannot carry the request',
            coalesce(question_type, '<absent>');
    END IF;

    SELECT is_generated, generation_expression
      INTO generated_kind, generation_expr
      FROM information_schema.columns
     WHERE table_name = 'evidence_search' AND column_name = 'probe_text';

    IF generated_kind IS DISTINCT FROM 'ALWAYS' THEN
        RAISE EXCEPTION
            'evidence_search.probe_text is not a generated column (is_generated = %) — it could drift from the row it summarises',
            coalesce(generated_kind, '<absent>');
    END IF;

    -- The assertion this whole migration exists for. A `probe_text` that is
    -- generated but does not read `question` is exactly the state the 99
    -- answer-only cards were already in, and it would look healthy.
    IF generation_expr IS NULL OR position('question' IN generation_expr) = 0 THEN
        RAISE EXCEPTION
            'evidence_search.probe_text does not read `question` (expression: %) — the 99 answer-only cards would stay unreachable',
            coalesce(generation_expr, '<null>');
    END IF;

    SELECT count(*) INTO index_count
      FROM pg_indexes
     WHERE tablename = 'evidence_search'
       AND indexname = 'idx_evidence_search_probe_trgm';
    IF index_count <> 1 THEN
        RAISE EXCEPTION
            'the trigram index over probe_text is missing after the rebuild — the trigram half of the gather would silently return nothing';
    END IF;
END $$;
