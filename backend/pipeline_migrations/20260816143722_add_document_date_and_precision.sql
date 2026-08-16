-- add_document_date_and_precision: Add document_date and date_precision
--
-- Created: 2026-08-16
-- Target: pipeline database (colossus_legal_v2)
--
-- Task B2 §3 / P4a. The document's OWN date — when it was filed, served, signed
-- or sent — has never existed anywhere in this system. Measured and reconfirmed
-- twice (.394 P4a, and the wave-verification report): Neo4j `Document` nodes
-- carry six properties and none is a date; `documents` has `created_at`,
-- `updated_at` and `assigned_at`, all row lifecycle. The only place a document's
-- date exists today is inside its title string, in ambiguous formats
-- ("08 08 16", "041212", "11 1 13").
--
-- ## The two kinds of date, never confused (B2 §1)
--
--   statement_date  — when a STATEMENT was made or sworn. Extracted per
--                     statement by the templates. Not this.
--   document_date   — when the DOCUMENT was filed/served/created. Intake
--                     metadata, entered by a human. This.
--
-- Nothing in this migration or its code parses a date out of a title. The
-- invented-date class stays dead: a human enters these from the documents
-- themselves.
--
-- ## Why two columns and not one
--
-- Real documents are dated to different precisions. A filing stamp gives a day;
-- a letter may give only "November 2009"; an undated exhibit may give a year or
-- nothing at all. Storing 2009-11-01 for "November 2009" would be a fabricated
-- day that no reader could later tell from a real one. So the DATE column always
-- holds the first day of the stated period, and `date_precision` says how much of
-- it the source actually stated. A consumer that shows "1 November 2009" for a
-- month-precision date is the consumer's bug; the data does not lie.
--
-- ## Mandatory-with-override, enforced here
--
-- The ruling is that intake must ASK, may be told "unknown", and must never
-- accept a silent blank. The CHECK below is the database's half of that:
--
--   date_precision = 'unknown'  <->  document_date IS NULL
--
-- The two states cannot disagree. "I do not know this document's date" is
-- recordable and distinguishable from "nobody has been asked yet"
-- (date_precision IS NULL) — the distinguishability Standing Rule 1 requires.
-- Every existing row starts in that third state, which is honest: the nine
-- ingested documents have not been asked yet, and Roman backfills them by hand.

ALTER TABLE documents ADD COLUMN IF NOT EXISTS document_date DATE;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS date_precision TEXT;

-- The vocabulary. Kept in step with `domain::date_precision::DatePrecision`,
-- which is the code-owned lookup the API validates against; this constraint is
-- the backstop for anything that reaches the table another way.
ALTER TABLE documents DROP CONSTRAINT IF EXISTS documents_date_precision_valid;
ALTER TABLE documents ADD CONSTRAINT documents_date_precision_valid
    CHECK (date_precision IS NULL
           OR date_precision IN ('day', 'month', 'year', 'unknown'));

-- The mandatory-with-override invariant, stated as an invariant.
ALTER TABLE documents DROP CONSTRAINT IF EXISTS documents_date_precision_agrees;
ALTER TABLE documents ADD CONSTRAINT documents_date_precision_agrees
    CHECK (
        -- Never asked: neither is set.
        (date_precision IS NULL AND document_date IS NULL)
        -- Asked and answered "unknown": precision set, date deliberately absent.
        OR (date_precision = 'unknown' AND document_date IS NULL)
        -- Asked and dated: both set, at the precision the source stated.
        OR (date_precision IN ('day', 'month', 'year') AND document_date IS NOT NULL)
    );

-- Consumers arrive later (the /timeline spine, the .394 P4a count-line
-- fallback), and every one of them filters to the dated rows. Partial, because
-- the undated rows are exactly the ones no consumer wants.
CREATE INDEX IF NOT EXISTS idx_documents_document_date
    ON documents (document_date)
    WHERE document_date IS NOT NULL;

COMMENT ON COLUMN documents.document_date IS
    'The document''s own date (filed/served/signed/sent), entered by a human at intake. First day of the stated period; see date_precision for how much of it the source stated. NEVER parsed from the title.';
COMMENT ON COLUMN documents.date_precision IS
    'day | month | year | unknown. NULL means nobody has been asked yet, which is distinct from a human answering "unknown".';
