-- scan_header_confirm_wording: Scan header confirm wording
--
-- Created: 2026-09-11
-- Target: pipeline database
--
-- =============================================================================
-- The words the rebuilt Scenario-facts header speaks
-- =============================================================================
--
-- Sixteen rows for one change: the scan control on the scenario-facts header
-- stops starting runs and starts ASKING, and the line beside the heading stops
-- being able to lie about the run history.
--
-- ## Why the last-scan line needed rows of its own
--
-- Until today the header showed the served "No scan has run yet" notice whenever
-- no run had COMPLETED. On PROD S-13 that produced a screen where the sentence
-- "No scan has run yet" sat three lines above a history table listing a
-- cancelled run. Both came out of this database and only one was true.
--
-- The fix is one source, not a better sentence: the header now renders either the
-- newest run's own line — which names the status, so a cancelled run reads as
-- cancelled — or the never-scanned notice, and `count_scan_runs` decides which.
-- These rows are that line.
--
-- ## Why the confirmation is three sentences and not one with slots
--
-- The estimate is MEASURED (`seconds_per_candidate_by_model`), so a model nobody
-- has scanned with yet has no estimate at all. A single template with an optional
-- `{minutes}` would render "about  minutes" the first time such a model was
-- picked, and a defaulted number would promise a human an afternoon nothing had
-- ever measured. Three complete sentences, one per state that can actually occur:
-- timed, untimed, and no candidate count (which has no estimate either, the
-- estimate being the pool size times the rate).
--
-- ## The four literals this retires
--
-- `scan_header_scan_label`, `scan_header_history_label`,
-- `scan_header_running_notice` and `scan_header_no_model_notice` were literals in
-- `ScenarioFactsHeader.tsx`, listed in that file as rows it owed. Two of them
-- changed wording in the rebuild ("Scan again" → "Scan", because the control no
-- longer scans; "history" → "History", because it is a pill now and not an inline
-- link), and a changed sentence may not go back into a component as a literal.
--
-- Idempotent: `ON CONFLICT (key) DO NOTHING`, so a re-run changes nothing and a
-- row Roman has edited is never clobbered.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    -- ── The split pill and the two refusals ──────────────────────────────────
    ('scan_header_scan_label', 'Scan', 'text', 'Scan',
     NULL, NULL,
     'The left half of the split Scan|model pill on the Scenario facts header. '
     'It lost the word "again" when the control stopped starting a run: it opens '
     'the confirmation bar, and a button reading "Scan again" that does not scan '
     'is a lie a human only finds by clicking it.',
     NULL, now(), 'system (seed)'),

    ('scan_header_history_label', 'History', 'text', 'History',
     NULL, NULL,
     'The pill that opens the scan run history. Capitalised because it is a pill '
     'beside the others now, where it used to be a lowercase inline link.',
     NULL, now(), 'system (seed)'),

    ('scan_header_running_notice', 'A scan is running — its progress is below.', 'text', 'A scan is running — its progress is below.',
     NULL, NULL,
     'Why the scan control is refused while a run is in flight. Shown on the '
     'control itself, because a control that refuses without saying why reads as '
     'a broken one.',
     NULL, now(), 'system (seed)'),

    ('scan_header_no_model_notice', 'No scan-eligible model is available.', 'text', 'No scan-eligible model is available.',
     NULL, NULL,
     'Why the scan control is refused when the model catalogue offers nothing. '
     'Kept distinct from the running notice: one resolves itself in minutes and '
     'the other needs an operator, and a single sentence covering both could not '
     'tell a human which of those they are looking at.',
     NULL, now(), 'system (seed)'),

    -- ── The honest last-scan line ────────────────────────────────────────────
    ('scan_header_last_scan_template', 'Last scan {when} · {status} · {count} candidates', 'text', 'Last scan {when} · {status} · {count} candidates',
     NULL, NULL,
     'The line beneath the Scenario facts heading, describing the newest run '
     'whatever became of it. {when} is filled by the browser because the date is '
     'formatted in the reader''s locale; {status} is one of the four '
     'scan_header_status_* rows; {count} is the candidate pool the run read.',
     NULL, now(), 'system (seed)'),

    ('scan_header_last_scan_no_count_template', 'Last scan {when} · {status}', 'text', 'Last scan {when} · {status}',
     NULL, NULL,
     'The same line for a run that never reached its pool read. A separate '
     'sentence rather than {count} filled with 0, because "0 candidates" claims '
     'the run looked and found nothing — and a run that died before reading the '
     'pool did not look.',
     NULL, now(), 'system (seed)'),

    -- ── The four run states, mid-sentence ────────────────────────────────────
    --
    -- Lowercase, and deliberately not the scan_status_*_label pills: those are
    -- chips on the history table where a capital reads as a badge. "Last scan
    -- Sep 11 · Complete · 313 candidates" reads as a proper noun dropped into a
    -- sentence. Same states, two registers, two sets of rows.
    ('scan_header_status_completed', 'completed', 'text', 'completed',
     NULL, NULL,
     'How the last-scan line says a run that finished. Lowercase: it sits '
     'mid-sentence, not in a badge.',
     NULL, now(), 'system (seed)'),

    ('scan_header_status_cancelled', 'cancelled', 'text', 'cancelled',
     NULL, NULL,
     'How the last-scan line says a run somebody stopped. This is the state that '
     'produced the PROD S-13 defect: before these rows existed, a scenario whose '
     'only run was cancelled was told "No scan has run yet".',
     NULL, now(), 'system (seed)'),

    ('scan_header_status_failed', 'failed', 'text', 'failed',
     NULL, NULL,
     'How the last-scan line says a run that died. The run is still named on the '
     'page; what it may not do is propose candidates.',
     NULL, now(), 'system (seed)'),

    ('scan_header_status_running', 'running', 'text', 'running',
     NULL, NULL,
     'How the last-scan line says a run still in flight.',
     NULL, now(), 'system (seed)'),

    -- ── The confirmation bar ─────────────────────────────────────────────────
    ('scan_header_confirm_timed_template', 'Run a theme scan with {model}? {count} candidates, about {minutes} minutes.', 'text', 'Run a theme scan with {model}? {count} candidates, about {minutes} minutes.',
     NULL, NULL,
     'The confirmation when this deployment has measured how long the chosen '
     'model takes. {model} is the server-composed confirm label, which states the '
     'cost for both billing classes ("(local · $0)" / "(API — billed)"). '
     '{minutes} is derived from completed runs of that model, never configured.',
     NULL, now(), 'system (seed)'),

    ('scan_header_confirm_template', 'Run a theme scan with {model}? {count} candidates.', 'text', 'Run a theme scan with {model}? {count} candidates.',
     NULL, NULL,
     'The confirmation when nothing has timed this model yet. The time clause is '
     'absent rather than guessed: an estimate is either measured or it does not '
     'exist.',
     NULL, now(), 'system (seed)'),

    ('scan_header_confirm_no_count_template', 'Run a theme scan with {model}?', 'text', 'Run a theme scan with {model}?',
     NULL, NULL,
     'The confirmation when the candidate count could not be read. With no pool '
     'size there is no estimate either, the estimate being the pool size times '
     'the measured rate.',
     NULL, now(), 'system (seed)'),

    ('scan_header_confirm_run_label', 'Run scan', 'text', 'Run scan',
     NULL, NULL,
     'The button that actually starts the run. It is the only control in the '
     'application wired to the scan start — the pill that opens this bar is not.',
     NULL, now(), 'system (seed)'),

    ('scan_header_confirm_cancel_label', 'Cancel', 'text', 'Cancel',
     NULL, NULL,
     'The button that dismisses the confirmation without running anything.',
     NULL, now(), 'system (seed)'),

    -- ── The overflow menu (card-grammar bundle) ──────────────────────────────
    --
    -- `⋯` is a shape, and the shape stays in code with the other glyphs. But a
    -- button whose visible content is a shape has no accessible NAME, and a
    -- screen reader reaching it announces only "button". The name is a sentence
    -- a human reads, so it is a row — filed with Reset order, the action it
    -- holds, rather than with the scan.
    ('card_more_actions_label', 'More actions', 'text', 'More actions',
     NULL, NULL,
     'The accessible name of the ⋯ overflow control on the Scenario facts '
     'header. Reset order moved inside it on 2026-09-11: it discards weeks of '
     'curation and it was sitting one mis-click from the fold.',
     NULL, now(), 'system (seed)')
