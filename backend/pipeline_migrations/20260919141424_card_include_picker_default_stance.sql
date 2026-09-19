-- card_include_picker_default_stance: the Include picker's prefill becomes a row
--
-- Created: 2026-09-19 14:14:24
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_CARDTRIAGE_SPLIT_v1, thing 3. THE ONE MIGRATION this task writes.
-- Roman's ruling of 2026-09-17, recorded in the doc block this change deletes.
-- =============================================================================
--
-- ## What this replaces
--
-- `const DEFAULT_STANCE: CardFactStance = "supports"` in
-- `frontend/src/components/includePickerModel.ts`. The architecture gate called
-- it a Standing-Rule-2 value and was right: 802:142 is a property of the
-- evidence gathered so far, not a logical invariant, and another case could
-- reasonably open on "Helps them".
--
-- It waited for this task because of WHERE IT IS APPLIED. The picker is opened
-- from `cardTriage`'s reducer, which is pure and has no settings snapshot, so
-- reading a stored default meant threading it through the queue — the same seam
-- the prompt-machine extraction is about. Roman ruled the row ships with the
-- split, where the threading belongs.
--
-- The constant is DELETED, not relocated: the stance now arrives on the picker's
-- `open` action, so no module holds a default of its own.
--
-- ## The vocabulary is supports | rebuts
--
-- ⚑ Not `supports | contradicts`. Measured on 2026-09-19 against both sides:
--
--   backend   `domain::fact_card::CardStance::ALL` = [Supports, Rebuts]
--   frontend  `services/scenarioCards.ts` — `type CardFactStance = "supports" | "rebuts"`
--
-- `rebuts` is the token the GRAPH's own `r.stance` carries (802 `supports` /
-- 142 `rebuts`, measured) and the word the 59 drafted cards on disk use. The
-- assertion below checks the seeded value against exactly those two spellings,
-- so a prettier synonym typed on the Settings page is refused HERE rather than
-- at the boot that follows it.
--
-- (`contracts/fact_action_include.json` is NOT the source: it pins ONE example
-- body — the exact bytes both languages test against — and lists no
-- alternatives. Checked.)
--
-- ## Why seeded `supports`
--
-- It is what the constant said, so this migration changes no behaviour on the
-- day it lands. That is the point: the value moves out of the build without
-- moving on screen, and the next change to it is a Settings edit.
--
-- ## Idempotent
--
-- ON CONFLICT (key) DO NOTHING; an operator who has already chosen `rebuts`
-- keeps it, and the assertion checks the END state either way (rule 25a).

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('card_include_picker_default_stance', 'supports', 'text', 'supports',
     NULL, NULL,
     'Which way the Include picker is pre-set when it opens on a candidate card: '
     '''supports'' (the card helps the accusation stand) or ''rebuts'' (it cuts '
     'against it). Only those two spellings; the backend refuses to start on any '
     'other. The human always sees which is chosen and can change it before '
     'saving, so this only decides which way the common case opens.',
     'api::scenario_cards → frontend cardPrompts (the Include picker''s prefill)',
     NOW(), 'migration')
ON CONFLICT (key) DO NOTHING;

-- ─── The END-state assertion (CLAUDE.md rule 25a) ────────────────────────────
--
-- Two silent failures are possible and Postgres reports neither: an INSERT that
-- collided with an existing row and did nothing, and a value outside the
-- vocabulary. The second is the one worth the SQL: the backend refuses to boot
-- on it, and failing here stops the deploy one step earlier with the two legal
-- spellings named.

DO $$
DECLARE
    configured TEXT;
BEGIN
    SELECT value INTO configured
      FROM app_settings
     WHERE key = 'card_include_picker_default_stance';

    IF configured IS NULL THEN
        RAISE EXCEPTION
            'card stance: card_include_picker_default_stance is missing. The '
            'backend declares it at boot and refuses to start without it.';
    END IF;

    IF configured NOT IN ('supports', 'rebuts') THEN
        RAISE EXCEPTION
            'card stance: card_include_picker_default_stance is %, which is not a '
            'stance. A card cuts one of two ways — type supports or rebuts exactly. '
            'The backend refuses to start on anything else.', configured;
    END IF;
END $$;
