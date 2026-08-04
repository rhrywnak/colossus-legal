-- card_and_facts_usability_wording: the wording task 2.12 changes and adds
--
-- Created: 2026-08-04 16:25:38
-- Target: pipeline database (colossus_legal_v2, applied at backend boot by the
--         runtime sqlx::migrate::Migrator — forward-only, no down migration)
--
-- Task 2.12. Every item in that task is friction Roman hit by hand in his first
-- real working session on 2.10 (2026-08-04). Nothing here is invented.
--
-- ## Item A: the button no longer moves anyone, so it must stop saying it does
--
-- Saving a link used to advance to the next stuck card. That made the human do
-- the thinking on a card, get moved away, and scroll BACKWARDS to press Include
-- — two passes over the same hundred cards. Saving now leaves the selection
-- where it is, Include and Exclude unlock in place, and the ordinary
-- post-ruling advance carries them onward. One pass.
--
-- ## ⚠️ WHY THIS IS AN UPDATE AND NOT A CHANGED SEED
--
-- `link_save_label` is ALREADY in the database. Task 2.10's seed used
-- `ON CONFLICT (key) DO NOTHING`, which is what makes that migration safely
-- re-runnable — and which means editing its `VALUES` list now would change
-- nothing at all on any environment that has already run it. DEV has:
--
--     key = link_save_label | value = 'Save and next' | updated_by = 'system (seed)'
--
-- So the correction has to be an explicit UPDATE, and it has to be careful:
-- a human may have edited this row since, and their words are not ours to
-- overwrite.
--
-- ### The guard is the VALUE, deliberately, and not `updated_by`
--
-- `updated_by = 'system (seed)'` looks like the natural test for "nobody has
-- touched this", and it is the wrong one twice over: a human who edited the
-- label and then set it back to the seed text would be wrongly protected, and a
-- human who changed some other column would be wrongly overwritten. What
-- actually matters is whether the row still says the thing that is now false.
-- So the WHERE clause names the old text itself — if it no longer reads
-- 'Save and next', somebody chose what it says and this migration leaves it
-- alone.
--
-- `default_value` moves with `value` (and is guarded too): leaving it behind
-- would make the Settings page advertise "Default Save and next" for a default
-- that no longer exists, and would put the stored row out of step with
-- `domain::wording::Wording::for_test`.
--
-- `updated_by` is deliberately NOT touched. This is not a person's edit, and
-- that column's whole job is to say whether one has happened.
UPDATE app_settings
   SET value         = 'Save',
       default_value = 'Save',
       updated_at    = now()
 WHERE key           = 'link_save_label'
   AND value         = 'Save and next'
   AND default_value = 'Save and next';

-- ─── Item B: a disabled control must say why ───────────────────────────────────
--
-- Roman filled in a link panel, saw Include still greyed, and reported it as
-- broken. It was correct behaviour with no explanation — which is the same
-- defect class as 1.7C's silent keypress: a distinct state with no observable.
--
-- ─── Item G: taking a fact back out, from the row it is on ─────────────────────
--
-- Until now the only way out was to return to the candidate card and undo there
-- — item A's two-pass problem in its other form. The removal itself is the
-- EXISTING `record_removal` path (`DELETE …/facts/:graph_node_id`): the fact
-- reference is deleted, so the candidate is served with no ruling at all and
-- re-enters the queue as NOT RULED. It is emphatically not an exclude — the
-- human is saying "not this scenario", not "this is bad evidence" — and the
-- wording below has to carry that distinction, because the button cannot.
--
-- ON CONFLICT DO NOTHING keeps this idempotent, as with every seed here.
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('link_save_blocks_ruling', 'Save the link first.', 'text',
     'Save the link first.',
     NULL, NULL,
     'Shown beside a greyed-out Include and Exclude while a link panel holds '
     'accusations you have ticked but not yet saved. Without it, a correct '
     'refusal reads as a broken button.',
     NULL, now(), 'system (seed)'),

    ('fact_remove_label', 'Remove', 'text', 'Remove',
     NULL, NULL,
     'The control on a fact row that takes an item back out of this scenario.',
     NULL, now(), 'system (seed)'),

    ('fact_remove_confirm_template',
     'Remove {code} from this scenario? It goes back to the queue as not ruled.',
     'text',
     'Remove {code} from this scenario? It goes back to the queue as not ruled.',
     NULL, NULL,
     'The question asked before a fact is taken out. {code} is the candidate''s '
     'number and is required — a confirmation that does not name what it is '
     'about is not a confirmation. The second sentence matters as much as the '
     'first: removing is not excluding, and the human needs to know the item '
     'comes back rather than being judged bad evidence.',
     NULL, now(), 'system (seed)'),

    ('fact_remove_confirm_yes', 'Remove it', 'text', 'Remove it',
     NULL, NULL,
     'The button that confirms taking a fact out of the scenario.',
     NULL, now(), 'system (seed)'),

    ('fact_remove_confirm_cancel', 'Keep it', 'text', 'Keep it',
     NULL, NULL,
     'The button that abandons the removal and leaves the fact where it is.',
     NULL, now(), 'system (seed)'),

    ('fact_remove_failed_template',
     'That fact could not be removed: {detail} Reload the page to see what is actually stored.',
     'text',
     'That fact could not be removed: {detail} Reload the page to see what is actually stored.',
     NULL, NULL,
     'Shown when taking a fact out of the scenario failed. {detail} is the reason '
     'the server or the network gave, and is required — a message that says '
     'something went wrong and nothing about what leaves a human with no way to '
     'tell whether their ruling is still there.',
     NULL, now(), 'system (seed)')
ON CONFLICT (key) DO NOTHING;
