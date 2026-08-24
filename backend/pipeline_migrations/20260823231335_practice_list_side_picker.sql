-- practice_list_side_picker: one side at a time, in the authored story order.
--
-- Created: 2026-08-23 23:13:35
-- Target: pipeline database (colossus_legal_v2)
--
-- ## What this is for
--
-- PRACTICE_MOCKUP_v8 replaces the interleaved deck list with a side picker. One
-- side shows at a time, uninterrupted, in the order the deck is authored in. The
-- defense's view is the cross, top to bottom. Chuck's view is his directs in deck
-- order and then his redirects in deck order, each redirect carrying the defense
-- question it repairs.
--
-- ## Domain note: why the interleave had to go
--
-- The pairing it drew — each defense trap followed by the redirect that repairs
-- it — describes a COURTROOM MOMENT correctly and describes NEITHER PERSON'S JOB.
-- Marie answers one side at a time. Chuck reads one side at a time. A list that
-- alternates is a list in which neither of them can find their place, and the
-- measured consequence on 2026-08-23 was that nine of Chuck's ten questions on
-- S-5 had never been answered at all.
--
-- ## ⚑ FOUR NEW ROWS, AND ONE CORRECTION — and why only four are new
--
-- The picker's two BUTTON LABELS are NOT new rows. `practice_who_george_title`
-- and `practice_who_chuck_title` already hold "The defense asks" and "Chuck asks"
-- — the same two choices the practice bar offers three inches above, which is the
-- whole point of the picker reusing them. Two rows saying the same words is two
-- places to edit and one of them eventually not edited.
--
-- The redirect's quoted antecedent is not a new row either: it is drawn by the
-- shared `PrintAntecedent` component and reads `practice_print_after_template`
-- ("After the defense asks: {question}"), which is already exactly the sentence
-- the mockup draws.
--
-- ## ⚑ TWO FORMAT RULES, BOTH LOAD-BEARING, BOTH LEARNED THE HARD WAY
--
-- 1. `value` and `default_value` must each be ONE quoted literal on ONE line.
--    `seeded_value_in` reads the first quoted literal after the key; adjacent
--    literal concatenation is invisible to it and pins a truncated string.
--
-- 2. The key must sit IMMEDIATELY after the opening paren -- `VALUES ('key',` --
--    because the parser's marker is the two characters `('` followed by the key.
--    L1's migration was first written with the key indented on its own line:
--    perfectly good SQL, and the fixture then reported the key as seeded by NO
--    migration at all. Well-formatted SQL is what breaks this.
--
-- Corrections use the `SET value         = '` / `WHERE key           =` spacing
-- exactly, for the same reason: `corrected_value_in` searches for that shape.
--
-- The apostrophe here is the ASCII one, escaped `''`, because that is what every
-- other row in this store uses -- there is not one typographic apostrophe in any
-- migration. The mockup draws curly ones; they are normalised deliberately.
--
-- No answer, note, flag or change-log row is read or written by this file.

-- ─────────────────────────────────────────────────────────────────────────────
-- NEW ROWS — the picker, and what each side's list says about itself
-- ─────────────────────────────────────────────────────────────────────────────

-- The picker button: a side's name, and how many questions are on that side.
--
-- Domain note: the count is on the BUTTON and not on a line of its own, because
-- the question it answers -- "how much is behind this tab" -- is a question about
-- the tab. The heading's old two-sided count line is retired by the same change:
-- it printed these same two numbers a second time, three inches away.
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('practice_deck_side_tab_template',
    '{side} · {n}',
    'text',
    '{side} · {n}',
    NULL, NULL,
    'One button of the deck''s side picker. {side} is that side''s own title -- practice_who_george_title or practice_who_chuck_title, the same two rows the practice bar reads -- and {n} is how many questions that side holds. Domain note: the picker deliberately reuses the bar''s titles rather than seeding a second pair, so the two controls three inches apart can never come to disagree about what a side is called.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

-- What the defense's list is, said once, above it.
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('practice_deck_defense_countline',
    'The defense''s cross — top to bottom, the order they will press her in.',
    'text',
    'The defense''s cross — top to bottom, the order they will press her in.',
    NULL, NULL,
    'The line under the side picker when the defense''s side is showing. Domain note: it says the ORDER MEANS SOMETHING. This list was interleaved with Chuck''s until 2026-08-23, and a reader who remembers that needs telling that what she is now reading top to bottom is the sequence she will actually face.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

-- What Chuck's list is. It has two halves and they do different jobs, so the
-- line names both -- a reader who takes the redirects for more opening questions
-- has misread the half of the deck that exists to repair the other side.
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('practice_deck_chuck_countline',
    'Chuck''s questions — the direct tells the jury the story in order; the redirects repair the defense''s questions.',
    'text',
    'Chuck''s questions — the direct tells the jury the story in order; the redirects repair the defense''s questions.',
    NULL, NULL,
    'The line under the side picker when Chuck''s side is showing. Domain note: it names BOTH halves because they are answered differently -- a direct is told, a redirect is a repair to a question the defense has just asked -- and the two run consecutively in one list.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

-- The first half of Chuck's list. Its sibling `practice_redirects_subheader`
-- already existed and is corrected below; this is the label that was missing,
-- which is why the directs ran under no heading at all.
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('practice_directs_subheader',
    'The direct — the story, in order',
    'text',
    'The direct — the story, in order',
    NULL, NULL,
    'The heading above Chuck''s direct questions in his half of the deck list. Domain note: it is shown only when the list actually holds both kinds -- a heading above the only section on screen labels nothing.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

-- ─────────────────────────────────────────────────────────────────────────────
-- CORRECTION — one row that named an internal database value on screen
-- ─────────────────────────────────────────────────────────────────────────────

-- "(dealt in Mixed)" goes. `mixed` is a value in the `who` column of a table no
-- one outside this codebase has seen, and the parenthesis put it on Marie''s
-- screen as though it were a place she could go. It described the interleave,
-- which this change removes, so the sentence was about to be false as well.
--
-- The wording is squared with its new sibling above: both name the half of the
-- deck they head and what that half is FOR.
UPDATE app_settings
SET value         = 'The redirects — after the defense''s questions',
    default_value = 'The redirects — after the defense''s questions',
    meaning       = 'The heading above Chuck''s redirects in his half of the deck list. Domain note: this read "Redirects — asked after the defense''s questions (dealt in Mixed)" until 2026-08-23. "Mixed" was a value in the sessions table''s `who` column, printed on screen as though it named somewhere a person could go, and it described an interleaved list that no longer exists.',
    updated_at    = NOW(),
    updated_by    = 'migration'
WHERE key           = 'practice_redirects_subheader';
