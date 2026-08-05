-- queue_counting_state_wording: the queue's summary while it is still counting
--
-- Created: 2026-08-05 12:09:00
-- Target: pipeline database (colossus_legal_v2, applied at backend boot by the
--         runtime sqlx::migrate::Migrator — forward-only, no down migration)
--
-- WHY (the beta.377 click-through, and the defect task 2.13 half-fixed)
--
--   The queue's collapsed header claims a state it has not measured. On DEV it
--   read "No candidates gathered yet" three lines below the scan panel's own
--   "148 candidates gathered", on the same screen.
--
--   Task 2.13 split "all ruled" from "empty pool" on the condition `total = 0`.
--   That was the wrong seam. There are THREE states, not two:
--
--       counts unknown   — nothing has measured the pool yet
--       pool is empty    — measured, and there is nothing in it
--       all ruled        — measured, full, and nothing is outstanding
--
--   Collapsing the first into either of the others produces a confident sentence
--   about a number nobody has read (Standing Rule 1). Before 2.13 that sentence
--   was "All candidates ruled" over 92 unruled candidates; after 2.13 it was
--   "No candidates gathered yet" over 148 gathered ones. Both were false; the
--   second was merely more obviously so.
--
--   The correct pattern was already eleven lines up in the same component: the
--   progress label renders "Counting candidates…" when `progress === null`
--   rather than "0 of 0 ruled", with a comment citing Standing Rule 1. This row
--   gives the SUMMARY the same honest third state — and, because it is a stored
--   row rather than a literal, it also retires the hardcoded string that label
--   has been carrying since it was written.
--
-- NOT the mount latch. The reason the counts stay unknown is that `CardQueue`
--   only reports them while it is MOUNTED, and it is not mounted while the
--   region is collapsed — so a collapsed queue can never learn its own counts.
--   That is pre-existing, now diagnosed, and filed as its own task. This
--   migration and its fix make the header HONEST about not knowing; they do not
--   make it know.
--
-- FORWARD-ONLY: no down migration. A correction is a further forward migration,
--   never an edit to this file once applied (ON CONFLICT DO NOTHING means editing
--   the VALUES list changes nothing on an environment that has already run it).

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('queue_counting_summary', 'Counting candidates…', 'text', 'Counting candidates…',
     NULL, NULL,
     'The queue''s line while it is still counting — shown before anything has '
     'measured the pool, and whenever the counts are not known. Deliberately '
     'different from both "no candidates" and "all ruled": a queue that has not '
     'looked yet must not report either result.',
     NULL, now(), 'system (seed)')

ON CONFLICT (key) DO NOTHING;
