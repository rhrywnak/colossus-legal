CREATE TABLE IF NOT EXISTS pipeline_steps (
    id SERIAL PRIMARY KEY,
    document_id TEXT NOT NULL REFERENCES documents(id),
    step_name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'running',
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    duration_secs DOUBLE PRECISION,
    triggered_by TEXT,
    input_params JSONB DEFAULT '{}',
    result_summary JSONB DEFAULT '{}',
    error_message TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_pipeline_steps_document ON pipeline_steps(document_id);
CREATE INDEX IF NOT EXISTS idx_pipeline_steps_step ON pipeline_steps(step_name);
CREATE INDEX IF NOT EXISTS idx_pipeline_steps_status ON pipeline_steps(status, started_at);
