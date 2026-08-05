-- facts_polish_2_13c_wording: the words task 2.13c adds to the facts card
--
-- Created: 2026-08-05 14:57:31
-- Target: pipeline database (colossus_legal_v2, applied at backend boot by the
--         runtime sqlx::migrate::Migrator — forward-only, no down migration)
--
-- Five new strings, every one of them a row rather than a literal (the
-- configuration law's text half). Each answers something Roman reported on the
-- deployed .378 build:
--
--   * the answer needs a prefix, because a question with a label and an answer
--     without one are not the equals the design says they are;
--   * a fact that moves to the background pile must SAY so — his words: the dot
--     made things "silently vanish";
--   * the three weights and the drag must disclose themselves on screen rather
--     than being discoverable only by hovering;
--   * the footer counted folded facts as "shown", which was untrue;
--   * a placed fact could not be un-placed at all, so there was no control to
--     name.
--
-- ON CONFLICT DO NOTHING keeps this re-runnable and never stamps over a value a
-- human has edited. The consequence, learned in 2.12: editing the VALUES list of
-- an applied migration changes nothing anywhere — a correction is its own later
-- UPDATE, guarded on the old text.
--
-- FORWARD-ONLY: no down migration.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('fact_answer_label', 'A:', 'text', 'A:',
     NULL, NULL,
     'Marks the answer beneath an interrogatory question, as the partner of the '
     'question label. The two print at the same size in the same column so the '
     'exchange reads as a question and its answer rather than a heading and a body.',
     NULL, now(), 'system (seed)'),

    ('fact_background_move_notice',
     '{code} moved to the background pile — show', 'text',
     '{code} moved to the background pile — show',
     NULL, NULL,
     'Shown when a fact is weighed down into the background pile and leaves the '
     'list. {code} names which fact and must stay in the text — a card that '
     'disappears without saying where it went is the failure this notice exists '
     'to prevent.',
     NULL, now(), 'system (seed)'),

    ('fact_weights_hint',
     'Star each fact by how much it carries — carries, backup, background — and drag to set the order.',
     'text',
     'Star each fact by how much it carries — carries, backup, background — and drag to set the order.',
     NULL, NULL,
     'The one line under the Scenario facts heading that tells a reader the '
     'weights and the drag exist. A feature nobody can see is a feature nobody '
     'uses; this is the surface disclosing itself.',
     NULL, now(), 'system (seed)'),

    ('fact_footer_template', '{shown} shown · {background} in background', 'text',
     '{shown} shown · {background} in background',
     NULL, NULL,
     'The count beneath the facts list. Both placeholders must stay in the text: '
     'the old wording called folded facts "shown", which was untrue, and the only '
     'honest line names the two numbers separately.',
     NULL, now(), 'system (seed)'),

    ('fact_unplace_label', 'Clear my order', 'text', 'Clear my order',
     NULL, NULL,
     'The control that forgets where a human dragged one fact, returning it to '
     'the list''s natural order. Until this existed a drag was permanent short of '
     'removing the fact from the scenario entirely.',
     NULL, now(), 'system (seed)')

ON CONFLICT (key) DO NOTHING;
