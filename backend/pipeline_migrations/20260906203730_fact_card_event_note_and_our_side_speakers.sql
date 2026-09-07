-- fact_card_event_note_and_our_side_speakers — INTEGRATION 2026-09-07, rulings R2 and R3
--
-- Created: 2026-09-06 20:37:30
-- Target: pipeline database (colossus_legal_v2)
--
-- Two small corrections that only make sense once PROOF_MATRIX_v2 and
-- FACT_CARD_v2 sit on one branch. Both were raised in
-- CC_REPORT_FACT_CARD_v2 and ruled on 2026-09-07.
--
--   R2 — a pick's `reason` had nowhere to go and was reported unstored. It now
--        rides the include event in `scenario_fact_card_events`, which needs a
--        `note` column to carry it.
--   R3 — `rehearsal_our_side_speakers` was seeded with Marie alone. Our side is
--        Marie and her counsel; every one of them is a name whose statements
--        must NOT be put to her as an accusation.
--
-- No explicit BEGIN/COMMIT: sqlx already runs each migration file in its own
-- transaction, and 123 of the 125 files here rely on that. The pair also breaks
-- the integration proof, which runs all three unmerged files inside ONE
-- `BEGIN … ROLLBACK` — an inner `COMMIT` there would commit the outer
-- transaction and APPLY them to DEV.

-- ─── R2. The ledger learns to carry a sentence ────────────────────────────────
--
-- Its sibling ledger `evidence_allegation_ruling_events` has carried a `note`
-- since PROOF_MATRIX_v2 §2, for the same purpose and with the same nullability.
-- The two tables record different human acts on the same case and should not
-- differ in what they can remember about them.
--
-- Domain note: this is NOT a card field. §1's five sentences are the witness's
-- own words; a ranker's "why I picked this" is a sentence ABOUT the choice, and
-- putting it on the card would read to Marie as something she is meant to say.
-- The ledger is where the account of an act belongs.
--
-- Nullable, and NULL is the ordinary state: an ordinary field edit carries no
-- note, and requiring one would force every writer to invent a sentence.

ALTER TABLE scenario_fact_card_events
    ADD COLUMN IF NOT EXISTS note TEXT;

COMMENT ON COLUMN scenario_fact_card_events.note IS
    'Optional sentence explaining the ACT, not the card: a loader''s reason for '
    'picking this fact, or a human''s remark on an edit. NULL is the ordinary '
    'state. Never rendered on the card — the five card fields are the witness''s '
    'own words, and this is somebody talking about them.';

-- ─── R3. Our side is Marie and her counsel ────────────────────────────────────
--
-- `is_ours` compares the graph's speaker to this list case-folded and WHOLE, so
-- each way a name appears in the record needs its own token — "Jeffrey Sharp"
-- does not match a statement recorded as "Jeff Sharp". Both forms are listed,
-- and the same for Douglas/Doug Buk and Charles M./Charles Penzien.
--
-- Domain note: the failure direction is why this list names OUR side rather than
-- the opposition. A name missing here shows Marie an EXTRA card to read; a name
-- wrongly here hides one she has to answer on the stand. So the list is short,
-- explicit, and only ever grown deliberately.
--
-- `default_value` moves with `value`: the default is what a reset restores, and
-- a reset that put Marie back on her own would silently reinstate the defect
-- this row is fixing.

-- The value is ONE literal on ONE line, in the house `SET value         = '`
-- alignment, and not a two-line SQL string continuation. That is not style: the
-- drift test `the_fixtures_carry_the_values_the_migration_actually_seeds` parses
-- this file with `corrected_value_in`, which reads to the closing quote of the
-- FIRST literal — a continuation would have it report half our side and pass.

UPDATE app_settings
   SET value         = 'Marie Awad, Jeffrey Sharp, Jeff Sharp, Douglas Buk, Doug Buk, Charles M. Penzien, Charles Penzien, Paul Williams, Shaw',
       default_value = 'Marie Awad, Jeffrey Sharp, Jeff Sharp, Douglas Buk, Doug Buk, Charles M. Penzien, Charles Penzien, Paul Williams, Shaw',
       meaning       = 'The speakers whose statements are OURS, comma-separated: Marie '
                       'and her counsel, each in every form the record spells them. '
                       'Everything else — including a document with no recorded '
                       'speaker — is treated as the other side or the court on the '
                       'rehearsal page, and appears under The Accusation with her '
                       'answer beneath it. A name not listed here shows an extra card, '
                       'never one fewer.',
       updated_at    = now(),
       updated_by    = 'migration:integration_2026_09_07_ruling_r3'
 WHERE key           = 'rehearsal_our_side_speakers';

-- ─── The end-state assertion (CLAUDE.md rule 25a) ─────────────────────────────
--
-- A statement matching zero rows is SILENT in Postgres, and the old value keeps
-- being served — which here would mean Marie's own counsel put to her as the
-- other side, on a page whose whole job is to rehearse what she must answer.
--
-- Asserted on the END STATE rather than on the update count, so the file is safe
-- to re-run: a second run changes nothing and still passes.

DO $$
DECLARE
    speakers  text;
    tokens    int;
    note_cols int;
BEGIN
    SELECT value INTO speakers
      FROM app_settings
     WHERE key = 'rehearsal_our_side_speakers';

    IF speakers IS NULL THEN
        RAISE EXCEPTION
            'rehearsal_our_side_speakers is missing — 20260906161201 must run first';
    END IF;

    -- Nine names, comma-separated. Counted from the stored string rather than
    -- trusted from the UPDATE above, so a typo that merged two names into one
    -- token fails here instead of quietly shortening our side by one.
    tokens := array_length(string_to_array(speakers, ','), 1);
    IF tokens <> 9 THEN
        RAISE EXCEPTION
            'expected 9 our-side speakers after this file, found %: %', tokens, speakers;
    END IF;

    IF position('Jeff Sharp' in speakers) = 0
       OR position('Doug Buk' in speakers) = 0
       OR position('Charles Penzien' in speakers) = 0 THEN
        RAISE EXCEPTION
            'the short forms of counsel''s names are missing from %', speakers;
    END IF;

    SELECT count(*) INTO note_cols
      FROM information_schema.columns
     WHERE table_name = 'scenario_fact_card_events'
       AND column_name = 'note';

    IF note_cols <> 1 THEN
        RAISE EXCEPTION
            'scenario_fact_card_events.note was not added (found % columns)', note_cols;
    END IF;

    RAISE NOTICE
        'integration 2026-09-07: ledger note added, our side is % speakers', tokens;
END $$;
