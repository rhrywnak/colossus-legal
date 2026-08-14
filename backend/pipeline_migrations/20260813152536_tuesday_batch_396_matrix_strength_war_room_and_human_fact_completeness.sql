-- tuesday_batch_396: the matrix's strength vocabulary, the war room's own words,
-- and the sentence a twice-linked card speaks
--
-- Created: 2026-08-13 15:25:36
-- Target: pipeline database (colossus_legal_v2, applied at backend boot by the
--         runtime sqlx::migrate::Migrator — forward-only, no down migration)
--
-- ONE migration for the whole .396 batch, per the task's discipline. Three
-- unrelated-looking groups, and the reason they ride together is that they ship
-- together: a half-applied batch would boot-refuse on the missing half.
--
-- ## What is NOT here, and why
--
-- The batch's human-fact completeness item (P2c — edit-in-place, a weight tier,
-- and joining the facts list's order) was DROPPED for capacity, on the task's own
-- ruled drop order. Its two `scenario_human_facts` columns and its ten form
-- wording rows were drafted here and then removed rather than shipped ahead of
-- the code that uses them: a column nothing writes and a row nothing reads are
-- configuration debt, and the follow-up task can carry them with its own
-- migration, in one piece, where they can be reviewed against the code that
-- reads them.
--
-- ══════════════════════════════════════════════════════════════════════════════
-- GROUP 1 — THE PROOF MATRIX'S STRENGTH VOCABULARY (P1)
-- ══════════════════════════════════════════════════════════════════════════════
--
-- The matrix used to lead with one number per Element: how many distinct Evidence
-- items corroborate an Allegation bearing on it. That number counts an opposing
-- party's sworn admission and a qualified "to the best of my recollection" as the
-- same thing. It now leads with the STRONG count and keeps the raw figure beside
-- it as depth.
--
-- ## The three `matrix_tier_*_pairs` rows are the MAP, and they are why this is
-- ## configuration rather than code
--
-- Each entry is `statement_type+evidence_strength`. The key is a PAIR because
-- measurement said it had to be: on DEV, `sworn_party_admission` appears under
-- BOTH `admission` (21 items) and `partial_admission` (12 items), so strength
-- alone cannot separate a firm admission from a hedged one — which is the exact
-- distinction the headline exists to make.
--
-- The six pairs seeded below are every combination measured across every
-- CORROBORATES edge on DEV on 2026-08-13, mapped as Roman ruled that day:
--
--   STRONG  what the opposing side cannot dispute — their own admissions, and
--           the court's own findings and orders
--   HEDGED  sworn but qualified
--   OTHER   our own sworn affidavit testimony: visible and ranked, never hidden,
--           but not an opponent's admission and so not the headline
--
-- A pair NOT listed here gets no tier: it is still counted as approved, still
-- rendered in the drill-down, and carries no chip. That is deliberate — a new
-- document type must never make an item vanish from a proof surface, and must
-- never be silently promoted into the headline either.
--
-- ## No `attributed`/`recitation` row, on purpose
--
-- Pass 2 creates no finding-edge from a recitation, so that class is
-- structurally absent from this surface. Roman ruled 2026-08-13: do not render an
-- always-zero tier. If recitations ever start corroborating, they get a row then.
--
-- ══════════════════════════════════════════════════════════════════════════════
-- GROUP 2 — THE WAR ROOM'S OWN WORDS (P3b)
-- ══════════════════════════════════════════════════════════════════════════════
--
-- Ruling R2 of 2026-08-10 (task R2 §3) renamed the Trial Prep subtitle, renamed
-- the "Drafted / in review" tile to "Draft", and killed the "pattern analysis
-- pending" chip — "every one of these is a row value, so Roman can retune any of
-- them later with zero builds". The R2 batch shipped nine `scenario_identity_*`
-- rows and the scan-model row and NONE of §3.
--
-- Measured on DEV 2026-08-13: no war-room row exists in `app_settings` at all,
-- and both sentences are still compiled-in literals. So these rows were never
-- created — not created-and-bypassed — and this group is them, arriving three
-- days late.
--
-- The chip needs no row because it DIES: it was fed a field the backend hardcodes
-- to NULL, so it read "pattern analysis pending" on every card in every state, on
-- a page whose whole job is to distinguish what is ready from what is not.
--
-- ## ON CONFLICT DO NOTHING, as always
--
-- Makes the migration safely re-runnable, and means a value Roman has already
-- edited is never stamped back to ours. The consequence, learned in 2.12: editing
-- the VALUES list of an applied migration changes nothing anywhere — a correction
-- has to be its own later UPDATE, guarded on the old text.
--
-- FORWARD-ONLY: no down migration. A bad forward migration is corrected by a
-- FURTHER forward migration, never by editing this file once applied.

