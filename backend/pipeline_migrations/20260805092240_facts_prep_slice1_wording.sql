-- facts_prep_slice1_wording: every word task 2.13 slice 1 puts on screen
--
-- Created: 2026-08-05 09:22:40
-- Target: pipeline database (colossus_legal_v2, applied at backend boot by the
--         runtime sqlx::migrate::Migrator — forward-only, no down migration)
--
-- THE CONFIGURATION LAW, TEXT HALF (Roman's ruling of 2026-08-04, carried into
-- the SCENARIO_FACTS_REDESIGN_v1 sign-off as rule 2):
--
--     "Every heading, prompt, label, button and message this task introduces is
--      served from the settings store with a default and a plain-language
--      description, and is editable on the admin Settings page. A literal
--      user-facing string in this task's code is a defect of the same class as a
--      compiled-in threshold."
--
-- So slice 1 introduces THIRTEEN new strings and thirteen rows, and no literal.
-- A missing or blank row is a BOOT REFUSAL naming the key, not a blank button —
-- see `domain::wording`.
--
-- ## Why the tier NAMES are here and the tier TOKENS are not
--
-- `scenario_fact_refs.tier` stores `carries` / `backup` / `background`: structure,
-- owned by the `FactTier` enum in code. What a human READS for each of those is
-- wording, owned by these rows. Roman renames "Carries the scenario" to whatever
-- he likes on the Settings page, and no stored row changes and no rebuild
-- happens. The signed design left the names explicitly open ("they live in
-- settings, so rename at will"), which only works if the two halves are kept
-- apart like this.
--
-- ## The boolean that is a two-token text row
--
-- `fact_tier_background_default_state` is a yes/no in spirit, and `ValueKind` has
-- no boolean — the store's kinds are float / count / ratio / text. Ruled
-- 2026-08-05 (Phase A, item 3): use a `text` row with a closed two-token
-- vocabulary rather than widening `ValueKind` in this slice. `collapsed` /
-- `expanded` parse into a Rust enum with a loud error on anything else, which is
-- the same parse-don't-validate discipline a real boolean kind would have given,
-- with none of the surface area. If a genuine boolean need accumulates, that is
-- its own small task with its own review.
--
-- ## ON CONFLICT DO NOTHING, as always
--
-- Makes the migration safely re-runnable, and means a value Roman has already
-- edited is never stamped back to ours. The consequence, learned in 2.12: editing
-- the VALUES list of an applied migration changes nothing anywhere — a correction
-- has to be its own later UPDATE, guarded on the old text.
--
-- FORWARD-ONLY: no down migration. A bad forward migration is corrected by a
-- FURTHER forward migration, never by editing this file once applied.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('fact_question_label', 'Q:', 'text', 'Q:',
     NULL, NULL,
     'Marks the interrogatory question printed above a discovery answer, on both '
     'the candidate cards and the scenario facts list. A bare "Yes" is noise; the '
     'same "Yes" under the question it answers is a sworn admission.',
     NULL, now(), 'system (seed)'),

    ('fact_statement_kind_label', 'Kind:', 'text', 'Kind:',
     NULL, NULL,
     'Introduces the kind of statement a fact is (for example "admission", '
     '"partial admission", "attorney argument") in the facts list.',
     NULL, now(), 'system (seed)'),

    ('fact_tier_carries_label', 'Carries the scenario', 'text', 'Carries the scenario',
     NULL, NULL,
     'The name of the heaviest weight a fact can be given: the ones you lead with. '
     'Rename freely — the stored value is a fixed token, so this is only what you read.',
     NULL, now(), 'system (seed)'),

    ('fact_tier_backup_label', 'Backup', 'text', 'Backup',
     NULL, NULL,
     'The name of the middle weight, and where every newly included fact lands. '
     'True and useful, but not what the scenario stands on.',
     NULL, now(), 'system (seed)'),

    ('fact_tier_background_label', 'Background', 'text', 'Background',
     NULL, NULL,
     'The name of the lightest weight: kept, but folded out of the way beneath the '
     'list. Nothing is ever hidden — the count is always shown and one click opens it.',
     NULL, now(), 'system (seed)'),

    ('fact_tier_prompt', 'How much does this fact carry?', 'text',
     'How much does this fact carry?',
     NULL, NULL,
     'The prompt on the weight control of a fact row, shown when you hover or focus it.',
     NULL, now(), 'system (seed)'),

    ('fact_tier_background_default_state', 'collapsed', 'text', 'collapsed',
     NULL, NULL,
     'Whether the Background pile starts folded away or already open. Exactly two '
     'values are understood: collapsed or expanded. Anything else stops the backend '
     'at boot rather than guessing.',
     NULL, now(), 'system (seed)'),

    ('fact_background_count_template', '{count} in the background · show', 'text',
     '{count} in the background · show',
     NULL, NULL,
     'The line standing in for the folded Background facts. {count} is replaced by '
     'how many there are, and must stay in the text — a pile that does not say how '
     'big it is looks like nothing is there.',
     NULL, now(), 'system (seed)'),

    ('fact_background_hide_label', 'Hide background', 'text', 'Hide background',
     NULL, NULL,
     'The control that folds the Background facts away again once they are open.',
     NULL, now(), 'system (seed)'),

    ('fact_order_drag_hint', 'Drag to reorder', 'text', 'Drag to reorder',
     NULL, NULL,
     'The label on a fact row''s drag handle, read aloud by screen readers and shown '
     'on hover. The order of the facts is the order of the argument.',
     NULL, now(), 'system (seed)'),

    ('fact_tier_save_failed_template',
     'Could not save the weight for {code}. {reason}', 'text',
     'Could not save the weight for {code}. {reason}',
     NULL, NULL,
     'Shown when a weight could not be stored. {code} names which fact and {reason} '
     'carries the failure''s own words; both must stay in the text, or the message '
     'cannot say what went wrong or to which fact.',
     NULL, now(), 'system (seed)'),

    ('fact_order_save_failed_template',
     'Could not save the new order for {code}. {reason}', 'text',
     'Could not save the new order for {code}. {reason}',
     NULL, NULL,
     'Shown when a drag could not be stored — including when there is no room left '
     'between two facts. {code} names the fact and {reason} carries the failure''s '
     'own words; both must stay in the text.',
     NULL, now(), 'system (seed)'),

    -- ─── The queue's collapsed summary (from the beta.376 click-through) ───────
    --
    -- Filed to this batch on 2026-08-05. The collapsed queue header read "All
    -- candidates ruled" on a scenario with 92 candidates still unruled. It is not
    -- a counting bug: the summary is computed from `ruled`/`total`, and before the
    -- queue has reported them BOTH are zero, so "nothing is unruled" is arithmetic
    -- ally true and substantively false — the pool is not empty, it is not yet
    -- known. One string and one condition (`total = 0`) separate the two, so a
    -- header can no longer announce that work is finished before it has looked.
    ('queue_empty_pool_summary', 'No candidates gathered yet', 'text',
     'No candidates gathered yet',
     NULL, NULL,
     'The queue''s one-line summary when there are no candidates at all — including '
     'the moment before the pool has loaded. Kept separate from "all ruled" so a '
     'queue that has not counted yet can never claim the work is done.',
     NULL, now(), 'system (seed)'),

    ('queue_all_ruled_summary', 'All candidates ruled', 'text', 'All candidates ruled',
     NULL, NULL,
     'The queue''s one-line summary when every gathered candidate has been ruled on. '
     'Shown only when there is a pool and none of it is outstanding.',
     NULL, now(), 'system (seed)')

ON CONFLICT (key) DO NOTHING;
