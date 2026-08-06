-- add_accusation_text_to_scenarios: the accusation in a human's plain words
--
-- Created: 2026-08-05 20:30:22
-- Target: pipeline database (colossus_legal_v2, forward-only)
--
-- Task 2.11 B1. This kills the first of the two beta.371 rehearsal defects at
-- the root rather than at the render.
--
-- ## The defect, precisely
--
-- The rehearsal page's "What they say" block reads `definition->>'attack_text'`.
-- On S-2 that column holds:
--
--     "We wouldn't have had to go through the auction process, but the parties
--      did not cooperate with each other."
--
-- That is a verbatim first-person quote FROM THE RECORD — a thing one of them
-- said once. It is not what they SAY, in the sense the page means: the standing
-- accusation, stated plainly by a human, that the instances below are instances
-- OF. Rendering the quote in that slot silently promotes one piece of evidence
-- into the summary of all of it.
--
-- `attack_text` is not wrong; it is being asked the wrong question. It stays
-- exactly as it is — THEIR words, in the record, where the working view uses it.
-- This column is the different thing the rehearsal block actually needs.
--
-- ## Why a column and not another key in `definition`
--
-- `definition` is the authored scenario body as JSONB. A JSONB key cannot be
-- constrained, cannot be indexed usefully here, and reads back as
-- `Option<Value>` needing a cast at every site. This value is one authored
-- sentence with one consumer and a NOT-blank rule worth enforcing — a column
-- says so and the type system carries it.
--
-- NULLABLE, and the NULL is the honest-gap law working: a scenario whose
-- accusation nobody has written yet renders the gap message. It never renders a
-- stand-in, and it is never backfilled from `attack_text` — that would
-- reintroduce the defect with an extra step.

ALTER TABLE scenarios
    ADD COLUMN accusation_text TEXT;

ALTER TABLE scenarios
    ADD CONSTRAINT scenarios_accusation_text_not_blank
    CHECK (accusation_text IS NULL OR btrim(accusation_text) <> '');

COMMENT ON COLUMN scenarios.accusation_text IS
    'The standing accusation in a human''s plain words — what the marked instances '
    'are instances OF. NULL until somebody writes it, which the rehearsal page renders '
    'as a named gap. Never derived from definition->>''attack_text'', which is THEIR '
    'verbatim words in the record and a different thing.';
