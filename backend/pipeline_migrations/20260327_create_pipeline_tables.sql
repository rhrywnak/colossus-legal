-- Pipeline tables for the document extraction pipeline.
-- These live in colossus_legal_v2 (clean room, separate from colossus_legal).

-- Document metadata and lifecycle state
CREATE TABLE IF NOT EXISTS documents (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_hash TEXT NOT NULL,
    document_type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'UPLOADED',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Extracted text, page by page
CREATE TABLE IF NOT EXISTS document_text (
    document_id TEXT NOT NULL REFERENCES documents(id),
    page_number INTEGER NOT NULL,
    text_content TEXT NOT NULL,
    PRIMARY KEY (document_id, page_number)
);

-- Each LLM extraction run (one per pass per document)
CREATE TABLE IF NOT EXISTS extraction_runs (
    id SERIAL PRIMARY KEY,
    document_id TEXT NOT NULL REFERENCES documents(id),
    pass_number INTEGER NOT NULL,
    model_name TEXT NOT NULL,
    input_tokens INTEGER,
    output_tokens INTEGER,
    cost_usd NUMERIC(10, 4),
    raw_output JSONB NOT NULL,
    schema_version TEXT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ,
    status TEXT NOT NULL DEFAULT 'RUNNING'
);

-- Individual extracted items with verification status
CREATE TABLE IF NOT EXISTS extraction_items (
    id SERIAL PRIMARY KEY,
    run_id INTEGER NOT NULL REFERENCES extraction_runs(id),
    document_id TEXT NOT NULL REFERENCES documents(id),
    entity_type TEXT NOT NULL,
    item_data JSONB NOT NULL,
    verbatim_quote TEXT,
    grounding_status TEXT,
    grounded_page INTEGER,
    review_status TEXT NOT NULL DEFAULT 'PENDING',
    reviewed_by TEXT,
    reviewed_at TIMESTAMPTZ,
    review_notes TEXT
);

-- Extracted relationships
CREATE TABLE IF NOT EXISTS extraction_relationships (
    id SERIAL PRIMARY KEY,
    run_id INTEGER NOT NULL REFERENCES extraction_runs(id),
    document_id TEXT NOT NULL REFERENCES documents(id),
    from_item_id INTEGER NOT NULL REFERENCES extraction_items(id),
    to_item_id INTEGER NOT NULL REFERENCES extraction_items(id),
    relationship_type TEXT NOT NULL,
    properties JSONB,
    review_status TEXT NOT NULL DEFAULT 'PENDING',
    reviewed_by TEXT,
    reviewed_at TIMESTAMPTZ,
    tier INTEGER NOT NULL DEFAULT 1
);

-- Pipeline configuration per document
CREATE TABLE IF NOT EXISTS pipeline_config (
    document_id TEXT PRIMARY KEY REFERENCES documents(id),
    pass1_model TEXT NOT NULL DEFAULT 'claude-sonnet-4-6',
    pass2_model TEXT,
    pass1_max_tokens INTEGER NOT NULL DEFAULT 32000,
    pass2_max_tokens INTEGER,
    schema_file TEXT NOT NULL,
    admin_instructions TEXT,
    prior_context_doc_ids TEXT[],
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for common query patterns
CREATE INDEX IF NOT EXISTS idx_documents_status ON documents(status);
CREATE INDEX IF NOT EXISTS idx_documents_type ON documents(document_type);
CREATE INDEX IF NOT EXISTS idx_extraction_runs_document ON extraction_runs(document_id);
CREATE INDEX IF NOT EXISTS idx_extraction_items_document ON extraction_items(document_id);
CREATE INDEX IF NOT EXISTS idx_extraction_items_run ON extraction_items(run_id);
CREATE INDEX IF NOT EXISTS idx_extraction_items_review ON extraction_items(review_status);
CREATE INDEX IF NOT EXISTS idx_extraction_relationships_document ON extraction_relationships(document_id);
CREATE INDEX IF NOT EXISTS idx_extraction_relationships_run ON extraction_relationships(run_id);
