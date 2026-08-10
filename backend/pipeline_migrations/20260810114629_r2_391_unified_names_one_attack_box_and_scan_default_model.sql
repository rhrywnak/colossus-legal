-- r2_391_unified_names_one_attack_box_and_scan_default_model
--
-- Created: 2026-08-10 11:46:29
-- Target: pipeline database (colossus_legal_v2)
--
-- Task CC_TASK_R2_SURFACE_BATCH_391_v1. Three kinds of change, one deploy.
--
-- ## 1 — ONE THING, ONE NAME (Roman, 2026-08-10)
--
-- The identity of a scenario was asked for in one vocabulary and displayed in
-- another. The read-only block called a field "Our theme — one sentence" and the
-- editor one click away called the same column "Our answer, in one sentence";
-- "Their motivation" was edited as "What they want the jury to believe"; "Bears
-- on" was edited as "Complaint paragraphs this touches". Four columns, eight
-- names, and nothing but a reader's memory connecting them.
--
-- The ruling: THE HEADER'S NAMES WIN. The rows below are that vocabulary, and
-- both surfaces now render the SAME row — not two rows seeded with the same text,
-- which is a pair free to drift the first time one is edited. One name, one row,
-- every surface, and Roman retunes any of them from the Settings page with no
-- build.
--
-- ## 2 — value corrections on rows that already exist
--
-- `link_cut_supports_label` reads "It supports us  !!!" (two spaces, three
-- exclamation marks). That is a typo that reached a lawyer-facing card, not a
-- name awaiting ratification, so it is corrected rather than deferred — the
-- no-renames rule protects vocabulary under review, not slips.
--
-- `card_filter_progress_template` loses its `{filter}` slot. The line no longer
-- measures the active filter (it measures every candidate the scans put forward,
-- which does not move when a chip is clicked), so a template naming a filter
-- would be describing something the number is not about. "ruled" becomes
-- "addressed" because Include, Exclude and Defer are all rulings and the sentence
-- has to cover all three.
--
-- ## 3 — the scan's default model becomes a row (10e)
--
-- BENEATH the `THEME_SCAN_MODEL` env var, not replacing it: read order is env var
-- → this row → the catalogue's existing fallbacks. Additive, so no deploy or
-- Ansible change rides with it. Retiring the env var (and paying the
-- colossus-ansible template entry it has owed since D2b) stays filed.
--
-- ## Why `ON CONFLICT (key) DO NOTHING` for inserts and explicit UPDATEs for edits
--
-- Seeding is idempotent and must never overwrite a value Roman has since tuned.
-- The three corrections below are deliberate overwrites of values nobody would
-- want to keep, so they are UPDATEs — and each is fenced with a WHERE on the
-- current value, so a re-run cannot clobber a later edit of Roman's.
--
-- Forward-only, no down migration, applied at backend boot by the Migrator. A
-- declared key with no row REFUSES START (v2 §2b), so this must reach the
-- database with or before the .391 image.

