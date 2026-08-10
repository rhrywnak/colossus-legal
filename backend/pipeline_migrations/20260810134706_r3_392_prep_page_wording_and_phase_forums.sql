-- r3_392_prep_page_wording_and_phase_forums
--
-- Created: 2026-08-10 13:47:06
-- Target: pipeline database (colossus_legal_v2)
--
-- Task CC_TASK_R3_REHEARSAL_PAGE_392_v1 — the rehearsal page becomes the prep
-- page Marie works from in front of Chuck.
--
-- ## 1 — the plain-words count line, grammatical at ONE
--
-- The page opens with "They said it 5 times, in 3 documents, from December 2009
-- through October 2015 — every one is below, with your answer under it."
--
-- Seven rows for one sentence, and the reason is the deferral: .391 put off the
-- general singular/plural template system, and this page cannot wait for it.
-- "They said it 1 times" on the surface a witness preps from is the kind of slip
-- a reader stops trusting the whole page over. So each count-bearing clause
-- carries BOTH forms as its own row and the composer picks — the narrow version
-- of the same idea, and the rows a general system would eventually read.
--
-- The span clause has two forms for a reason that is not pluralisation: when
-- every dated instance shares one date, "from December 2009 through December
-- 2009" is a sentence nobody would write by hand, and on a five-instance
-- scenario that is a plausible state.
--
-- The span can also be ABSENT. Measured on DEV: 228 of 525 evidence nodes carry
-- a date, so 57% do not, and a scenario whose instances are all undated has no
-- endpoints. The clause is then omitted rather than rendered with an invented
-- span — which is why the frame puts it between commas that close up cleanly.
--
-- ## 2 — the four case phases, and the forum each document belongs to
--
-- Phase chips tag every instance card and filter the list. The assignment rule is
-- FORUM-WINS: a Court of Appeals ruling is COA even though its date falls inside
-- the probate years, because containment is not the question — which court was
-- speaking is.
--
-- ## Why the forums are a ROW and not read from the graph
--
-- Measured 2026-08-10: `Document` nodes carry six properties — `doc_type`, `id`,
-- `ingested_at`, `source_document_id`, `status`, `title` — and NONE of them is a
-- forum. `doc_type` does not stand in for one either: `court_ruling` covers both
-- the Judge Tighe probate opinion (April 2012) and the Court of Appeals ruling
-- (January 2012), which are precisely the two documents the forum-wins rule
-- exists to separate.
--
-- Date-only assignment was considered and REJECTED by the architect: it would tag
-- the Court of Appeals ruling "Probate" on the page Marie preps from — a
-- known-wrong chip, which is worse than a missing one.
--
-- So the case's nine documents name their own forum here. Nine rows in one
-- string, retunable in Settings with no build, and a document absent from the map
-- simply falls through to the date rule rather than being guessed at.
--
-- ## Why the boundaries are dates and the forums are ids
--
-- The two halves answer different questions. A boundary says "when did probate
-- begin"; a forum says "who was speaking in this document". Only the first is a
-- property of the case's timeline, and only the second needs to know that
-- `doc-court-of-appeals-rulling-01-12-2012` is an appeal.
--
-- ## Idempotence and ordering
--
-- `ON CONFLICT (key) DO NOTHING` — seeding must never overwrite a value Roman has
-- since tuned. Forward-only, applied at backend boot by the Migrator; a declared
-- key with no row REFUSES START (v2 §2b), so this reaches the database with or
-- before the .392 image.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    -- ── 1 · the count line ─────────────────────────────────────────────────
    ('rehearsal_count_line_template', 'They said it {times}, in {documents}{span} — every one is below, with your answer under it.', 'text',
     'They said it {times}, in {documents}{span} — every one is below, with your answer under it.',
     NULL, NULL,
     'The prep page''s opening count line. {times} and {documents} arrive already '
     'pluralised; {span} is the date clause and arrives EMPTY when nothing in the '
     'scenario is dated, so it carries its own leading comma and the sentence '
     'closes up cleanly without it.',
     'services::rehearsal_render::plain_count_line', NOW(), 'migration'),

    ('rehearsal_count_times_one', '1 time', 'text', '1 time', NULL, NULL,
     'The singular form of the times-said count. Carries {n} so a rewording can '
     'move the number around inside it.',
     'services::rehearsal_render::plain_count_line', NOW(), 'migration'),

    ('rehearsal_count_times_many', '{n} times', 'text', '{n} times', NULL, NULL,
     'The plural form of the times-said count. Carries {n}.',
     'services::rehearsal_render::plain_count_line', NOW(), 'migration'),

    ('rehearsal_count_documents_one', '1 document', 'text', '1 document', NULL, NULL,
     'The singular form of the document count. Carries {n}.',
     'services::rehearsal_render::plain_count_line', NOW(), 'migration'),

    ('rehearsal_count_documents_many', '{n} documents', 'text', '{n} documents', NULL, NULL,
     'The plural form of the document count. Carries {n}.',
     'services::rehearsal_render::plain_count_line', NOW(), 'migration'),

    ('rehearsal_count_span_range', ', from {from} through {through}', 'text',
     ', from {from} through {through}', NULL, NULL,
     'The date clause when the dated instances span a range. The LEADING COMMA is '
     'part of this row: the clause is optional, and a comma living in the frame '
     'would strand itself on a scenario with no dates.',
     'services::rehearsal_render::plain_count_line', NOW(), 'migration'),

    ('rehearsal_count_span_one_date', ', on {date}', 'text', ', on {date}', NULL, NULL,
     'The date clause when every dated instance shares ONE date. Its own row '
     'because "from December 2009 through December 2009" is a sentence nobody '
     'would write, and on a small scenario it is a plausible state.',
     'services::rehearsal_render::plain_count_line', NOW(), 'migration'),

    -- ── 2 · the four case phases ───────────────────────────────────────────
    ('rehearsal_phase_labels', 'Pre-probate|Probate|COA|Complaint', 'text',
     'Pre-probate|Probate|COA|Complaint', NULL, NULL,
     'The four case phases, in chronological order, pipe-separated. These are the '
     'chip labels AND the tokens the forum map below refers to, so a rename here '
     'must be made in both places — which is why they are one list rather than '
     'four rows.',
     'services::rehearsal_phase', NOW(), 'migration'),

    ('rehearsal_phase_boundaries', '2009-06|2014-01', 'text', '2009-06|2014-01',
     NULL, NULL,
     'The two dates that split the timeline when no forum is known: before the '
     'first is Pre-probate, on or after the second is Complaint, and everything '
     'between is Probate. COA is never assigned by date — an appeal is decided by '
     'WHICH COURT spoke, and its dates fall inside the probate years. Compared as '
     'string prefixes, so a year-only evidence date ("2011") sorts and classifies '
     'correctly against a month boundary.',
     'services::rehearsal_phase', NOW(), 'migration'),

    ('rehearsal_phase_document_forums',
     'doc-court-of-appeals-rulling-01-12-2012=COA', 'text',
     'doc-court-of-appeals-rulling-01-12-2012=COA', NULL, NULL,
     'Which forum each document belongs to, as `document-id=Phase` pairs '
     'separated by commas. FORUM WINS over the date: the Court of Appeals ruling '
     'is COA even though 2012-01-12 falls inside the probate years. Measured '
     '2026-08-10: the graph carries no forum property and doc_type does not stand '
     'in for one (court_ruling covers both the probate opinion and the appeal), so '
     'this map is where that fact lives. A document not listed here falls through '
     'to the date rule rather than being guessed at — add a pair to correct one, '
     'no build needed.',
     'services::rehearsal_phase', NOW(), 'migration'),

    ('rehearsal_phase_undated_label', 'No date yet', 'text', 'No date yet',
     NULL, NULL,
     'The chip on an instance whose statement carries no date and whose document '
     'names no forum. 57% of this case''s evidence has no date (measured '
     '2026-08-10), so this is a common and honest state — it is the standing '
     'prompt to add one on the working page, not an error.',
     'services::rehearsal_phase', NOW(), 'migration'),

    -- ── 3 · the chronology section's count ─────────────────────────────────
    ('rehearsal_answered_line_all', '{answered} of {total} answered', 'text',
     '{answered} of {total} answered', NULL, NULL,
     'The heading count on the chronology section when every marked statement '
     'has an answer paired to it.',
     'services::rehearsal_render::answered_line', NOW(), 'migration'),

    ('rehearsal_answered_line_some', '{answered} of {total} answered — {remaining} to prepare', 'text',
     '{answered} of {total} answered — {remaining} to prepare', NULL, NULL,
     'The same count when work remains. Its own row rather than one template with '
     'an always-present clause: "5 of 5 answered — 0 to prepare" is a to-do list '
     'with an empty item on it. The second clause says outright what is left, so '
     'a witness does not have to subtract toward the number they came for.',
     'services::rehearsal_render::answered_line', NOW(), 'migration')
ON CONFLICT (key) DO NOTHING;
