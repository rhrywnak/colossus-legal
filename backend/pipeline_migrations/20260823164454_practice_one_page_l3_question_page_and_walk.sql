-- practice_one_page_l3: the question page, the critique, and the practice walk.
--
-- Target: pipeline database (colossus_legal_v2)
--
-- Mockup v7 views 2 through 7. Four strings are NOT here because they already
-- exist and are reused: `practice_answer_label`, `practice_answer_button`,
-- `practice_back_label` and `practice_points_to_label`.
--
-- Format rules as ever — key immediately after `VALUES (`, value ONE quoted
-- literal on ONE line. See `src/domain/wording_tests.rs` above `seeded_value_in`
-- for why, and for the rule that subsumes it.
--
-- No answer, note, flag or change-log row is read or written.

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_read_working_label',
    'Reading your answer',
    'Reading your answer',
    'string',
    'The heading of the critique block WHILE the read is running. Present from the moment she presses Answer — Roman''s defect #1 of 2026-08-20 was that nothing appeared until the read returned, so the page looked inert and she pressed again.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_read_usually_quick',
    'Usually a few seconds.',
    'Usually a few seconds.',
    'string',
    'Under the waiting block for the first ten seconds.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_read_still_working',
    'Still working — your answer is already saved either way.',
    'Still working — your answer is already saved either way.',
    'string',
    'Replaces the line above after ten seconds. Domain note: the SAVED half is the fact she needs while waiting, and it is true from the moment the row was written — her answer is the first write and the read is the second.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_read_stop_waiting',
    'Stop waiting',
    'Stop waiting',
    'string',
    'Abandons the READ, never the answer. Her words are already on disk when this appears.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_read_why_label',
    'Why',
    'Why',
    'string',
    'The label on the critique''s reasoning part. Withheld when the part is empty, which is legitimate.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_read_pointers_label',
    'What to do instead',
    'What to do instead',
    'string',
    'The label on the critique''s list of pointers.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_read_source_missing',
    'this source was not sent — report it',
    'this source was not sent — report it',
    'string',
    'Shown where a critique cites a key with no source behind it. Domain note: it should be impossible, because the read refuses a key it was not sent. It is SHOWN rather than hidden precisely because hiding it would hide the failure the source list exists to expose.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_read_unreviewed',
    'Chuck has not reviewed this.',
    'Chuck has not reviewed this.',
    'string',
    'At the foot of a critique. The machine''s judgement is not an attorney''s.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_read_wrong_label',
    'This is wrong →',
    'This is wrong →',
    'string',
    'Flags a misbehaving read. Domain note: this is NOT Chuck''s feedback path — it is Roman''s only signal that the read is going wrong, which is what tunes the prompt. Nobody reads it on a schedule; it is the difference between a bad read being noticed and being lived with.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_earlier_versions_template',
    '▸ {n} earlier versions',
    '▸ {n} earlier versions',
    'string',
    'One quiet line above the answer box. Earlier versions are NOT editable and she never has to open it. Domain note: a version is a change she made — pressing Answer twice on unchanged text re-reads and writes nothing.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_earlier_version_one',
    '▸ 1 earlier version',
    '▸ 1 earlier version',
    'string',
    'The same line when there is exactly one. A separate row rather than a plural rule in code: pluralisation is wording, and English is not the only language this could ever hold.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_your_answer_dated_template',
    'Your answer · {when}',
    'Your answer · {when}',
    'string',
    'Heads her answer in practice mode, with the day she wrote it.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_show_answer_label',
    'Show my answer',
    'Show my answer',
    'string',
    'Reveals what she wrote, in practice mode. Domain note: practice makes NO model call and NO write of any kind.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_next_question_label',
    'Next question ▸',
    'Next question ▸',
    'string',
    'Moves to the next question in the walk. Domain note: this is ALSO how she skips — there is no skip control, because there is nothing to explain.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_change_answer_label',
    'Change this answer',
    'Change this answer',
    'string',
    'Hands her from the practice walk to the question page.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_practice_counter_template',
    'PRACTICE · {side} · {n} OF {m}',
    'PRACTICE · {side} · {n} OF {m}',
    'string',
    'The line above the question in practice mode.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_practice_say_aloud',
    'Say your answer out loud.',
    'Say your answer out loud.',
    'string',
    'In the greyed area before she reveals. Standing rule of 2026-08-19: no control on a practice page is dim and silent — the grey area CARRIES its instruction.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_practice_then_press_template',
    'Then press {label} to see what you wrote.',
    'Then press {label} to see what you wrote.',
    'string',
    'The second line of the greyed area, naming the control by its stored label so renaming the button cannot leave this sentence naming one that does not exist.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_practice_skip_hint',
    'To skip it, just press Next question.',
    'To skip it, just press Next question.',
    'string',
    'Under the practice buttons. There is no skip control; this says so once.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_practice_end_title',
    'That''s all of them.',
    'That''s all of them.',
    'string',
    'The end of a practice walk.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_practice_end_count_template',
    '{n} questions from {side}.',
    '{n} questions from {side}.',
    'string',
    'Under that heading.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_practise_again_label',
    'Practise them again',
    'Practise them again',
    'string',
    'Restarts the walk from the top.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_practice_none_answered',
    'There is nothing to practise yet — practice walks the questions you have already answered.',
    'There is nothing to practise yet — practice walks the questions you have already answered.',
    'string',
    'Shown when the chosen side has no answered questions. Domain note: practising an answer she has not written is nothing to practise, so the walk offers only answered questions, and the empty case says why rather than showing an empty screen.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_deck_question_missing',
    'That question is no longer in this deck.',
    'That question is no longer in this deck.',
    'string',
    'Shown when the question page is opened for a question the deck no longer holds. Domain note: a real state, not a defect — Chuck deletes questions and a bookmark outlives them. Her ANSWERS to it are untouched; the mechanism behind Delete is a hide.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_read_fallible',
    'This read is generated and can be wrong. If something here looks wrong to you, tell Chuck.',
    'This read is generated and can be wrong. If something here looks wrong to you, tell Chuck.',
    'string',
    'Under the three-part critique. Domain note: it is a LINE and not a control — nothing clickable, nothing that writes. ⚑ WHY THIS SURVIVED WHEN FIVE OTHER UNREAD SURFACES WERE DELETED THE SAME DAY: everywhere else silence was safe. Here a machine tells Marie things about her own case in a confident three-part shape with citations underneath, and SILENCE READS AS AUTHORITY. This is the only thing on that screen saying the read is fallible. It renders only where a critique renders — not on the one-sentence fallback, and not while the read is running.',
    'migration')
ON CONFLICT (key) DO NOTHING;