-- ─── The rows ────────────────────────────────────────────────────────────────

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    -- ═══ Group 1a: the tier MAP ═══════════════════════════════════════════════
    --
    -- KEY AND VALUE STAY ON ONE LINE for these three. `settings_store_tests` has
    -- its own seed scanner — stricter than the wording one — that matches
    -- `('key', '` and gives up if the value starts on the next line. Reflowing
    -- these for tidiness makes that test report the row as "not seeded by the
    -- migration", which is a confusing way to be told about a line break.

    ('matrix_tier_strong_pairs', 'admission+sworn_party_admission, court_finding+court_finding, court_order+court_order',
     'text',
     'admission+sworn_party_admission, court_finding+court_finding, court_order+court_order',
     NULL, NULL,
     'Which kinds of proof count as STRONG — the number the Proof Matrix leads '
     'with. Each entry is the kind of statement and its strength, joined by a '
     'plus sign, and entries are separated by commas. Strong means what the other '
     'side cannot dispute: their own sworn admissions, and the court''s own '
     'findings and orders. Anything not listed in one of the three lists still '
     'counts as approved and still shows in the drill-down — it simply carries no '
     'strength label.',
     NULL, now(), 'system (seed)'),

    ('matrix_tier_hedged_pairs', 'partial_admission+sworn_party_admission, partial_admission+sworn_party_evasion',
     'text',
     'partial_admission+sworn_party_admission, partial_admission+sworn_party_evasion',
     NULL, NULL,
     'Which kinds of proof count as HEDGED: sworn, but qualified — an admission '
     'that admits part and limits the rest, or an evasive answer the extraction '
     'still scored as supporting. True and usable; not what you lead with. Same '
     'format as the strong list.',
     NULL, now(), 'system (seed)'),

    ('matrix_tier_other_pairs', 'factual_assertion+sworn_testimony',
     'text',
     'factual_assertion+sworn_testimony',
     NULL, NULL,
     'Which kinds of proof count as OTHER: everything named but not in the first '
     'two lists — today, our own sworn affidavit statements. Ranked and always '
     'visible, but not an opponent''s admission, so it does not carry the '
     'headline. Same format as the strong list.',
     NULL, now(), 'system (seed)'),

    -- ═══ Group 1b: the matrix's words ═════════════════════════════════════════

    ('matrix_strong_column_label', 'Strong support', 'text', 'Strong support',
     NULL, NULL,
     'The heading over the Proof Matrix''s headline number — the count of proof '
     'the other side cannot dispute. This column used to be headed with a word '
     'for the raw total; it now makes a narrower claim, so it says so.',
     NULL, now(), 'system (seed)'),

    ('matrix_raw_approved_template', '· {count} approved', 'text',
     '· {count} approved',
     NULL, NULL,
     'The small print beside the headline, carrying the raw total the headline '
     'narrows from. {count} is replaced by that total and must stay in the text — '
     'without it the line makes a claim with no number attached. Nothing is '
     'hidden: the strong count is a narrower reading, and the wider one stays on '
     'screen next to it.',
     NULL, now(), 'system (seed)'),

    -- The VALUE literal stays on ONE line, however long, and only the `meaning`
    -- wraps. The disk/code consistency test (Rule 21) reads seeded values straight
    -- out of this file with a deliberately crude scanner that stops at the first
    -- closing quote — a value split across two adjacent SQL literals would be read
    -- as its first fragment, and the fixture would disagree with the row for a
    -- reason nobody could see. Same rule applies to every long value below.
    ('matrix_strong_hint',
     'Sworn admissions by the other side, and the court''s own findings. The number beside it is every approved item, however qualified.',
     'text',
     'Sworn admissions by the other side, and the court''s own findings. The number beside it is every approved item, however qualified.',
     NULL, NULL,
     'What the headline number means, shown when you hover it and read aloud by '
     'screen readers. A number whose definition is invisible is one a reader has '
     'to trust instead of check.',
     NULL, now(), 'system (seed)'),

    ('matrix_tier_strong_chip', 'Their own words', 'text', 'Their own words',
     NULL, NULL,
     'The label on a drill-down row that counts toward the headline: a sworn '
     'admission from the other side, or a finding by the court. Rename freely — '
     'which items earn it is set by the strong list, not by this name.',
     NULL, now(), 'system (seed)'),

    ('matrix_tier_hedged_chip', 'Qualified', 'text', 'Qualified',
     NULL, NULL,
     'The label on a drill-down row that is sworn but hedged — admitted in part, '
     'or answered evasively.',
     NULL, now(), 'system (seed)'),

    ('matrix_tier_other_chip', 'Our sworn word', 'text', 'Our sworn word',
     NULL, NULL,
     'The label on a drill-down row that is ours rather than theirs — an '
     'affidavit statement made under oath by our side.',
     NULL, now(), 'system (seed)'),

    ('matrix_duplicate_template', '×{count}', 'text', '×{count}',
     NULL, NULL,
     'Marks a row where the same statement was recorded more than once and the '
     'copies were folded into one line. {count} is how many there were and must '
     'stay in the text. Shown only above one — a row reading "×1" on every line '
     'would be noise.',
     NULL, now(), 'system (seed)'),

    ('matrix_ranked_list_note', 'Strongest first', 'text', 'Strongest first',
     NULL, NULL,
     'Said once above the drill-down list, so a reader knows the order is a claim '
     'about strength and not the order things happened to be found in.',
     NULL, now(), 'system (seed)'),

    -- ═══ Group 2: the war room's words (the R2 §3 rows, owed since 2026-08-10) ═

    ('war_room_subtitle',
     'The attacks and what we answer them with — built by you, gathered by the system, rehearsed by Marie.',
     'text',
     'The attacks and what we answer them with — built by you, gathered by the system, rehearsed by Marie.',
     NULL, NULL,
     'The sentence under the Trial Prep heading. It replaces "System-generated '
     'cross-examination scenarios", which credited the machine for work a human '
     'does: you write the attack, the scan gathers candidates, and you rule every '
     'one of them.',
     NULL, now(), 'system (seed)'),

    ('war_room_metric_scenarios_label', 'Scenarios', 'text', 'Scenarios',
     NULL, NULL,
     'The label on the tile counting every scenario in the case.',
     NULL, now(), 'system (seed)'),

    ('war_room_metric_ready_label', 'Ready', 'text', 'Ready',
     NULL, NULL,
     'The label on the tile counting scenarios that have passed the readiness '
     'checks and can be rehearsed.',
     NULL, now(), 'system (seed)'),

    ('war_room_metric_draft_label', 'Draft', 'text', 'Draft',
     NULL, NULL,
     'The label on the tile counting scenarios that are not yet Ready. It '
     'replaces "Drafted / in review": the tile carries ONE number, and a slashed '
     'pair of words invites a reader to look for two.',
     NULL, now(), 'system (seed)'),

    -- ═══ P2a: the link panel, on a card that already holds a link ═════════════

    ('card_already_linked_note',
     'This card is already linked. Add another accusation if it bears on more than one.', 'text',
     'This card is already linked. Add another accusation if it bears on more than one.',
     NULL, NULL,
     'Shown above the linking control on a card that already carries at least one '
     'link. Until now the control disappeared after the first link, so a '
     'statement bearing on two accusations could only be linked to one of them '
     'and the second link meant dropping the first. The schema and the API always '
     'allowed many.',
     NULL, now(), 'system (seed)')

ON CONFLICT (key) DO NOTHING;
