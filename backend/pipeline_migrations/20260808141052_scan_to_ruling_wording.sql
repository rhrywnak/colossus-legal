-- scan_to_ruling_wording: the words the PROJECTION speaks
--
-- Created: 2026-08-08 14:10:52
-- Target: pipeline database (colossus_legal_v2)
--
-- Task CC_TASK_SCAN_TO_RULING_WORKFLOW_v1, piece 6
-- (SCAN_TO_RULING_WORKFLOW_REDESIGN_v1, as ruled by the architect 2026-08-08).
--
-- ## What changed on the screen, and therefore here
--
-- A completed scan's admitted verdicts are now a READ-TIME PROJECTION: they
-- appear in the candidate queue as PROPOSED cards with no human step in between.
-- Select-twice — tick a finding, press Merge, then find the same item again in
-- the queue — is gone, and so is the Merge button.
--
-- Twelve new rows, in two families:
--
-- 1. Four CURATION rows (`queue_*` / `card_*`) — the queue's leading heading and
--    the three new things a proposed card says. They ride the evidence-links
--    wording payload the card surface already receives.
-- 2. Eight SCAN rows (`scan_*`) — the collapsed scan card's one-line summary and
--    the seven strings of the numbers-only report. They ride the scan-history
--    payload the panel already receives.
--
-- ## One UPDATE, and why it is guarded
--
-- `scan_history_delete_confirm_template` became FALSE with this change. It says
-- a run's verdicts "are what support the rulings it produced" — under the new
-- model, deleting a run removes its UNRULED proposals and a run any ruling has
-- drawn on refuses deletion outright (architect ruling R1). The UPDATE therefore
-- rewrites it, but ONLY where the stored value is still the seeded default: if
-- Roman has edited that sentence on the Settings page, his words stay. That is
-- the same respect for a human edit that `ON CONFLICT (key) DO NOTHING` gives
-- every INSERT below.
--
-- ## The deploy ordering hazard is real for this file too
--
-- All twelve keys are declared to the boot loader (`WORDING_KEYS`,
-- `SCAN_WORDING_KEYS`), and a declared key with no row makes the backend REFUSE
-- TO START — there are no compiled-in defaults to serve with (v2 §2b). The
-- runtime Migrator applies this at backend boot, before the settings load, so a
-- normal deploy orders itself. A rollback to an older image is safe (extra rows
-- are ignored); a roll-FORWARD without this file is not.
--
-- Forward-only, idempotent, no down migration — the house rule for every seed.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    -- ── 1. The queue leads with proposals ────────────────────────────────────
    --
    -- The first line a curator reads on a scanned scenario. Both facts are
    -- load-bearing: how many are waiting, and WHICH scan put them there. Without
    -- the second, a projection is indistinguishable from the raw evidence pool —
    -- the exact false claim removed from this same heading on 2026-08-08.
    ('queue_proposed_heading_template',
     'Candidates awaiting ruling — {count} proposed by the {when} scan',
     'text',
     'Candidates awaiting ruling — {count} proposed by the {when} scan',
     NULL, NULL,
     'The queue''s heading when a completed scan is proposing candidates. Must '
     'keep {count} and {when} — a heading that does not say which scan proposed '
     'these rows cannot be told apart from one describing the raw pool.',
     NULL, now(), 'system (seed)'),

    -- ── 2. What a proposed card says ─────────────────────────────────────────
    ('card_proposed_attribution_template', 'Proposed by the {when} scan',
     'text', 'Proposed by the {when} scan',
     NULL, NULL,
     'The attribution line on a proposed card. Must keep {when} — an attribution '
     'with no date attributes nothing, and last night''s judgment and last '
     'month''s are different things to weigh.',
     NULL, now(), 'system (seed)'),

    -- The template exists so the chip can NAME THE SCAN as the speaker, exactly
    -- as the banded-confidence label does. A chip reading "supports" beside a
    -- sworn admission reads as the record''s own stance, which it is not.
    ('card_proposed_role_template', 'Scan: {verb}', 'text', 'Scan: {verb}',
     NULL, NULL,
     'The proposed-role chip on a card a scan put forward. Must keep {verb}, '
     'which is filled with the plain stance word (supports / disputes). The rest '
     'of the sentence is what stops the chip reading as the record''s own stance '
     'rather than a model''s claim about it.',
     NULL, now(), 'system (seed)'),

    -- One ruling settles every byte-identical twin (architect ruling R2), so the
    -- badge has to say how far the click reaches.
    ('card_proposed_covers_template', '×{count} — covers {codes}',
     'text', '×{count} — covers {codes}',
     NULL, NULL,
     'The badge on a proposed card whose single ruling also settles a '
     'byte-identical twin. Must keep {count} and {codes}. Shown only when a card '
     'speaks for more than itself.',
     NULL, now(), 'system (seed)'),

    -- ── 3. The scan card, collapsed ──────────────────────────────────────────
    --
    -- The card folds away once a run exists, because the scan is no longer where
    -- the work happens — the queue below it is. This line has to carry enough
    -- that nobody expands the card just to find out whether it is worth expanding.
    ('scan_card_collapsed_summary_template',
     'Last scan {when} · {model} · {count} proposed',
     'text', 'Last scan {when} · {model} · {count} proposed',
     NULL, NULL,
     'The single line a collapsed scan card shows. Must keep {when}, {model} and '
     '{count} — the card is folded by default, and these three are what let a '
     'human decide not to open it.',
     NULL, now(), 'system (seed)'),

    -- ── 4. The report is a receipt now ───────────────────────────────────────
    --
    -- The epitaph of select-twice. The findings list was a work surface with
    -- checkboxes and a Merge button; it is numbers only now, and this sentence
    -- says so before anybody goes looking for the controls that used to be there.
    ('scan_report_advisory_note',
     'Advisory only. Nothing here needs your click — the proposed candidates are already in the queue below.',
     'text',
     'Advisory only. Nothing here needs your click — the proposed candidates are already in the queue below.',
     NULL, NULL,
     'The line under the scan report''s heading. The report used to be where '
     'candidates were selected and merged; it is a read-only record now, and a '
     'human who remembers the old screen needs telling.',
     NULL, now(), 'system (seed)'),

    -- Deliberately NOT part of the conservation sentence (architect ruling R5):
    -- conservation is composed from the run's FROZEN counts and describes what
    -- that run did, while this number falls every time the human rules. Splicing
    -- a live number into a frozen record would make the record appear to move.
    ('scan_report_proposed_line_template',
     '{count} proposed and awaiting your ruling — a live count, not part of the run''s frozen record above.',
     'text',
     '{count} proposed and awaiting your ruling — a live count, not part of the run''s frozen record above.',
     NULL, NULL,
     'The scan report''s live proposed count, under the frozen conservation line. '
     'Must keep {count}. Kept a separate sentence on purpose: everything above it '
     'describes what the run did and never changes, and this falls as you rule.',
     NULL, now(), 'system (seed)'),

    -- The five tile captions, in display order.
    ('scan_report_tile_gathered', 'gathered', 'text', 'gathered',
     NULL, NULL,
     'Caption of the scan report''s first tile: how many rows the ungated gather '
     'returned.',
     NULL, now(), 'system (seed)'),

    ('scan_report_tile_folded', 'duplicates folded', 'text', 'duplicates folded',
     NULL, NULL,
     'Caption of the scan report''s second tile: rows folded into a '
     'byte-identical twin and judged once. These are LLM calls saved, not '
     'candidates discarded.',
     NULL, now(), 'system (seed)'),

    ('scan_report_tile_set_aside', 'set aside before judging', 'text',
     'set aside before judging',
     NULL, NULL,
     'Caption of the scan report''s third tile: rows the pre-filter kept from the '
     'judge — empty quotes, content-free statement kinds, unanchored fragments.',
     NULL, now(), 'system (seed)'),

    ('scan_report_tile_judged', 'judged', 'text', 'judged',
     NULL, NULL,
     'Caption of the scan report''s fourth tile: the groups the model was actually '
     'asked about — one LLM call each.',
     NULL, now(), 'system (seed)'),

    ('scan_report_tile_proposed', 'proposed', 'text', 'proposed',
     NULL, NULL,
     'Caption of the scan report''s fifth tile: how many candidates are in the '
     'queue awaiting a ruling right now. The only tile whose number is live.',
     NULL, now(), 'system (seed)')

ON CONFLICT (key) DO NOTHING;

-- ── The one correction ──────────────────────────────────────────────────────
--
-- Guarded to the seeded default so a human edit is never overwritten. Under the
-- projection there is no merge: deleting a run removes the proposals nobody has
-- ruled, and a run any ruling has drawn on is refused (409) because those
-- rulings cite it. The old sentence promised the opposite.
UPDATE app_settings
   SET value = 'Remove the scan run from {run}? Any candidate it proposed that you have not ruled disappears with it. Your rulings are untouched — and a run your rulings cite cannot be removed at all.',
       default_value = 'Remove the scan run from {run}? Any candidate it proposed that you have not ruled disappears with it. Your rulings are untouched — and a run your rulings cite cannot be removed at all.',
       meaning = 'Asked before a scan run is destroyed. Must keep {run}, which names the run by when it was started — the history lists several rows that otherwise look alike. The sentence has to be exact about what survives: rulings do, unruled proposals do not.',
       updated_at = now(),
       updated_by = 'system (scan-to-ruling correction)'
 WHERE key = 'scan_history_delete_confirm_template'
   AND value = 'Remove the scan run from {run}? Its verdicts are deleted with it, and they are what support the rulings it produced.';
