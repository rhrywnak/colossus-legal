-- ruling_acknowledgment_wording: every ruling says what it did
--
-- Created: 2026-08-08 17:16:30
-- Target: pipeline database (colossus_legal_v2)
--
-- Task CC_TASK_DEFER_FIX_AND_FACETS_v1, the reframed rider + D3a
-- (architect's confirmation of 2026-08-08).
--
-- ## The measured defect these five rows answer
--
-- On beta.385 the architect pressed Defer on a locked card in S-4 and reported
-- the feature dead: no dialog, no state change, no error, nothing. The database
-- says otherwise. The ruling landed at 20:51:04 — a `scenario_ruling_anchors`
-- row, a `scenario_fact_refs` row with `status = undecided` and the
-- server-composed `defer_reason`, and `source_run_id` populated. It was the most
-- recent ruling of any kind on DEV, four minutes after the last include.
--
-- Defer was never broken. It was SILENT — and worse than silent: ruling the card
-- made it human-touched, precedence stopped the projection proposing it, and the
-- card left the Proposed filter and vanished from the list. A correct write, a
-- correct filter, and a screen that read exactly like a dead button.
--
-- So the rule these rows encode is not "report errors". It is: EVERY ruling
-- action acknowledges itself, in success as well as failure, and a card that
-- leaves the list because of a ruling says so as it goes.
--
-- ## Forward-only, idempotent, no down migration — the house rule for a seed.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    -- ── The ruling landed ────────────────────────────────────────────────────
    ('card_ruling_saved_template', 'Saved — {code} is now {state}.',
     'text', 'Saved — {code} is now {state}.',
     NULL, NULL,
     'Shown when a ruling has been stored. Must keep {code} and {state} — on a '
     'list where thirty cards look alike, an acknowledgment that cannot name the '
     'card it is about is the same as no acknowledgment.',
     NULL, now(), 'system (seed)'),

    -- ── …and took the card out of the list ───────────────────────────────────
    --
    -- THE VANISH. This is the sentence whose absence made a working feature look
    -- dead: under the Proposed filter a ruled card is no longer proposed, so it
    -- correctly leaves the list — and a card disappearing with nothing said is
    -- indistinguishable from a click that did nothing.
    ('card_ruling_left_filter_template',
     '{code} has left the {filter} list — that is where ruled candidates go.',
     'text',
     '{code} has left the {filter} list — that is where ruled candidates go.',
     NULL, NULL,
     'Shown when a stored ruling takes the card out of the list the human is '
     'looking at. Must keep {code} and {filter}. Without this sentence a correct '
     'filter reads as a card that vanished for no reason.',
     NULL, now(), 'system (seed)'),

    -- ── The one-press defer on a locked card ─────────────────────────────────
    --
    -- A locked card carries the system's own reason, so Defer commits in one
    -- press with no prompt (prompting would ask the human to retype a sentence
    -- the server wrote). They should still be able to READ the sentence they
    -- just signed.
    ('card_defer_recorded_template', 'Deferred. The reason recorded is: {reason}',
     'text', 'Deferred. The reason recorded is: {reason}',
     NULL, NULL,
     'Shown when a locked card is deferred in one press and the system supplies '
     'the reason. Must keep {reason} — the whole point is that the human reads '
     'what was recorded on their behalf.',
     NULL, now(), 'system (seed)'),

    -- ── The ruling did not land ──────────────────────────────────────────────
    ('card_ruling_failed_template',
     '{code} could not be saved: {detail} The queue has been reloaded, so what you see now is what is stored.',
     'text',
     '{code} could not be saved: {detail} The queue has been reloaded, so what you see now is what is stored.',
     NULL, NULL,
     'Shown when a ruling could not be stored. Must keep {code} and {detail}. '
     'The second sentence matters as much as the first: the queue reconciles '
     'itself after a refusal, and a human who does not know that will not trust '
     'what is on the screen afterwards.',
     NULL, now(), 'system (seed)'),

    -- ── D3a: the locked card states its condition on its FACE ────────────────
    --
    -- The sentence already existed as the disabled buttons' tooltip. A condition
    -- a human can only find by hovering is one most humans never find — and this
    -- one is load-bearing, because it is also the promise that Defer will work.
    ('card_locked_condition_label', 'Include and Exclude are closed on this card:',
     'text', 'Include and Exclude are closed on this card:',
     NULL, NULL,
     'Introduces the standing reason a card cannot be included or excluded, '
     'stated on the card face rather than in a tooltip. The reason itself is '
     'composed per card by the backend and follows this label.',
     NULL, now(), 'system (seed)')

ON CONFLICT (key) DO NOTHING;
