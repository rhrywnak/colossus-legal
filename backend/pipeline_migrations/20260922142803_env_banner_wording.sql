-- env_banner_wording: the test-system warning bar
--
-- Created: 2026-09-22 14:28:03
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_ENV_BANNER_v1, mockup of record ENV_BANNER_MOCKUP_v1_2026-09-22.
-- =============================================================================
--
-- ## Why
--
-- Chuck and Marie can reach both machines, and nothing on screen said which one
-- they were on. Practice typed on the test machine is not there for trial. The
-- bar says so, on every page, and cannot be dismissed.
--
-- ## What decides whether it shows — NOT these rows
--
-- The browser's own runtime config (`window.__COLOSSUS_CONFIG__.environment`,
-- written by Ansible into config.js and already carrying "dev" / "prod" on the
-- two machines today). Anything that is not exactly the production token shows
-- the bar, including a missing value: a false "this is the real one" is the
-- dangerous failure, a false warning costs nothing. These rows are only the
-- WORDS.
--
-- ## ⚑ env_banner_text is duplicated in the browser bundle, on purpose
--
-- The bar must be on screen at first paint — before any request returns and
-- even when the backend is down. So the frontend carries a compiled fallback of
-- this exact sentence and swaps in the stored value when it arrives. A vitest
-- (`envBanner.test.ts`) reads THIS FILE and fails if the two ever differ, so
-- rewording the row without rewording the fallback cannot ship.
--
-- ## Fresh-database reproduction
--
-- These four keys are new, so this migration is their only source: a database
-- built from zero has them once it runs. Idempotent — ON CONFLICT (key) DO
-- NOTHING — so a re-run changes nothing and a row somebody edited is never
-- clobbered.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('env_banner_text', 'TEST SYSTEM — practice here is not saved for trial.', 'text', 'TEST SYSTEM — practice here is not saved for trial.', NULL, NULL,
     'The warning across the top of every page on the test machine. It is never shown on the real system. User words only: it must not name the machine, the environment, the host or the version — a witness needs to know that practice typed here is not kept for trial, not what the build knows about itself. The browser carries a compiled copy of this exact sentence so the warning paints before anything loads and survives a backend outage; a test fails if the two differ.',
     NULL, now(), 'migration'),

    ('env_banner_link_label', 'Go to the real system', 'text', 'Go to the real system', NULL, NULL,
     'The link beside the warning, pointing at env_banner_real_url. The warning never waits for it: the sentence paints first and the link appears with the stored words.',
     NULL, now(), 'migration'),

    ('env_banner_print_line', 'TEST SYSTEM — NOT FOR TRIAL', 'text', 'TEST SYSTEM — NOT FOR TRIAL', NULL, NULL,
     'The shorter warning printed at the top of EVERY page of anything printed from the test machine — paper leaves the screen behind, and a printed practice deck must not be mistaken for the real one. Printed from the real system, nothing is added.',
     NULL, now(), 'migration'),

    ('env_banner_real_url', 'https://colossus-legal.cogmai.com', 'text', 'https://colossus-legal.cogmai.com', NULL, NULL,
     'Where "Go to the real system" goes. A per-deployment address, which is why it is a row and not a literal in the code.',
     NULL, now(), 'migration')
ON CONFLICT (key) DO NOTHING;

-- ─── The END-state assertion (CLAUDE.md rule 25a) ────────────────────────────
DO $$
DECLARE
    present INTEGER;
BEGIN
    SELECT count(*) INTO present
      FROM app_settings
     WHERE key IN ('env_banner_text', 'env_banner_link_label',
                   'env_banner_print_line', 'env_banner_real_url');
    IF present <> 4 THEN
        RAISE EXCEPTION
            'env banner: expected 4 wording rows, found %. The backend declares '
            'every one at boot and refuses to start without it.', present;
    END IF;
END $$;
