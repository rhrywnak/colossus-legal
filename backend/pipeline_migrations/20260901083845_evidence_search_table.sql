-- evidence_search_table: the lexical half of the ranked gather
--
-- Created: 2026-09-01 08:38:45
-- Target: pipeline database (colossus_legal_v2, applied at backend boot by the
--         runtime sqlx::migrate::Migrator over ./pipeline_migrations — NOT the
--         compile-time migrate! macro, which serves ./migrations and the MAIN
--         database. See database.rs::init_pools for the two-pool split.)
--
-- GATHER_CASCADE_EXECUTION_PLAN_v1 §3 L1, taking its stated branch: the
-- verbatim quote lives in Neo4j only, so this migration adds the read-model
-- table the index step will fill (task L1c).
--
-- ## SCOPE: this file is the table and its two indexes. Nothing fills it yet.
--
-- L1a is deliberately only the shape. The one-shot backfill from Neo4j is L1b;
-- the pipeline index step writing the mirror beside its Qdrant upsert — the ONE
-- write path — is L1c. Until L1c ships, this table is empty and nothing reads
-- it, which is also what makes the rollback at the bottom of this comment free.
--
-- ## Why a Postgres mirror rather than a Neo4j-native full-text index
--
-- Roman's ruling, 2026-09-01. The lexical half of the gather exists to catch the
-- things embeddings are worst at: "$50,000", "Milster", "Form 1724". Every
-- full-text analyzer — Lucene's inside Neo4j, Postgres's here — tokenises those
-- strings and throws the punctuation away. The difference is that Postgres also
-- ships pg_trgm, so ONE store can hold both the analyzed index and a
-- character-level one over the same column and the two can be queried together.
-- Neo4j has no trigram equivalent. So the corpus's hardest queries decide the
-- store, and the store is Postgres. The precise, measured behaviour of both
-- halves is on the trigram index below.
--
-- ## Why NO foreign key to `documents`
--
-- Deliberate. The graph is the authority for what Evidence exists. A foreign key
-- here would give this mirror the power to REJECT a row the graph already
-- accepted — a derived table vetoing its own source. When L1c writes a row whose
-- `document_id` Postgres has never heard of, the right outcome is a mirrored row
-- and a loud log, not a constraint violation that fails the index step.
--
-- ## FORWARD-ONLY
--
-- The pipeline Migrator applies migrations forward only; this repo has no down
-- files and no down convention (checked: `find backend -name '*.down.sql'`
-- returns nothing). A bad forward migration is corrected by a further forward
-- migration. Because nothing reads this table yet, the rollback is exactly:
--
--     DROP INDEX IF EXISTS idx_evidence_search_probe_trgm;
--     DROP INDEX IF EXISTS idx_evidence_search_vector;
--     DROP TABLE IF EXISTS evidence_search;
--     -- pg_trgm is deliberately NOT dropped: see the note on the extension.

