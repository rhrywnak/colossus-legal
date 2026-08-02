-- create_app_settings_store: Create the app settings store
--
-- Created: 2026-08-01 22:51:47
-- Target: pipeline database (colossus_legal_v2, applied at backend boot by the
--         runtime sqlx::migrate::Migrator — forward-only, no down migration)
--
-- Tracker task 1.6. THE CONFIGURATION LAW (v2 §2b, Roman's ruling, binding):
--
--   "every numeric threshold, limit, band boundary, and tunable parameter … is
--    read from stored configuration at runtime. Changing a parameter NEVER
--    requires a rebuild, redeploy, or code change. … A compiled-in parameter is
--    a defect of the same class as a hardcoded person name: no parameters in
--    code, ever."
--
-- This migration builds the store the law requires and seeds it with the seven
-- parameters the scenario function has today — four of them lifted out of code
-- by this same task, three ratified but not yet consumed.
--
-- ## Why `app_settings` and not `scenario_settings`
--
-- Every parameter seeded here belongs to the scenario function, but nothing in
-- the SHAPE of the table does. Naming it for the one caller it has today would
-- mean either a second table or an awkward rename the first time a tunable from
-- outside the scenario surface arrives (tracker 3.11 sweeps for those — e.g.
-- `rerank_threshold`, an env var with an `unwrap_or` default today).
--
-- ## Why not extend `rag_config`
--
-- That table (migration 20260418) is key + JSONB and holds CASE DATA — document
-- aliases, person names. It carries no type, no bounds, no default and no
-- meaning text, all of which §2b's admin page requires. It also has zero readers
-- in colossus-legal and zero in colossus-rs; whether it is a future feature or a
-- corpse is filed as tracker 3.11, not decided here.

-- ─── 1. The store ──────────────────────────────────────────────────────────────
--
-- ## Why one TEXT value column and a declared kind
--
-- Three typed columns (value_num / value_int / value_text) would need a CHECK
-- that exactly one is non-null, and would give the Settings page three edit
-- widgets for what a human thinks of as one field. JSONB is over-general: the
-- only non-scalar parameter here is the card-test ratio, and that is two
-- integers, not a document.
--
-- TEXT + `value_kind` gives one uniform edit box on the page, one typed parser
-- per kind in code, a value that reads plainly in psql, and bounds enforced
-- numerically by the parser against `min_value` / `max_value`.
CREATE TABLE app_settings (
    -- The stable key code reads by. Never renamed: a renamed key is a MISSING
    -- parameter at boot, which is a refusal to start (by design — see below).
    key           TEXT        PRIMARY KEY,

    -- The current value, as text. Parsed per `value_kind` at read time.
    value         TEXT        NOT NULL,

    -- 'float' | 'count' | 'ratio'. Free TEXT rather than a CHECK for the same
    -- reason the scenario status vocabulary is: the kind list lives in code
    -- (`domain::settings::ValueKind`) and is deliberately extensible, while a
    -- CHECK would have to be migrated in lockstep with it. An unknown kind is
    -- refused loudly at boot, which is a stronger guarantee than a CHECK gives —
    -- a CHECK cannot tell you the PARSER is missing.
    value_kind    TEXT        NOT NULL,

    -- What this parameter shipped as. Displayed beside the current value so a
    -- human can see they have changed something, and what from. NOT a fallback:
    -- nothing in code ever reads this to recover from a bad `value`.
    default_value TEXT        NOT NULL,

    -- Inclusive bounds, NULL meaning unbounded on that side. NUMERIC rather than
    -- TEXT because these are compared, never displayed raw.
    min_value     NUMERIC,
    max_value     NUMERIC,

    -- One line of plain language, with its § citation. This is what the admin
    -- page shows a human who has never read the requirements document — the law
    -- names it as a required column of the v1 surface, not a nicety.
    meaning       TEXT        NOT NULL,

    -- NULL when a live code path reads this parameter today. Otherwise the task
    -- that will ('task 2.4'), so the page can say plainly that changing it does
    -- nothing yet. A settings page that silently lists inert knobs is a page
    -- that lies about its own reach.
    consumed_by   TEXT,

    -- Last-change display, so the page needs no join for "changed by X". The
    -- HISTORY is `app_setting_changes` — see the header there for why both.
    updated_at    TIMESTAMPTZ NOT NULL,
    updated_by    TEXT        NOT NULL
);

COMMENT ON TABLE app_settings IS
    'The configuration store required by v2 §2b: every tunable parameter with '
    'its type, bounds, default and plain-language meaning. Loaded at boot into '
    'one snapshot and re-loaded on write; a missing or invalid parameter is a '
    'BOOT REFUSAL, because after task 1.6 there are no compiled-in defaults to '
    'fall back to.';

-- ─── 2. The change ledger ──────────────────────────────────────────────────────
--
-- ## Why a ledger and not just updated_by / updated_at
--
-- Those two columns hold only the LATEST edit. "Who changed N the night before
-- trial" survives exactly until the next edit — and the night before trial is
-- precisely when a value gets changed twice. This is the same argument that
-- carried `scenario_status_transitions` (task 1.5), and the answer should not be
-- different because the subject is a number instead of a status.
--
-- Append-only, no FK to `app_settings`: the record of a human decision must
-- outlive the row it was about, exactly as in the readiness ledger. No key is
-- deleted today; the record should not depend on that staying true.
CREATE TABLE app_setting_changes (
    id        UUID        PRIMARY KEY DEFAULT gen_random_uuid(),

    -- No FK — see the header.
    key       TEXT        NOT NULL,

    -- Both sides, so a ledger row reads without needing the setting's current
    -- state or the previous ledger row. `old_value` is NOT NULL: every change
    -- has a before, because the seed guarantees the key already exists.
    old_value TEXT        NOT NULL,
    new_value TEXT        NOT NULL,

    -- Who. NOT NULL: an unattributed configuration change is the thing this
    -- table exists to make impossible.
    actor     TEXT        NOT NULL,
    at        TIMESTAMPTZ NOT NULL,

    -- A change that changes nothing is not a change. The write service refuses
    -- it upstream with a readable sentence; this is the backstop.
    CONSTRAINT app_setting_changes_actually_change
        CHECK (old_value <> new_value)
);

CREATE INDEX idx_app_setting_changes_key ON app_setting_changes (key, at DESC);

COMMENT ON TABLE app_setting_changes IS
    'Append-only history of every configuration change: key, before, after, who, '
    'when. app_settings.updated_by holds only the latest edit; this holds all of '
    'them, because the question asked later is nearly always about an edit that '
    'has since been superseded.';

-- ─── 3. The seed ───────────────────────────────────────────────────────────────
--
-- All seven parameters, defaults equal to the values in force on 2026-08-01 — so
-- this migration changes NO behaviour. It moves where four numbers are read from,
-- and makes three more editable ahead of the code that will read them.
--
-- ON CONFLICT DO NOTHING keeps the migration idempotent (the rag_config
-- precedent). `updated_by` is 'system (seed)' so a never-edited parameter stays
-- visibly distinguishable from one a human has set.
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    -- ── Live: read by code as of this migration ───────────────────────────────
    ('confidence_band_high', '0.80', 'float', '0.80', 0, 1,
     'A scan''s self-reported confidence at or above this shows as the High band '
     'on a candidate card. Bands exist so nobody reads a raw percentage as a '
     'measurement (§7). Must be greater than the medium cutoff.',
     NULL, now(), 'system (seed)'),

    ('confidence_band_medium', '0.50', 'float', '0.50', 0, 1,
     'A scan''s self-reported confidence at or above this — but below the high '
     'cutoff — shows as the Medium band. Anything lower shows as Low (§7).',
     NULL, now(), 'system (seed)'),

    ('quote_context_window_chars', '240', 'count', '240', 0, NULL,
     'How many characters of the surrounding page are shown on each side of a '
     'quote on a candidate card, so a ruling is made in context rather than on a '
     'fragment (§7.1).',
     NULL, now(), 'system (seed)'),

    ('talking_points_cap', '3', 'count', '3', 1, NULL,
     'The most talking points one scenario may carry. Marie holds a few points '
     'under pressure, not a list (§10).',
     NULL, now(), 'system (seed)'),

    -- ── Dormant: ratified, no consumer yet. Listed and editable so the page is
    --    complete; labelled so it is honest about doing nothing today.
    ('readiness_item_threshold_n', '5', 'count', '5', 1, NULL,
     'How many included, citable items a scenario needs before it can be called '
     'ARMED rather than brittle (§9). Changing this has no effect today.',
     'task 2.4', now(), 'system (seed)'),

    ('card_test_ratio', '9/10', 'ratio', '9/10', NULL, NULL,
     'The share of candidates in a scan run that must be rulable from the card '
     'alone for the card test to pass, written as n/m (§7). Changing this has no '
     'effect today.',
     'task 2.4', now(), 'system (seed)'),

    ('reanchor_close_match_tolerance', '0.85', 'float', '0.85', 0, 1,
     'How close a re-found quote must be to count as the same quote, as a '
     'similarity from 0 to 1. PROVISIONAL — task 2.5 (§12.1) defines the matching '
     'algorithm and may re-seed this with a different unit. Changing this has no '
     'effect today.',
     'task 2.5', now(), 'system (seed)')
ON CONFLICT (key) DO NOTHING;
