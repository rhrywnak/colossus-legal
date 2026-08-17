-- add_document_phase: Add documents.phase
--
-- Created: 2026-08-17
-- Target: pipeline database (colossus_legal_v2)
--
-- Roman: "the documents processed list does not include the phase." Every
-- document belongs to one of four phases of this case, the Documents list has no
-- way to show it, and nothing in the schema records it.
--
-- ## The vocabulary is NOT invented here
--
-- The four phases already exist, as the timeline's own data:
-- `frontend/public/data/timeline.json` carries `estate`, `probate`, `appeals`
-- and `civil_lawsuit`, and the Home page's timeline band has been rendering
-- pills from them. This column stores THOSE slugs and no others. A second
-- vocabulary — even one that only differed in spelling — would mean a document
-- tagged `coa` and a timeline phase called `appeals` never meeting.
--
-- ## Slugs here, labels nowhere
--
-- Ruled 2026-08-17: the display labels (PRE-PROBATE · PROBATE · COA · COMPLAINT)
-- live in `timeline.json` and are read from there by every surface that renders
-- one. This database stores the slug, the API returns the slug, and neither ever
-- carries a label. That is why renaming a phase for display is a one-line data
-- edit and not a migration.
--
-- ## Never required (chronology design R4: absence tolerated)
--
-- NULL means nobody has said which phase this document belongs to. There is no
-- "unknown" member and no CHECK forcing a value, because unlike a document's
-- date there is no meaningful difference between "not asked" and "answered
-- unknown" — a document that belongs to no phase of this case would not be in
-- the corpus. All nine existing rows start NULL and Roman sets them by hand.

ALTER TABLE documents ADD COLUMN IF NOT EXISTS phase TEXT;

-- The vocabulary, as the database's backstop. Kept in step with
-- `domain::case_phase::CasePhase`, which is what the API validates against, and
-- with `timeline.json`, which is what the frontend renders from. Three copies of
-- four strings is two too many, but the other two are in different languages and
-- a different repository layer; this one exists so a value that reaches the table
-- by any other route is still refused.
ALTER TABLE documents DROP CONSTRAINT IF EXISTS documents_phase_valid;
ALTER TABLE documents ADD CONSTRAINT documents_phase_valid
    CHECK (phase IS NULL
           OR phase IN ('estate', 'probate', 'appeals', 'civil_lawsuit'));

-- The Documents list filters by phase, and the P1 spine orders by it. Partial,
-- because a filter on a phase never wants the rows that have none.
CREATE INDEX IF NOT EXISTS idx_documents_phase
    ON documents (phase)
    WHERE phase IS NOT NULL;

COMMENT ON COLUMN documents.phase IS
    'Which phase of the case this document belongs to: estate | probate | appeals | civil_lawsuit. The same slugs the timeline uses (frontend/public/data/timeline.json). NULL means nobody has said yet — never required. Display labels live in timeline.json, never here.';
