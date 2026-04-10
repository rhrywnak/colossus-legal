-- F5: Review edit history for audit trail
CREATE TABLE IF NOT EXISTS review_edit_history (
    id SERIAL PRIMARY KEY,
    item_id INTEGER NOT NULL REFERENCES extraction_items(id),
    field_changed TEXT NOT NULL,
    old_value TEXT,
    new_value TEXT,
    changed_by TEXT NOT NULL,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_review_edit_history_item ON review_edit_history(item_id);
