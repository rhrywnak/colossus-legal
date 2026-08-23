-- practice_one_page_l2_print: codes leave the paper.
--
-- Created: 2026-08-23 14:51:27
-- Target: pipeline database (colossus_legal_v2)
--
-- ## ⚑ FORMAT RULES (both learned the hard way — see the L1 and L2 migrations)
--
-- And note what this very comment IS: by quoting the format below, it puts the
-- string a parser hunts for EARLIER in the file than the statement it wants.
-- That is not a reason to stop documenting the rule beside the rule — it is a
-- reason for every parser to strip comments first. The rule, and the three
-- times it bit on 2026-08-23, are stated once in `src/domain/wording_tests.rs`
-- above `seeded_value_in`.
--
-- Corrections use the `SET value         = '` / `WHERE key           =` spacing
-- exactly: `corrected_value_in` searches for that shape. Values are ONE quoted
-- literal on ONE line.
--
-- ## What is NOT here, and why
--
-- The sheet footer is deleted in code, not re-worded here.
-- `practice_print_footer_template` and `practice_print_sheet_number_template`
-- keep their rows: they are declared to the boot loader, and dropping a row a
-- running build reads is a REFUSAL TO START. They simply stop being rendered.
-- Nothing then trails a sheet's content, so no sheet can end on a page carrying
-- only a footer — which is the defect the `break-before: avoid` rule was
-- supposed to fix and measurably did not.
--
-- No answer, note, flag or change-log row is read or written.

-- The redirect's antecedent line loses the code it pointed with.
--
-- Domain note: `{key}` was the whole reason a code appeared on paper — "After
-- the defense asks g1: …". With codes gone from the screen and the sheet alike,
-- the QUOTED QUESTION is the identification, and it is the better one: Chuck
-- reads the words he is repairing rather than a label he has to look up.
UPDATE app_settings
SET value         = 'After the defense asks: {question}',
    default_value = 'After the defense asks: {question}',
    meaning       = 'Printed above a redirect, quoting the defense question it repairs. Domain note: it carried a {key} until 2026-08-23, when question codes left both the screen and the paper — the quoted question is the identification now, and it is the one Chuck can act on without a lookup.',
    updated_at    = NOW(),
    updated_by    = 'migration'
WHERE key           = 'practice_print_after_template';
