-- chat_keepwarm_settings: the automatic keep-loaded ping's five settings and the
-- three words the Admin "Chat case file" box says about it
--
-- Created: 2026-09-25 08:19:41
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_CACHE_KEEPWARM_v1 R3 (GO 2026-09-25, rulings on PLAN R3 §9).
-- =============================================================================
--
-- ## Why
--
-- The chat's case file (~369k tokens) stays in the provider's cache for one hour
-- after its last use. A pre-warm inside that hour costs about $0.07; letting it
-- expire costs about $2.95 at the next question. The backend now sends that
-- pre-warm on its own, inside a daily window, while somebody is chatting. These
-- five rows are its whole configuration, read live on every wake of the pinger,
-- so an edit takes effect at the next ping with no restart.
--
-- The three wording rows are the one line the box adds under "Loaded until":
-- on (with the window), on but paused for the rest of the day, or off. The
-- backend fills {start} and {end}; the browser draws the finished sentence.
--
-- ## Kinds
--
-- `chat_keepwarm_enabled` is text `on` / `off` (the store has no boolean kind),
-- and the window times are text `HH:MM` in the case's timezone
-- (`practice_case_timezone`). The reader refuses any other spelling at boot and
-- on save, naming the key, and refuses a window whose end is not after its start.
--
-- ## Fresh-database reproduction
--
-- A database built from zero creates `app_settings` in 20260801225147 and seeds
-- these rows here. `ON CONFLICT (key) DO NOTHING` makes a re-run insert nothing
-- and never overwrite a value Roman has edited; the assertion passes on both.
-- The boot migrator applies it; no operator SQL is needed.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('chat_keepwarm_enabled', 'on', 'text', 'on', NULL, NULL,
     'Keep the chat''s case file loaded automatically while people are chatting: on or off. Takes effect at the next ping — no restart.',
     NULL, now(), 'migration'),
    ('chat_keepwarm_window_start', '06:00', 'text', '06:00', NULL, NULL,
     'The time of day (HH:MM, 24-hour, in the case''s timezone) from which the automatic ping may run. Takes effect at the next ping — no restart.',
     NULL, now(), 'migration'),
    ('chat_keepwarm_window_end', '23:00', 'text', '23:00', NULL, NULL,
     'The time of day (HH:MM, 24-hour, in the case''s timezone) after which the automatic ping stops until tomorrow. Must be after the start. Takes effect at the next ping — no restart.',
     NULL, now(), 'migration'),
    ('chat_keepwarm_interval_minutes', '48', 'count', '48', 5, 55,
     'How many minutes after the last chat question or ping the next automatic ping is sent. Must be shorter than the case file''s loaded time (one hour). Takes effect at the next ping — no restart.',
     NULL, now(), 'migration'),
    ('chat_keepwarm_daily_cap_dollars', '2.00', 'float', '2.00', 0, 100,
     'The most the automatic pings may cost in one day, in dollars. When the next ping would pass it, they stop until tomorrow. Takes effect at the next ping — no restart.',
     NULL, now(), 'migration'),
    ('chat_case_file_automatic_on', 'Automatic: on, {start} – {end}', 'text', 'Automatic: on, {start} – {end}', NULL, NULL,
     'Admin → Overview, Chat case file: the line when the automatic ping is on. {start} and {end} are the window''s times.',
     NULL, now(), 'migration'),
    ('chat_case_file_automatic_paused', 'Automatic: on, paused until tomorrow', 'text', 'Automatic: on, paused until tomorrow', NULL, NULL,
     'Admin → Overview, Chat case file: the line when the automatic ping is on but has stopped for the rest of the day (the daily cap, a reload, a model without known prices, or the window has closed).',
     NULL, now(), 'migration'),
    ('chat_case_file_automatic_off', 'Automatic: off', 'text', 'Automatic: off', NULL, NULL,
     'Admin → Overview, Chat case file: the line when the automatic ping is switched off.',
     NULL, now(), 'migration')
ON CONFLICT (key) DO NOTHING;

-- ─── The END-state assertion (CLAUDE.md rule 25a) ────────────────────────────
-- All eight rows present with the kind this build reads — and no stray row under
-- either prefix, which the LIKE counts catch (underscores escaped so `_` does not
-- match any single character).
DO $$
DECLARE
    typed    INTEGER;
    params   INTEGER;
    words    INTEGER;
BEGIN
    SELECT count(*) INTO typed
      FROM app_settings
     WHERE (key, value_kind) IN (
        ('chat_keepwarm_enabled', 'text'),
        ('chat_keepwarm_window_start', 'text'),
        ('chat_keepwarm_window_end', 'text'),
        ('chat_keepwarm_interval_minutes', 'count'),
        ('chat_keepwarm_daily_cap_dollars', 'float'),
        ('chat_case_file_automatic_on', 'text'),
        ('chat_case_file_automatic_paused', 'text'),
        ('chat_case_file_automatic_off', 'text'));
    SELECT count(*) INTO params FROM app_settings WHERE key LIKE 'chat\_keepwarm\_%';
    SELECT count(*) INTO words FROM app_settings WHERE key LIKE 'chat\_case\_file\_automatic\_%';
    IF typed <> 8 OR params <> 5 OR words <> 3 THEN
        RAISE EXCEPTION
            'chat keep-warm settings: expected 8 typed rows (5 chat_keepwarm_*, 3 chat_case_file_automatic_*), found typed=% params=% words=%',
            typed, params, words;
    END IF;
END $$;
