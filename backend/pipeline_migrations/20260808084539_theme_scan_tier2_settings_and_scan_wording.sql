-- theme_scan_tier2_settings_and_scan_wording: the scan's parameters and its words
--
-- Created: 2026-08-08 08:45:39
-- Target: pipeline database (colossus_legal_v2)
--
-- Task CC_TASK_SATURDAY_TIER2_AND_UI_v1, pieces 1b/1c/1d and 3a
-- (SCAN_QUALITY_DESIGN_v1 Tier 2, as amended by Roman on 2026-08-07/08).
--
-- ## Six rows, three jobs
--
-- 1. `theme_scan_prompt_file` — WHICH judging prompt a scan reads at start.
--    This one REPLACES the `THEME_SCAN_PROMPT_FILE` env var and the compiled-in
--    default in `config.rs`, both retired in the same commit. Measured on DEV:
--    the env var was never set, so a compiled constant silently decided which
--    prompt judged every scan and nothing on any screen said so. It seeds as
--    `theme_scan_prompt_v3.md` — the Tier-1 prompt, already pushed to DEV's
--    template directory (commit eb22353).
--
-- 2. Two pre-filter parameters — what never reaches the judge. Both are dials
--    Roman can turn from the Settings page with no build, which is the point:
--    the first scan REPORTS how many rows each rule set aside, and the number
--    that looks wrong can be changed the same minute.
--
-- 3. Three scan-surface strings + one scenario notice — the words for the
--    conservation line, the two scan-history controls, and the never-scanned
--    empty state.
--
-- ## The deploy ordering hazard, and why it is real for this file
--
-- All six keys are declared to the boot loader (`REQUIRED_KEYS` and the two
-- `*_WORDING_KEYS` lists), and a declared key with no row makes the backend
-- REFUSE TO START — there are no compiled-in defaults to serve with (v2 §2b).
-- The runtime Migrator applies this at backend boot, before the settings load,
-- so a normal deploy orders itself. A rollback to an older image is safe (extra
-- rows are ignored); a roll-FORWARD without this file is not.
--
-- ## Why `ON CONFLICT (key) DO NOTHING` and no down migration
--
-- The house rule for every seeding migration: idempotent, and a re-run must never
-- overwrite a value Roman has since edited on the Settings page. Forward-only.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    -- ── 1. Which prompt judges (piece 1d) ────────────────────────────────────
    --
    -- Text, not a wording row: it names a FILE, not a sentence anybody reads.
    -- The write path additionally refuses a filename that does not resolve in the
    -- template directory, so a typo is caught on the page rather than at the next
    -- restart; boot refuses to start if the stored name has stopped resolving.
    ('theme_scan_prompt_file', 'theme_scan_prompt_v3.md', 'text',
     'theme_scan_prompt_v3.md',
     NULL, NULL,
     'The judging prompt a Theme Scan reads at the moment it starts, by filename, '
     'from the extraction-template directory. Changing it takes effect on the next '
     'scan with no restart. The file must already be deployed — a name that does '
     'not resolve is refused here, and would stop the service at its next start.',
     NULL, now(), 'system (seed)'),

    -- ── 2. What never reaches the judge (piece 1b) ───────────────────────────
    --
    -- 60 is the design's number (SCAN_QUALITY_DESIGN_v1 item 2.3), and Roman
    -- seeded it knowing its blast radius is unmeasured: the measurement counted
    -- 41 of 148 pool quotes under 60 characters but did not report how many of
    -- those also lack a paired question, which is the clause that decides. The
    -- first run's conservation counts answer that, and this row is how the answer
    -- gets acted on.
    ('theme_scan_prefilter_min_chars', '60', 'count', '60',
     0, NULL,
     'A quote shorter than this many characters, AND with no interrogatory '
     'question paired to it, is set aside before the judge sees it — an '
     'unanchored fragment cannot bear on an accusation. A short ANSWER whose '
     'question is in evidence is always judged: the question is what gives it '
     'meaning. Set to 0 to switch the rule off. Every set-aside quote is counted '
     'by reason on the scan''s results.',
     NULL, now(), 'system (seed)'),

    -- The vocabulary is the EXTRACTOR'S, which is exactly why it is a row: a new
    -- document type can introduce a kind without a rebuild. `referral` is the one
    -- the measurement caught being admitted as relevant ("See the responses to
    -- the previous interrogatories.") — a cross-reference with no assertion in it.
    ('theme_scan_prefilter_statement_types', 'referral', 'text', 'referral',
     NULL, NULL,
     'Statement kinds that never reach the judge, comma-separated (for example: '
     'referral). Matched case-insensitively against the kind the extractor '
     'recorded. Write the single word none to switch the rule off — the field '
     'cannot be left blank.',
     NULL, now(), 'system (seed)'),

    -- ── 3a. The conservation line (piece 1c) ─────────────────────────────────
    --
    -- Composed at READ time from each run's frozen counts, so editing this
    -- sentence re-words every historical run without rewriting one of them. All
    -- five placeholders are REQUIRED — an edit that drops one is refused, because
    -- a reconciliation missing a term still looks reconciled.
    ('scan_conservation_line_template',
     '{pool} gathered · {collapsed} duplicates folded · {excluded} set aside before judging · {judged} judged · {relevant} relevant',
     'text',
     '{pool} gathered · {collapsed} duplicates folded · {excluded} set aside before judging · {judged} judged · {relevant} relevant',
     NULL, NULL,
     'The line under a scan run''s counts that reconciles what was gathered with '
     'what was judged. Must keep {pool}, {collapsed}, {excluded}, {judged} and '
     '{relevant} — the whole point of the sentence is that its numbers add up.',
     NULL, now(), 'system (seed)'),

    -- ── 3b. The two scan-history controls (piece 4) ──────────────────────────
    ('scan_history_view_label', 'View results', 'text', 'View results',
     NULL, NULL,
     'The control on a scan-history row that reopens that run''s results. The row '
     'was already clickable and did not look it, which is why a completed run '
     'appeared unreachable after a page refresh.',
     NULL, now(), 'system (seed)'),

    ('scan_history_delete_confirm_template',
     'Remove the scan run from {run}? Its verdicts are deleted with it, and they are what support the rulings it produced.',
     'text',
     'Remove the scan run from {run}? Its verdicts are deleted with it, and they are what support the rulings it produced.',
     NULL, NULL,
     'Asked before a scan run is destroyed. Must keep {run}, which names the run '
     'by when it was started — the history lists several rows that otherwise look '
     'alike.',
     NULL, now(), 'system (seed)'),

    -- ── 3c. The never-scanned empty state (piece 3a) ─────────────────────────
    --
    -- The sibling of `scenario_no_target_notice`, and the answer to a different
    -- question: that one says there is nobody to gather about, this one says
    -- nobody has judged any of it yet. Measured 2026-08-07 — a freshly created
    -- scenario led with 148 raw pool cards under "Candidates awaiting ruling —
    -- from all scans", on a screen that also reported them never scanned.
    ('scenario_never_scanned_notice',
     'No scan has run yet. Run a theme scan above, or browse the raw evidence pool.',
     'text',
     'No scan has run yet. Run a theme scan above, or browse the raw evidence pool.',
     NULL, NULL,
     'Shown in place of the candidate queue on a scenario nothing has scanned yet. '
     'The raw evidence pool is still one click away, behind its own control — what '
     'this replaces is the claim that those rows came from a scan.',
     NULL, now(), 'system (seed)'),

    -- ── 3d. The opt-in that opens the raw pool (piece 3a) ────────────────────
    --
    -- Lives with the curation surface's words because that is where the queue's
    -- other summaries live and where the browser already receives them.
    ('queue_raw_pool_toggle_template', 'Browse the raw evidence pool ({count})',
     'text', 'Browse the raw evidence pool ({count})',
     NULL, NULL,
     'The control that opens the raw evidence pool on a never-scanned scenario. '
     'Must keep {count} — a control that hides how much is behind it is the '
     'reason the pool was leading the page in the first place.',
     NULL, now(), 'system (seed)')

ON CONFLICT (key) DO NOTHING;