-- The trigram half. `IF NOT EXISTS` so a re-run is a no-op, and so an
-- environment where an operator already installed it is not a failure.
--
-- If the connected role cannot create it, this migration FAILS LOUDLY at boot
-- rather than degrading to one index — a half-built search surface that silently
-- misses every dollar amount is precisely the silent failure Standing Rule 1
-- forbids. (Measured on DEV 2026-09-01: the backend connects as `postgres`,
-- which is `rolsuper = t`, and pg_trgm 1.6 is available but was NOT yet
-- installed. This statement is what installs it.)
CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE TABLE IF NOT EXISTS evidence_search (
    -- The Neo4j node id, e.g.
    -- `doc-george-phillips-admissions-response:evidence:41068bce`.
    -- TEXT, not UUID: these ids are composite and human-readable by design, and
    -- they are the join key every other store in this project already uses for
    -- Evidence (scenario_fact_refs.graph_node_id, scan_run_verdicts.graph_node_id,
    -- scenario_candidate_ordinals.graph_node_id all hold the same shape).
    evidence_id       TEXT        PRIMARY KEY,

    -- The Document node this Evidence was extracted from. NOT NULL because a
    -- quote with no source is not evidence. No FK — see the header.
    document_id       TEXT        NOT NULL,

    -- Nullable: `coalesce(e.title, '')` in the graph layer means a title-less
    -- node is real. NULL here is "the graph had none", which stays distinct from
    -- the empty string somebody typed.
    title             TEXT,

    -- The verbatim quote — the thing the reranker actually scores, and the only
    -- column the trigram index covers. NOT NULL: an Evidence row with no quote
    -- could not be scored at all, and mirroring one would quietly shrink the
    -- denominator of every recall number computed off this table. (Measured on
    -- DEV 2026-09-01: 0 of 1209 Evidence nodes have a null or empty
    -- verbatim_quote, so this constraint costs nothing today and names the
    -- invariant for the day it would.)
    quote             TEXT        NOT NULL,

    -- The extractor's note on why the quote matters. Nullable for the same
    -- reason as `title`.
    significance      TEXT,

    -- Page number for the pinpoint. Nullable — a quote with no page is a real
    -- (if poor) node.
    --
    -- BIGINT, not INTEGER (Roman's ruling R1, 2026-09-01, on
    -- CC_REPORT_GATHER_L1A_v1). The source of this value is
    -- `BiasInstance.page_number: Option<i64>`, and every one of the nine Rust
    -- call sites that carries a page number in this codebase types it `i64`. The
    -- nearest precedent, `scenario_ruling_anchors.page`, is BIGINT and wrote down
    -- the reason: INTEGER would make L1c narrow i64 → i32, which is either a
    -- fallible conversion — a new error path for a value that cannot realistically
    -- overflow — or a silent truncation. BIGINT stores it with no conversion at
    -- all, so there is no third state to invent an error for.
    page              BIGINT,

    -- The party ids this Evidence is ABOUT — e.g.
    -- `{org-catholic-family-services,person-emil-awad}`.
    --
    -- AN ARRAY, NOT A JOINED STRING. L2's subject filter asks a set-membership
    -- question ("is this Evidence about any party the linked allegations name?"),
    -- which Postgres answers directly with `&&` / `@>` on a TEXT[]; a joined
    -- string would have to be split again at read time, every time. G0's gate
    -- fixture already carries `about` as a list, and this matches it so the two
    -- representations of one fact cannot drift.
    --
    -- DEFAULT '{}' and NOT NULL: an Evidence node with no ABOUT edges has an
    -- empty list of subjects, which is a different and much more useful fact than
    -- NULL. It also means `about && $1` never has to guard for null.
    about             TEXT[]      NOT NULL DEFAULT '{}',

    -- When this mirror row was last written from the graph.
    --
    -- The ONLY timestamp this table carries, and deliberately so. An earlier
    -- draft also had `source_updated_at` — the graph's own last-modified stamp —
    -- for detecting stale rows. Roman ruled on 2026-09-01 that staleness is
    -- handled by WHOLE-DOCUMENT RE-SYNC instead: the index step rewrites every
    -- row for a document and deletes the ones the graph no longer has, so a row
    -- can never be stale relative to its document. That makes a per-row source
    -- stamp not merely unfillable (measured: no Evidence node carries an update
    -- property) but unnecessary, and a nullable column nothing ever writes is a
    -- question every future reader has to re-answer. This one IS knowable,
    -- always, because we are the ones writing it.
    synced_at         TIMESTAMPTZ NOT NULL DEFAULT now(),

    -- ## The full-text column
    --
    -- GENERATED ALWAYS AS … STORED, not a trigger. A trigger is code that can be
    -- dropped, disabled, or fail on one path while another writes around it; a
    -- generated column is recomputed by Postgres on every INSERT and every
    -- UPDATE and CANNOT drift from the row it summarises. Nothing in L1c has to
    -- remember to maintain it.
    --
    -- ## Rust/Postgres learning: why the two-argument to_tsvector is required
    --
    -- A generated column's expression must be IMMUTABLE. The one-argument
    -- `to_tsvector(text)` is only STABLE — it reads the session's
    -- `default_text_search_config`, so the same row could summarise differently
    -- for two connections. The two-argument form with a LITERAL configuration is
    -- IMMUTABLE, which is both why Postgres accepts it here and why it is the
    -- right thing: the index and the query must agree on the analyzer forever.
    --
    -- ## Why 'english'
    --
    -- The corpus is US probate litigation written in English: pleadings,
    -- transcripts, discovery responses. The english configuration gives us the
    -- two things that matter for recall — stemming, so a query for "deposit"
    -- reaches "deposited" and "depositing" (A-19/A-20 turn on exactly that
    -- word), and a stopword list, so "the", "of" and "was" do not dominate the
    -- ranking of a 60-word quote. 'simple' would give neither. No other language
    -- appears in the record.
    --
    -- And it is precisely BECAUSE 'english' analyzes that the trigram index below
    -- exists: the analyzer is the thing that turns "$50,000" into the two tokens
    -- '50' and '000', dollar sign discarded. The two indexes cover each other's
    -- blind spot; the trigram index's own comment states exactly where.
    --
    -- ## The weights
    --
    -- A: the quote — the words the witness actually said, the primary evidence.
    -- B: the title — the extractor's summary of the quote; useful, second-hand.
    -- C: the significance — the extractor's ARGUMENT about the quote. Real signal
    --    for retrieval, but a match there is a match on our own commentary rather
    --    than on the record, so it must never outrank a match in the quote
    --    itself. `ts_rank` reads these letters; the default weight vector is
    --    {D,C,B,A} = {0.1, 0.2, 0.4, 1.0}.
    search_vector     tsvector    GENERATED ALWAYS AS (
        setweight(to_tsvector('english', coalesce(quote, '')), 'A') ||
        setweight(to_tsvector('english', coalesce(title, '')), 'B') ||
        setweight(to_tsvector('english', coalesce(significance, '')), 'C')
    ) STORED,

    -- The surface the TRIGRAM half matches against: the same three fields
    -- `search_vector` carries, concatenated flat.
    --
    -- ⚑ Ruled 2026-09-01, after L2b measured what `quote` alone could reach.
    -- This column originally did not exist and the trigram index covered
    -- `quote`. The measured reason it changed:
    --
    --   109 of 1209 Evidence nodes have a quote of "Admitted." or "Denied as
    --   untrue." — a CORRECT extraction of a discovery response, and no
    --   retrievable text at all. Six of the seven $50,000 admissions AT-2 turns
    --   on are in that group, with the figure in `title`; a seventh has it in
    --   neither `title` nor `quote`. Against `quote` alone the trigram half was
    --   searching a column that says "Admitted." for 9% of the corpus.
    --
    -- The earlier draft argued for `quote` alone on the grounds that title and
    -- significance are OUR words, and a dollar amount appearing only in our
    -- commentary is not a citation anyone can put before a judge. That concern
    -- is real and it is answered by where it applies: this column decides what
    -- can be FOUND, not what can be cited. The card still carries its own quote,
    -- and a human still reads it before it goes anywhere. Meanwhile the
    -- full-text half has always carried all three (weighted A/B/C, directly
    -- above), so `quote`-only trigram made the two lexical halves search
    -- DIFFERENT text — which is what made their ranks incomparable.
    --
    -- Flat, with no weighting, because trigram has no weighting to give: it is a
    -- matching surface, not a scoring one. The single spaces keep a trigram from
    -- bridging the end of one field into the start of the next.
    probe_text        TEXT        GENERATED ALWAYS AS (
        coalesce(quote, '') || ' ' || coalesce(title, '') || ' ' ||
        coalesce(significance, '')
    ) STORED
);

