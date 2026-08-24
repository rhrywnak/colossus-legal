-- practice_one_page_l3: the line under a one-sentence critique.
--
-- Target: pipeline database (colossus_legal_v2)
--
-- ## Why this row exists — measured, not anticipated
--
-- Of 14 stored answers on DEV (2026-08-23), 12 carry `read_text` and only TWO
-- carry the three parts T1 introduced in .404. So the structured critique is the
-- MINORITY rendering today and the plain sentence is the normal one.
--
-- Chuck reads answer 3 and gets three parts with citations, then answer 4 and
-- gets one sentence, with nothing on the page explaining the difference. That
-- reads as breakage.
--
-- ## Domain note: it points at the FIX, and the fix already exists
--
-- Pressing Answer on unchanged text re-requests the read and attaches the parts
-- to the SAME row — no new version, because a version is a change she made and
-- not a button she pressed. So every older answer upgrades in one press. The
-- line says so; it is not an apology and not a history lesson.
--
-- ## ⚑ ARCHITECT'S ADDITION — Roman may strike it
--
-- If he wants the plain sentence bare, DELETE the one render site in
-- `PracticeCritique` and say so in the report. The row can stay unread.
--
-- Format rules as ever: the key sits immediately after `VALUES (`, and the value
-- is ONE quoted literal on ONE line. See `src/domain/wording_tests.rs` above
-- `seeded_value_in` for why that matters and what it costs when ignored.
--
-- No answer, note, flag or change-log row is read or written.
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('practice_read_plain_hint',
    'This is an older read. Press Answer again for the fuller one.',
    'text',
    'This is an older read. Press Answer again for the fuller one.',
    NULL, NULL,
    'Shown under a critique that is one sentence rather than three parts. Domain note: answers written before the three-part read shipped carry only a composed sentence, and without this line the two renderings read as breakage. Pressing Answer on unchanged text re-runs the read and attaches the parts to the same answer row — no new version — so the line points at a fix that already works.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;
