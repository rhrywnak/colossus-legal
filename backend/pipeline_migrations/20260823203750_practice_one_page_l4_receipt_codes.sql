-- practice_one_page_l4_receipt_codes: the code comes out of S-5's receipts.
--
-- Target: pipeline database (colossus_legal_v2)
--
-- ## What this is, and why the YAML alone is not enough
--
-- "Chuck's redirect after G3 — point 1." printed a question CODE on paper. Codes
-- left the screen and the printed sheets on 2026-08-23; this one survived inside
-- authored prose, where no code change reached it.
--
-- The prose lives in `backend/practice_decks/S-5.yaml`, which is the source and
-- is corrected in the same commit. But the YAML only reaches the database on a
-- re-seed, and Chuck prints from TODAY's rows. A YAML-only fix would be correct
-- and invisible; a migration-only fix would be wiped by the S-5 reset Roman is
-- holding. Both halves, therefore.
--
-- ## ⚑ Why this is permitted under the task's hard constraint
--
-- §2 forbids changing a pre-existing column value. That constraint protects
-- MARIE'S WORK — answers, notes, flags, change-log rows. This is authored
-- content WE wrote, and Roman ruled it expressly permitted.
--
-- ## ⚑ Written to be a NO-OP on already-corrected rows
--
-- It matches on the OLD text rather than assuming the pre-reset state, so
-- running it after a re-seed changes nothing rather than corrupting prose that
-- is already right. `regexp_replace` with an anchored pattern, guarded by a
-- WHERE that only selects rows still carrying the code.
--
-- ## ⚑ TWO FORMS, AND THE SECOND WAS FOUND BY THE GUARD, NOT BY GREP
--
-- S-5 writes "Chuck's redirect after G1 — …" (uppercase). S-7 writes
-- "Repairs g1 with …" (lowercase). A hand-built inventory searching for
-- "redirect after" found the first five and reported the deck clean; the
-- self-deriving, case-insensitive guard in `deck_shipped_tests` found the other
-- two immediately. That is the whole argument for deriving the forbidden strings
-- from the deck's own keys instead of writing a pattern.
--
-- Measured 2026-08-23: seven coded receipts — five in S-5, two in S-7, none in
-- S-6.
--
-- ## Domain note: S-6 IS DELIBERATELY UNTOUCHED
--
-- Its five redirects read "after the generalization", "after the half-truth",
-- "after the authority borrow", "after the echo", "after the braid". Those name
-- TACTICS, not deck keys — they point at something a reader can use, and the
-- ruling was about codes that point at nothing. Measured 2026-08-23: five coded
-- receipts across all three decks, all of them S-5's.
--
-- No answer, note, flag or change-log row is read or written.
-- S-5's form. Anchored, and the WHERE only selects rows still carrying a code,
-- so a re-run after the reset is a no-op rather than a corruption.
UPDATE practice_questions
SET receipt = regexp_replace(receipt, '^(Chuck''s redirect) after G[0-9]+ —', '\1 —')
WHERE receipt ~ '^Chuck''s redirect after G[0-9]+ —';

-- S-7's form. Case-insensitive on the key, because the two decks disagree about
-- case and a pattern that assumed one would leave the other's rows behind —
-- which is exactly how these two were missed the first time.
UPDATE practice_questions
SET receipt = regexp_replace(receipt, '^Repairs [gcrGCR][0-9]+ with', 'Repairs it with')
WHERE receipt ~ '^Repairs [gcrGCR][0-9]+ with';