-- The full-text half of the lexical read. GIN (not GiST): the table is written
-- rarely (one index step per document) and searched on every gather, which is
-- the exact trade GIN is built for.
CREATE INDEX IF NOT EXISTS idx_evidence_search_vector
    ON evidence_search USING GIN (search_vector);

-- The trigram half, and the reason there are TWO indexes on one table.
--
-- Stated precisely, because the loose version of this claim is wrong and the
-- next reader deserves the measured one. Everything below was measured on
-- Postgres 17.7 with the `english` configuration, 2026-09-01:
--
--   1. THE ANALYZER DISCARDS PUNCTUATION IT CANNOT TOKENISE.
--      `to_tsvector('english', '$50,000')` is `'50':1 '000':2` — the dollar sign
--      is gone and the comma has split the number in two. So full text does not
--      "miss" $50,000; it does something worse. A search for `$50,000` MATCHES a
--      quote reading "netted 50,000 in scrap" (measured: true), because by index
--      time the two strings are identical. `quote LIKE '%$50,000%'` on this
--      index tells them apart (measured: matches only the dollar-signed row).
--      For a case whose whole first act is one $50,000 check, that distinction
--      is the difference between a citation and a coincidence.
--
--      ⚑ AND HERE IS WHAT IT CANNOT DO, measured 2026-09-01 on the real corpus.
--      The claim above is about LIKE. The operator L2's gather actually uses is
--      the trigram one, and it cannot tell one figure from another at all:
--
--          cards literally containing $50,000                63
--          cards literally containing $500,000                2
--          '$50,000'     <% probe_text                       65  }  the SAME
--          '$500,000.00' <% probe_text                       65  }  65 ids
--          word_similarity('$500,000.00', a $50,000 card)  0.750  (threshold 0.6)
--          '$50,000'     <<% probe_text                      97  }  strict does
--          '$500,000.00' <<% probe_text                      97  }  not separate
--
--      So this index distinguishes `$50,000` from a bare `50,000` — which is
--      what point 1 claims and it is true — and it does NOT distinguish
--      `$50,000` from `$500,000.00`, a figure ten times larger. Neither trigram
--      operator separates them at any threshold, because trigrams see character
--      runs and `$50,000` is a run inside `$500,000.00`.
--
--      Recorded rather than fixed: telling amounts apart needs an exact-match
--      path (a LIKE clause, or a normalised currency column), which is a design
--      decision and a schema question, not a tuning one. It is written here
--      because this comment is where someone will come looking for what the
--      trigram half can do, and a comment that overstates a capability is how
--      the next person builds on something that is not there.
--
--   2. FULL TEXT MATCHES WHOLE TOKENS; TRIGRAMS MATCH SUBSTRINGS.
--      A query for `Milste` does not reach "Milster" through the tsvector
--      (measured: false) because token equality is all/nothing after stemming.
--      `quote ILIKE '%Milste%'` does (measured: true). Names, form numbers and
--      docket strings are typed from memory and half-remembered constantly.
--
--   3. FUZZY IS AVAILABLE BUT NOT FREE AT THE DEFAULTS — and this is the honest
--      caveat, not a feature claim. `word_similarity('Mllster', '… Milster …')`
--      is 0.5 against Postgres's default `word_similarity_threshold` of 0.6, so
--      the OCR transposition this corpus actually produces does NOT match with
--      `<%` out of the box. It becomes reachable only if L2 lowers the threshold
--      deliberately at query time. Nothing here does that; it is L2's ruling.
--
-- It covers `probe_text` — quote, title and significance concatenated — for the
-- reasons recorded on that column above. It REPLACES an earlier
-- `idx_evidence_search_quote_trgm` over `quote` alone, which is not created:
-- nothing would read it once the trigram half moved to `probe_text`, and an
-- unused GIN index is not free — it is rebuilt on every write the pipeline's
-- index step makes, and it is a trap for the next reader, who would reasonably
-- assume something searches it.
--
-- `quote` remains a plain column and is still read, projected and asserted
-- NOT NULL; it simply has no index of its own, because no query filters on it.
CREATE INDEX IF NOT EXISTS idx_evidence_search_probe_trgm
    ON evidence_search USING GIN (probe_text gin_trgm_ops);

