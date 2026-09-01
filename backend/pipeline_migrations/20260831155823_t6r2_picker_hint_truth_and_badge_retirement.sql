-- t6r2_picker_hint_truth_and_badge_retirement — one lie corrected, one withdrawn.
--
-- Created: 2026-08-31
-- Target: pipeline database (colossus_legal_v2)
--
-- Roman's two rulings on the T6 report, 2026-08-31 (round two). Both are about
-- the same thing: a string that makes a claim the record does not support.
--
-- ## ⚑ 1. THE HINT DESCRIBED A CONTROL THAT HAS NEVER EXISTED
--
-- `chronology_subsets_picker_hint` has said "drag the number to change the
-- story order" since task 2. The number has never been draggable. T2 shipped
-- ▲▼ buttons instead — a drag with no keyboard path is a control Marie cannot
-- use on a trackpad mid-question — and the sentence was never brought along.
-- T6.2 put the arrows directly under the sentence that misdescribes them, which
-- is how it was finally noticed. An UPDATE, not a new row: the hint is the same
-- hint, and a second row saying almost the same thing is how a store rots.
--
-- ## ⚑ 2. THE ⚑ BADGE MADE A FALSE CLAIM ABOUT THE RECORD
--
-- This RETIRES `chronology_subsets_date_to_confirm_badge` and reverses the T4
-- decision that created it.
--
-- The badge was to mark a date Roman is chasing — the Milster handoff, entered
-- from recollection. Nothing in the data records that, so T4 badged the nearest
-- thing the data does know: `approximate = true`. On "The $50,000" that happened
-- to be exactly the two rows the mockup marks, so it read correctly and shipped.
--
-- T6.2 put the same badge on the picker, which draws the WHOLE chronology, and
-- the cost became visible: four of thirty-one events wore "date to confirm",
-- including "Estate Auction Held" and "Motions for Default Filed", which nobody
-- has ever flagged. The ⚑ has a specific meaning in this case, and spreading it
-- across every approximate date destroys the signal it exists to carry. Roman's
-- ruling: a badge that makes a false claim about the record is worse than no
-- badge.
--
-- What SURVIVES is everything the data can actually support — amber approximate
-- dates, "~ Apr 2009", `chronology_subsets_precision_month_label`,
-- `chronology_subsets_precision_year_label`. Those say the date is approximate,
-- which is true. Only the claim that somebody must go and CONFIRM it comes off.
--
-- The window footer loses its "· n ⚑" half with it and reads "15 events" again.
-- `chronology_subsets_window_footer_events_template` is untouched and still
-- carries "{count} events" — it is asserted below precisely because "the footer
-- loses its second half" is one edit away from being misread as "the footer row
-- is retired".
--
-- Retired the way the T4 footer row and the two T3 rows were: DELETEd here and
-- removed from the keys registry, the domain struct, the wire DTO and the
-- fixture, so `no_declared_word_is_left_with_no_asker` stays green.

-- ── 1. the hint tells the truth ─────────────────────────────────────────────

UPDATE app_settings
   SET value = 'Tick an event to add it. Order defaults to date; use the order arrows to change the story order. The note is optional — one line on why this event is in the story.',
       default_value = 'Tick an event to add it. Order defaults to date; use the order arrows to change the story order. The note is optional — one line on why this event is in the story.',
       meaning = 'The line above the picker in the Add/Edit subset modal (mockup Screen 3). Domain note: it names the three things a row can do and nothing else, because a subset NEVER edits an event — the dates in the picker are read-only text (T6.2, defect D1/D9). Corrected 2026-08-31: it used to say "drag the number", describing a control that has never existed. T2 shipped ▲▼ order buttons instead, deliberately — a drag with no keyboard path is a control Marie cannot use on a trackpad mid-question — and this sentence was not brought along. If the arrows are ever replaced by a drag, this row changes with them.',
       updated_at = NOW(),
       updated_by = 'migration'
 WHERE key = 'chronology_subsets_picker_hint';

-- ── 2. the badge is retired ─────────────────────────────────────────────────

DELETE FROM app_settings
 WHERE key = 'chronology_subsets_date_to_confirm_badge';

-- ── the END-state assertions (CLAUDE.md rule 25a) ───────────────────────────
--
-- A statement matching zero rows is silent in Postgres and the old value keeps
-- being served. This migration UPDATEs and DELETEs, and BOTH are silent when
-- they match nothing — an UPDATE that found no row leaves the lie on screen and
-- says nothing about it, which is the exact failure 25a exists to catch.

DO $$
DECLARE
    n INTEGER;
BEGIN
    -- ── the hint is the NEW sentence ────────────────────────────────────────
    -- Asserted on the new clause and against the old one, not on a row count:
    -- the row existed before and exists after, so counting it proves nothing
    -- about whether the UPDATE actually landed.
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key = 'chronology_subsets_picker_hint'
       AND value LIKE '%use the order arrows%'
       AND value NOT LIKE '%drag the number%';
    IF n <> 1 THEN
        RAISE EXCEPTION
            'the picker hint must now say "use the order arrows" and must no longer '
            'say "drag the number" — it describes the ▲▼ buttons the picker has, '
            'not a drag it has never had (matched % rows)', n;
    END IF;

    -- ── the badge is GONE ───────────────────────────────────────────────────
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key = 'chronology_subsets_date_to_confirm_badge';
    IF n <> 0 THEN
        RAISE EXCEPTION
            'the date-to-confirm badge must be retired, % remain', n;
    END IF;

    -- ── what the ruling KEPT survives ───────────────────────────────────────
    -- The obvious misreading of "the badge comes off" is "the approximate-date
    -- wording comes off with it". It does not: "~ Apr 2009" and its caption say
    -- the date is APPROXIMATE, which is true and stays. And the footer row is
    -- not retired — the footer loses its "· n ⚑" half and still says
    -- "{count} events" from this row.
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key IN (
            'chronology_subsets_precision_month_label',
            'chronology_subsets_precision_year_label',
            'chronology_subsets_window_footer_events_template'
     );
    IF n <> 3 THEN
        RAISE EXCEPTION
            'the two precision captions and the footer events template must SURVIVE '
            'the badge retirement — they state what the data knows; found %', n;
    END IF;

    -- ── the whole chronology block is still blank-free ──────────────────────
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key LIKE 'chronology\_%' AND (value IS NULL OR btrim(value) = '');
    IF n <> 0 THEN
        RAISE EXCEPTION
            'a chronology row is blank; the boot loader would refuse to start (% rows)', n;
    END IF;
END $$;
