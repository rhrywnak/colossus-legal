-- qwen27b_display_name_drop_quantisation: Qwen27b display name drop quantisation
--
-- Created: 2026-09-11
-- Target: pipeline database
--
-- =============================================================================
-- The Qwen row stops saying "(NVFP4, local)" in its own name
-- =============================================================================
--
-- One row, one column. `unsloth/Qwen3.8-27B-NVFP4` was seeded on 2026-09-10
-- (migration `20260910105358`) with `display_name = 'Qwen3.8 27B (NVFP4, local)'`,
-- and it becomes `'Qwen3.8 27B'`.
--
-- ## Why the name may not carry its own parenthetical
--
-- `display_name` is the BARE name. Everything a picker or a confirmation adds to
-- it is composed from `billing_class` by `domain::billing_class`, which owns that
-- vocabulary precisely so a statement about what this deployment COSTS is made in
-- one place and not typed into a data row:
--
--   * the picker renders `display_label` — the bare name for a local model,
--     the name plus "(API — billed)" for a metered one;
--   * the scan confirmation renders `confirm_label` — the name plus
--     "(local · $0)" or "(API — billed)", because at the moment a human commits
--     to spending, silence about the free case is the wrong answer.
--
-- With "(NVFP4, local)" baked into the name, the confirmation read
--
--     Run a theme scan with Qwen3.8 27B (NVFP4, local) (local · $0)? …
--
-- — the same fact twice, once from the data and once from the vocabulary, with
-- the data's copy unable to change when the class does. Measured on DEV
-- 2026-09-11 and recorded as deviations 13 and 14 of the mockup table in
-- CC_REPORT_SCAN_HEADER_v1: the mockup draws "Qwen3.8 27B (local)" in the pill
-- and "Qwen3.8 27B (local · $0)" in the confirmation, which is exactly what the
-- composed labels produce once the name stops competing with them.
--
-- The quantisation is not lost. `NVFP4` is in the model ID, which is what an
-- operator matches on and what every log line and `scan_runs.model_id` carries.
-- It was never information the person choosing a model to spend on was reading.
--
-- ## Why the UPDATE is guarded on the old value
--
-- `WHERE … AND display_name = '<the seeded value>'` rather than on the id alone.
-- The guard makes a re-run a no-op and, more to the point, refuses to clobber a
-- name Roman has since edited by hand — the same discipline the wording rows get
-- from `ON CONFLICT DO NOTHING`. A row already carrying the new name matches
-- zero rows here and still satisfies the assertion below, which is the state we
-- want either way.

UPDATE llm_models
   SET display_name = 'Qwen3.8 27B'
 WHERE id = 'unsloth/Qwen3.8-27B-NVFP4'
   AND display_name = 'Qwen3.8 27B (NVFP4, local)';

-- ─── The row-count assertion (CLAUDE.md rule 25a) ─────────────────────────────
--
-- An UPDATE matching zero rows is SILENT in Postgres. Without this block a typo
-- in the id, or in the guard's old value, would report success and leave the old
-- name being served — which is the failure mode rule 25a exists for: the
-- migration passes, the deploy passes, and the screen is unchanged with nothing
-- to say why.
--
-- Asserted on the END state (exactly one row carries the new name) rather than
-- on the UPDATE's own row count, because the guarded UPDATE legitimately touches
-- zero rows on a database where the name is already correct. The END state is
-- the same in both cases and it is what the page depends on.
--
-- Counted across the whole table, not just this id, deliberately: two models
-- sharing one display name would make the scan picker ambiguous at exactly the
-- moment a human is choosing which one to spend money on.
DO $$
DECLARE
    named    INTEGER;
    current_name TEXT;
BEGIN
    SELECT count(*) INTO named
      FROM llm_models WHERE display_name = 'Qwen3.8 27B';

    IF named <> 1 THEN
        SELECT display_name INTO current_name
          FROM llm_models WHERE id = 'unsloth/Qwen3.8-27B-NVFP4';

        RAISE EXCEPTION
            'qwen27b display name: expected exactly 1 row named ''Qwen3.8 27B'' '
            'after this migration, found %. The row for '
            'unsloth/Qwen3.8-27B-NVFP4 currently reads %. Either that row is '
            'absent (migration 20260910105358 did not run), or its name was '
            'edited by hand to something this migration does not recognise, or '
            'a second model has been given the same name — which would make the '
            'scan picker ambiguous. Resolve the row by hand and re-run.',
            named, coalesce(quote_literal(current_name), 'NO SUCH ROW');
    END IF;
END $$;