COMMENT ON TABLE evidence_search IS
    'READ MODEL — a derived mirror, not a source of truth. Neo4j owns Evidence; this table copies the text so Postgres full-text and trigram search can reach it (GATHER_CASCADE_DESIGN_v1 Stage 1, lexical half). Nothing writes it but the pipeline index step (task L1c), beside its Qdrant upsert. Deliberately carries NO foreign key to documents: a derived table must not be able to reject a row the graph accepted.';
COMMENT ON COLUMN evidence_search.evidence_id IS
    'The Neo4j Evidence node id — the same composite TEXT key scenario_fact_refs, scan_run_verdicts and scenario_candidate_ordinals all hold.';
COMMENT ON COLUMN evidence_search.quote IS
    'The verbatim quote. NOT NULL: an unquotable row cannot be scored by the reranker, and mirroring one would silently shrink every recall denominator computed off this table.';
COMMENT ON COLUMN evidence_search.about IS
    'Party ids the Evidence is ABOUT, as an ARRAY so L2 subject filtering is a set-membership test (&&, @>) rather than a string re-split. Empty array = no ABOUT edges, which is a real state and distinct from NULL.';
COMMENT ON COLUMN evidence_search.synced_at IS
    'When this mirror row was last written from the graph. The only timestamp here: staleness is handled by whole-document re-sync (Roman, 2026-09-01), not by comparing a per-row source stamp.';
COMMENT ON COLUMN evidence_search.search_vector IS
    'Generated (never triggered, so it cannot drift): setweight quote A, title B, significance C over to_tsvector(''english'', …). The literal config is required — one-argument to_tsvector is only STABLE and a generated column demands IMMUTABLE.';
COMMENT ON INDEX idx_evidence_search_vector IS
    'Full-text half of the lexical gather. GIN because this table is written once per document and read on every gather.';
