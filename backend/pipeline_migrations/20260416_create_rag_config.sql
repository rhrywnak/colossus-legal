-- Creates the rag_config table and seeds it with Awad v. CFS case data.
--
-- Replaces hardcoded document_aliases HashMap and person_names Vec in
-- main.rs::build_rag_pipeline. Case-specific data belongs in the database,
-- not source code.
--
-- Per v5_2 Part 6 (rag_config table) and Part 11 (main.rs rewrite).

CREATE TABLE IF NOT EXISTS rag_config (
    id           SERIAL      PRIMARY KEY,
    config_key   TEXT        NOT NULL UNIQUE,
    config_value JSONB       NOT NULL,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by   TEXT
);

-- Seed document aliases. 14 entries covering the five case documents:
--   doc-phillips-discovery-response
--   doc-cfs-interrogatory-response
--   doc-awad-complaint
--   doc-penzien-reply-brief-310660
--   doc-penzien-coa-brief-300891
--   doc-phillips-coa-response-300891
-- ON CONFLICT DO NOTHING keeps this migration idempotent.
INSERT INTO rag_config (config_key, config_value) VALUES
    ('document_aliases', '{
        "phillips discovery": "doc-phillips-discovery-response",
        "phillips response": "doc-phillips-discovery-response",
        "cfs interrogatory": "doc-cfs-interrogatory-response",
        "cfs response": "doc-cfs-interrogatory-response",
        "complaint": "doc-awad-complaint",
        "awad complaint": "doc-awad-complaint",
        "penzien reply": "doc-penzien-reply-brief-310660",
        "reply brief": "doc-penzien-reply-brief-310660",
        "penzien brief": "doc-penzien-coa-brief-300891",
        "penzien appeal": "doc-penzien-coa-brief-300891",
        "appellant brief": "doc-penzien-coa-brief-300891",
        "phillips coa": "doc-phillips-coa-response-300891",
        "phillips appeal": "doc-phillips-coa-response-300891",
        "appellee response": "doc-phillips-coa-response-300891"
    }'::jsonb)
ON CONFLICT (config_key) DO NOTHING;

INSERT INTO rag_config (config_key, config_value) VALUES
    ('person_names', '[
        "George Phillips",
        "Emil Awad",
        "Marie Awad",
        "Charles Penzien",
        "Catholic Family Service"
    ]'::jsonb)
ON CONFLICT (config_key) DO NOTHING;