-- ── 1 · The unified identity vocabulary ─────────────────────────────────────
--
-- Shared by `ScenarioIdentityBlock` (read) and `ScenarioIdentityModal` (edit).
-- One row per idea, deliberately: the whole defect was two names for one column.
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('scenario_identity_attack_label',
     'The attack — what they claim, in their words', 'text',
     'The attack — what they claim, in their words',
     NULL, NULL,
     'Names definition->>attack_text on BOTH the identity block and the identity '
     'editor. Until .391 the block said "The attack — what they claim" and the '
     'editor said "What they say — quote it if you can"; one column, two names, '
     'one click apart.',
     'ScenarioIdentityBlock, ScenarioIdentityModal', NOW(), 'migration'),

    ('scenario_identity_attack_absent',
     'No attack text written yet — the pencil opens the editor.', 'text',
     'No attack text written yet — the pencil opens the editor.',
     NULL, NULL,
     'Shown in place of the attack text when nobody has written one. A stated '
     'absence, never an empty paragraph — an empty <p> reads as a broken render.',
     'ScenarioIdentityBlock', NOW(), 'migration'),

    ('scenario_identity_theme_label',
     'Our theme — one sentence', 'text',
     'Our theme — one sentence',
     NULL, NULL,
     'Names the theme_statement column on both identity surfaces. The header''s '
     'name won over the editor''s "Our answer, in one sentence" (Roman, '
     '2026-08-10).',
     'ScenarioIdentityBlock, ScenarioIdentityModal', NOW(), 'migration'),

    ('scenario_identity_theme_absent',
     'No theme written yet.', 'text', 'No theme written yet.',
     NULL, NULL,
     'The stated absence for the theme statement.',
     'ScenarioIdentityBlock', NOW(), 'migration'),

    ('scenario_identity_theme_helper',
     'Read aloud in rehearsal mode.', 'text', 'Read aloud in rehearsal mode.',
     NULL, NULL,
     'Under the theme field in the editor: says who reads this sentence and '
     'where, which is what stops it being written as a case note.',
     'ScenarioIdentityModal', NOW(), 'migration'),

    ('scenario_identity_motivation_label',
     'Their motivation — what they want the jury to believe', 'text',
     'Their motivation — what they want the jury to believe',
     NULL, NULL,
     'Names the motivation column on both identity surfaces. Roman''s unified '
     'name keeps the header''s noun ("Their motivation") and the editor''s '
     'explanation, so neither surface loses what it was saying.',
     'ScenarioIdentityBlock, ScenarioIdentityModal', NOW(), 'migration'),

    ('scenario_identity_motivation_absent',
     'No motivation written yet.', 'text', 'No motivation written yet.',
     NULL, NULL,
     'The stated absence for the motivation.',
     'ScenarioIdentityBlock', NOW(), 'migration'),

    ('scenario_identity_bears_on_label',
     'Bears on', 'text', 'Bears on',
     NULL, NULL,
     'Names anchor_allegation_ids on both identity surfaces. The editor called '
     'it "Complaint paragraphs this touches"; the header''s shorter name won.',
     'ScenarioIdentityBlock, ScenarioIdentityModal', NOW(), 'migration'),

    ('scenario_identity_bears_on_absent',
     'No allegations linked yet.', 'text', 'No allegations linked yet.',
     NULL, NULL,
     'The stated absence for the allegation chips.',
     'ScenarioIdentityBlock', NOW(), 'migration'),

    -- ── 3 · the scan's default model (10e) ──────────────────────────────────
    ('theme_scan_default_model', 'claude-opus-5', 'text', 'claude-opus-5',
     NULL, NULL,
     'The model the Theme Scan picker opens on, read BENEATH the '
     'THEME_SCAN_MODEL env var: env var → this row → the catalogue''s own '
     'fallbacks. Before .391 the third step was list ORDER, so the picker''s '
     'default depended on how the registry happened to sort. Additive — the env '
     'var still wins where it is set, so no deploy change rides with this.',
     'api::chat_models::list_scan_models', NOW(), 'migration')
ON CONFLICT (key) DO NOTHING;

-- ── 2 · Corrections to rows that already exist ──────────────────────────────
--
-- Each fenced on the CURRENT value, so re-running this migration cannot overwrite
-- a later edit of Roman's. If he has already retuned one of these by hand, the
-- UPDATE matches nothing and his value stands.

-- The typo that reached a lawyer-facing card.
UPDATE app_settings
   SET value         = 'Supports us',
       default_value = 'Supports us',
       updated_at    = now()
 WHERE key           = 'link_cut_supports_label'
   AND value         = 'It supports us  !!!';

-- The progress line stopped measuring the active filter, so it stopped naming
-- one. `{filter}` is removed from `REQUIRED_PLACEHOLDERS` in the same build —
-- leaving it required would refuse this very value on the Settings write path.
UPDATE app_settings
   SET value         = '{ruled} of {total} addressed',
       default_value = '{ruled} of {total} addressed',
       updated_at    = now()
 WHERE key           = 'card_filter_progress_template'
   AND value         = '{ruled} of {total} {filter} ruled';

-- NOT edited here, deliberately: `scenario_identity_meaning_needs_attack_text`.
--
-- Its .390 text says "the words you just typed", which was true while the editor
-- still had a "what that is meant to imply" box. That box is gone (one attack
-- box), so the only way to reach this refusal now is a LEGACY row whose gloss was
-- authored earlier — and the sentence then describes typing that did not happen.
--
-- It is left alone because the disk/code consistency test
-- (`wording_scenario_authoring_tests`) pins the fixture against the INSERT that
-- seeded it, and an UPDATE here would put the database and that fixture into
-- disagreement with nothing failing to say so. Roman can retune the row from the
-- Settings page in ten seconds; a build is the expensive way to fix a sentence on
-- a path that needs a pre-.391 scenario to reach. Filed in the completion report.