COMMENT ON INDEX idx_evidence_search_probe_trgm IS
    'Trigram half. Measured 2026-09-01 on PG 17.7: to_tsvector(''english'',''$50,000'') is ''50'' ''000'', so a full-text search for $50,000 also matches a bare "50,000" — this index tells THOSE two apart. It does NOT tell $50,000 from $500,000.00: measured, both match the same 65 ids under <% and the same 97 under <<%, because a trigram sees character runs and one figure is a run inside the other. Distinguishing amounts needs an exact-match path this index does not provide. It also reaches substrings ("Milste" -> "Milster"), which whole-token matching cannot. Fuzzy OCR variants are NOT covered at Postgres defaults (word_similarity 0.5 vs a 0.6 threshold) and would need L2 to lower it deliberately. Covers quote + title + significance (probe_text): 109 of 1209 quotes are a bare "Admitted." or "Denied as untrue.", so quote alone could not reach 9% of the corpus at all.';
COMMENT ON COLUMN evidence_search.probe_text IS
    'Generated (never triggered, so it cannot drift): quote, title and significance concatenated flat, space-separated. The trigram matching surface. Flat and unweighted because trigram has no weighting to give — search_vector above carries the same three fields WITH weights for the full-text half.';

-- The migration seeds no rows, so Rule 25a's row-count assertion does not apply.
-- What DOES need asserting is the shape: every statement above is
-- `IF NOT EXISTS`, and an `IF NOT EXISTS` against a pre-existing object of the
-- same name is a silent no-op — a table left over from a hand-run experiment
-- would let this migration "succeed" while leaving the wrong columns in place.
-- This block turns that into a loud failure at boot.
DO $$
DECLARE
    column_count      INTEGER;
    generated_kind    TEXT;
    generated_column  TEXT;
    about_type        TEXT;
    page_type         TEXT;
    index_count       INTEGER;
    extension_present INTEGER;
BEGIN
    SELECT count(*) INTO column_count
      FROM information_schema.columns
     WHERE table_name = 'evidence_search'
       AND column_name IN ('evidence_id', 'document_id', 'title', 'quote',
                           'significance', 'page', 'about',
                           'synced_at', 'search_vector', 'probe_text');
    IF column_count <> 10 THEN
        RAISE EXCEPTION
            'evidence_search has % of the 10 expected columns — an object of that name already existed with a different shape',
            column_count;
    END IF;

    -- Both derived columns must be GENERATED, for the same reason: a trigger
    -- can be dropped, disabled or simply not fire on a COPY, and a search
    -- surface that has drifted from the row it summarises fails by returning
    -- plausible results for the wrong text.
    FOR generated_column IN SELECT unnest(ARRAY['search_vector', 'probe_text'])
    LOOP
        SELECT is_generated INTO generated_kind
          FROM information_schema.columns
         WHERE table_name = 'evidence_search' AND column_name = generated_column;
        IF generated_kind IS DISTINCT FROM 'ALWAYS' THEN
            RAISE EXCEPTION
                'evidence_search.% is not a generated column (is_generated = %) — it could drift from the row it summarises',
                generated_column, coalesce(generated_kind, '<absent>');
        END IF;
    END LOOP;

    SELECT data_type INTO about_type
      FROM information_schema.columns
     WHERE table_name = 'evidence_search' AND column_name = 'about';
    IF about_type <> 'ARRAY' THEN
        RAISE EXCEPTION
            'evidence_search.about is % rather than an array — L2 subject filtering needs set membership, not a joined string',
            about_type;
    END IF;

    -- The R1 ruling, guarded. A table created from an EARLIER draft of this
    -- migration would carry `page INTEGER` and still pass the column-count check
    -- above, and this file's `CREATE TABLE IF NOT EXISTS` would no-op over it —
    -- after which L1c would narrow i64 to i32 against a column nobody looked at
    -- again. That is exactly the silent shape drift this block exists to catch.
    SELECT data_type INTO page_type
      FROM information_schema.columns
     WHERE table_name = 'evidence_search' AND column_name = 'page';
    IF page_type <> 'bigint' THEN
        RAISE EXCEPTION
            'evidence_search.page is % rather than bigint — L1c would have to narrow i64 to i32 (ruling R1, 2026-09-01)',
            page_type;
    END IF;

    SELECT count(*) INTO index_count
      FROM pg_indexes
     WHERE tablename = 'evidence_search'
       AND indexname IN ('idx_evidence_search_vector', 'idx_evidence_search_probe_trgm');
    IF index_count <> 2 THEN
        RAISE EXCEPTION
            'evidence_search carries % of its 2 search indexes — one half of the lexical gather would silently miss',
            index_count;
    END IF;

    SELECT count(*) INTO extension_present FROM pg_extension WHERE extname = 'pg_trgm';
    IF extension_present <> 1 THEN
        RAISE EXCEPTION 'pg_trgm is not installed — the trigram index cannot answer a "$50,000" query';
    END IF;
END $$;
