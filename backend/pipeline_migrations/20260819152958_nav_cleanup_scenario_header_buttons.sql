-- nav_cleanup_scenario_header_buttons: Unav_cleanup_scenario_header_buttons
--
-- Created: 2026-08-19 15:29:58
-- Target: pipeline database
--
-- TODO: Add your SQL here.


-- ─── The scenario header's two controls stop pointing at each other ──────────
--
-- ## What Roman saw (2026-08-19 evening)
--
-- `Rehearsal view →` and `Practice ▸` sat side by side in the scenario header,
-- each ending in a right-pointing glyph. The eye joins them: it reads as ONE
-- control with a decoration in the middle, or as an arrow aimed at its
-- neighbour. Neither is what they are — they are two different destinations.
--
-- So both lose the glyph and become plain buttons, the same size and shape as
-- `✎ Edit` beside them. `Rehearsal view` is secondary (the read-only study
-- page); `Practice` is primary (where the work happens).
--
-- The arrow on `Rehearsal view →` is a code literal and moves in the component.
-- THIS one is a stored value, so it moves here — and the two halves have to land
-- in the same release or the header shows one button with an arrow and one
-- without.
--
-- Written in the alignment `domain::wording::tests::corrected_value_in` parses.
-- Get it wrong and the fixture test goes green while the store holds the arrow.
UPDATE app_settings
   SET value         = 'Practice',
       updated_at    = NOW(),
       updated_by    = 'migration'
 WHERE key           = 'scenario_practice_link_label';


-- ─── The deck editor's drag grip ─────────────────────────────────────────────
--
-- Roman's 08-19 evening item: the practice deck's rows re-order by DRAG in edit
-- mode, reusing the grip the scenario-facts table already draws. The ▲▼ arrows
-- stay — they are the keyboard path, and a re-order only a mouse can perform is
-- one Chuck cannot do from the keyboard at all.
--
-- The hint is both the `title` and the `aria-label`, so the grip says the same
-- thing to a pointer and to a screen reader.
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('practice_editor_drag_hint', 'Drag to re-order within this side', 'text',
     'Drag to re-order within this side', NULL, NULL,
     'The grip on a deck row in edit mode. Says "within this side" because that '
     'is the rule the server enforces: George''s questions and Chuck''s are two '
     'ordered lists, and a cross question dragged among the directs would deal a '
     'Chuck question in a George sitting.',
     'frontend PracticeDeckRow', NOW(), 'migration');
