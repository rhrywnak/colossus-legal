-- build_403_labels_and_chuck_view: Ubuild_403_labels_and_chuck_view
--
-- Created: 2026-08-19 20:22:08
-- Target: pipeline database
--
-- TODO: Add your SQL here.


-- ─── §B · The three sides, named for what they DO ───────────────────────────
--
-- ## What was wrong with the old names
--
-- "George's side (cross)" named the OPPOSING COUNSEL. Marie is not preparing to
-- be questioned by a man called George — George is the fictional stand-in this
-- deck was drafted against, and on the stand she will be questioned by the
-- defense. A witness reading "George's side" has to translate before she can
-- use the screen, and the translation is the sort of thing that goes wrong under
-- pressure.
--
-- The new names say who is asking and, in a small grey term underneath, the word
-- the lawyers use for it. Marie reads the sentence; Chuck reads the term.
--
--   Chuck asks                 · direct
--   The defense asks           · cross
--   Chuck, after the defense   · redirect
--
-- Written in the alignment `domain::wording::tests::corrected_value_in` parses.
-- Get it wrong and the fixture test goes green while the store holds the old
-- words — the trap the scenario header's `Practice ▸` fell into on 08-19.
UPDATE app_settings
   SET value         = 'The defense asks',
       updated_at    = NOW(),
       updated_by    = 'migration'
 WHERE key           = 'practice_who_george_title';

UPDATE app_settings
   SET value         = 'Chuck asks',
       updated_at    = NOW(),
       updated_by    = 'migration'
 WHERE key           = 'practice_who_chuck_title';

UPDATE app_settings
   SET value         = 'Mixed',
       updated_at    = NOW(),
       updated_by    = 'migration'
 WHERE key           = 'practice_who_mixed_title';

-- The side-card descriptions keep their sense, re-worded to match the new names.
UPDATE app_settings
   SET value         = 'Built from what they actually said in the record — the attack, turned into a question.',
       updated_at    = NOW(),
       updated_by    = 'migration'
 WHERE key           = 'practice_who_george_detail';

UPDATE app_settings
   SET value         = 'The questions Chuck asks so you can tell it in your own words.',
       updated_at    = NOW(),
       updated_by    = 'migration'
 WHERE key           = 'practice_who_chuck_detail';

UPDATE app_settings
   SET value         = 'Both, in no fixed order — closest to the real day.',
       updated_at    = NOW(),
       updated_by    = 'migration'
 WHERE key           = 'practice_who_mixed_detail';

-- The row PILLS carry the same vocabulary. A row labelled "George's side" beside
-- a card headed "The defense asks" is the same translation problem one line down.
UPDATE app_settings
   SET value         = 'the defense',
       updated_at    = NOW(),
       updated_by    = 'migration'
 WHERE key           = 'practice_pill_george';

UPDATE app_settings
   SET value         = 'Chuck',
       updated_at    = NOW(),
       updated_by    = 'migration'
 WHERE key           = 'practice_pill_chuck';

UPDATE app_settings
   SET value         = 'the defense · a braid',
       updated_at    = NOW(),
       updated_by    = 'migration'
 WHERE key           = 'practice_pill_braid';


-- ─── §B · The three small grey terms ────────────────────────────────────────
--
-- Under each side card. Separate rows rather than baked into the titles: the
-- title is the sentence Marie reads and the term is the word Chuck reads, they
-- are set in different sizes and colours, and a single string carrying both
-- could not be styled as two things.
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('practice_who_george_term', 'cross', 'text', 'cross', NULL, NULL,
     'The lawyers'' word for what the defense does, small and grey under "The '
     'defense asks". Marie reads the sentence; Chuck reads the term.',
     'frontend PracticeStart', NOW(), 'migration'),
    ('practice_who_chuck_term', 'direct', 'text', 'direct', NULL, NULL,
     'The lawyers'' word under "Chuck asks".',
     'frontend PracticeStart', NOW(), 'migration'),
    ('practice_who_redirect_term', 'redirect', 'text', 'redirect', NULL, NULL,
     'The lawyers'' word for a repair question. Not a side card of its own — '
     'redirects ride with Chuck — but the deck list''s sub-header needs the term.',
     'frontend PracticeDeckList', NOW(), 'migration'),


-- ─── §C · The Chuck-view sub-header ─────────────────────────────────────────
--
-- A Chuck sitting deals his direct questions, then the redirects. Both wear
-- Chuck's pill because Chuck asks both — but a redirect is not a question he
-- opens with, it is one he asks to repair damage the defense just did, and a
-- list that ran the two together would read as ten opening questions.
--
-- So the list breaks them with a sub-header. It names when they are asked AND
-- where they are actually dealt, because in a Chuck-only sitting they are dealt
-- with no defense question in front of them — which is a rehearsal of the words,
-- not of the moment.
    ('practice_redirects_subheader',
     'Redirects — asked after the defense''s questions (dealt in Mixed)', 'text',
     'Redirects — asked after the defense''s questions (dealt in Mixed)', NULL, NULL,
     'Breaks Chuck''s direct questions from his redirects in the deck list. Says '
     'both when they are asked and where they are really dealt: a Chuck-only '
     'sitting drills the words, Mixed drills the moment.',
     'frontend PracticeDeckList', NOW(), 'migration');