ON CONFLICT (key) DO NOTHING;

-- ─── The row-count assertion (CLAUDE.md rule 25a) ─────────────────────────────
--
-- A statement matching zero rows is SILENT in Postgres, and `ON CONFLICT DO
-- NOTHING` makes that possible on purpose. A key mistyped above inserts nothing,
-- this migration reports success, and the backend then refuses to start with a
-- missing-key error naming the key and not the file that should have seeded it.
-- Assert the END state.
DO $$
DECLARE
    present INTEGER;
BEGIN
    SELECT count(*) INTO present
      FROM app_settings
     WHERE key IN (
        'scan_header_scan_label', 'scan_header_history_label',
        'scan_header_running_notice', 'scan_header_no_model_notice',
        'scan_header_last_scan_template',
        'scan_header_last_scan_no_count_template',
        'scan_header_status_completed', 'scan_header_status_cancelled',
        'scan_header_status_failed', 'scan_header_status_running',
        'scan_header_confirm_timed_template', 'scan_header_confirm_template',
        'scan_header_confirm_no_count_template',
        'scan_header_confirm_run_label', 'scan_header_confirm_cancel_label',
        'card_more_actions_label');

    IF present <> 16 THEN
        RAISE EXCEPTION
            'scan header wording: expected 16 settings rows after this '
            'migration, found %. The backend enumerates every scan_header_* key '
            'and card_more_actions_label at boot and refuses to start when one '
            'is missing, so this migration fails here rather than at the next '
            'deploy.', present;
    END IF;
END $$;
