-- chat_default_model_row: the Chat default stops being a compiled-in name
--
-- Created: 2026-09-19 13:02:53
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_CHAT_DEFAULT_MODEL_v1 — THE ONE MIGRATION this task writes.
-- Roman's ruling of 2026-09-19 (§5 option 3): a settings row, seeded
-- claude-opus-5, and the backend refuses to start if it names a model that is
-- not live.
-- =============================================================================
--
-- ## The defect this ends
--
-- `main.rs` carried `const DEFAULT_CHAT_MODEL = "claude-sonnet-4-6"`. On
-- 2026-09-19 that model was deactivated in the Admin list on PROD, so it left
-- the chat provider map (which is built from `llm_models WHERE is_active`) and
-- every `/ask` without an explicit `model` field answered 400. The name was
-- compiled in, so the remedy was a build.
--
-- It is a row now. Changing the Chat default is a Settings edit and a restart.
--
-- ## Why claude-opus-5
--
-- Measured on 2026-09-19, it is the only Anthropic model `is_active = true` on
-- PROD, and it is also active on DEV — so this value is correct in both places
-- on the day it ships. It is the same value `theme_scan_default_model` already
-- holds, which is not a coincidence: they are the same question asked of two
-- surfaces.
--
-- ## Modelled on theme_scan_default_model
--
-- Same shape, same `text` kind, same "a model name is not wording" reasoning as
-- `20260810114629_r2_391_unified_names_one_attack_box_and_scan_default_model.sql:132`.
-- The difference is the assertion below.
--
-- ## Why the assertion reads llm_models
--
-- The backend refuses to BOOT if this row names a model that is not an active
-- Anthropic row (`main.rs`, after the chat provider map is built). That refusal
-- is correct and deliberate — a default that is not there is exactly what broke
-- PROD — but discovering it at boot means discovering it mid-deploy.
--
-- So the same question is asked HERE, in the same transaction, against the same
-- database. A store where the seeded default is not live fails the migration and
-- the deploy stops one step earlier, with a sentence naming the remedy.
--
-- ## Idempotent
--
-- ON CONFLICT (key) DO NOTHING — a re-run inserts nothing and the assertion
-- checks the END state either way (CLAUDE.md rule 25a). An operator who has
-- already pointed the row at a different live model keeps their choice.
--
-- ## Rolling this back
--
-- Delete the row. The previous build's compiled-in default returns with it;
-- this build refuses to start without the row, which is the point.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('chat_default_model', 'claude-opus-5', 'text', 'claude-opus-5',
     NULL, NULL,
     'The model the Chat page answers with when the request names none, and the '
     'model its picker opens on. Must name a row in llm_models that is '
     'is_active = true AND provider = ''anthropic'': the backend verifies that '
     'at startup and REFUSES TO START otherwise, because a default that is not '
     'in the provider map answers every unqualified /ask with a 400. To change '
     'the Chat default, edit this row and restart; to retire the model it names, '
     'point this row somewhere live FIRST.',
     'main::build_chat_providers, api::ask, api::chat_models', NOW(), 'migration')
ON CONFLICT (key) DO NOTHING;

-- ─── The END-state assertion (CLAUDE.md rule 25a) ────────────────────────────
--
-- Three silent failures are possible here and Postgres reports none of them:
-- an INSERT that collided with an existing row and did nothing; a blank value;
-- and — the one this task exists for — a value naming a model this database
-- cannot serve.

DO $$
DECLARE
    configured TEXT;
    live       INTEGER;
    why        TEXT;
BEGIN
    SELECT value INTO configured FROM app_settings WHERE key = 'chat_default_model';
    IF configured IS NULL OR btrim(configured) = '' THEN
        RAISE EXCEPTION
            'chat default: chat_default_model is missing or blank. The backend '
            'declares it at boot and refuses to start without it.';
    END IF;

    SELECT count(*) INTO live
      FROM llm_models
     WHERE id = configured
       AND is_active = true
       AND provider = 'anthropic';

    IF live <> 1 THEN
        -- Say WHICH of the three it is: the remedy differs for each, and an
        -- operator reading a failed deploy has no other source for it.
        SELECT CASE
                 WHEN NOT EXISTS (SELECT 1 FROM llm_models WHERE id = configured)
                   THEN 'no llm_models row has that id — check it for a typo, or add the row'
                 WHEN EXISTS (SELECT 1 FROM llm_models
                               WHERE id = configured AND provider <> 'anthropic')
                   THEN 'that row is not an anthropic model, and chat dispatch is '
                        'anthropic-only — point chat_default_model at an anthropic model'
                 ELSE 'that model is deactivated — activate it in the Admin model '
                      'list, or point chat_default_model at a live model'
               END
          INTO why;
        -- The remedy IS the message: whoever reads a refused deploy has no
        -- other source for which of the three it is. Each arm above carries its
        -- own first move, so the tail says only what the consequence is.
        RAISE EXCEPTION
            'chat default: chat_default_model names %, and %. The backend '
            'refuses to start on this, so fix it before deploying.', configured, why;
    END IF;
END $$;
