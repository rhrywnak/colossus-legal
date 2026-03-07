// QAEntry uniqueness constraint
CREATE CONSTRAINT qa_entry_id IF NOT EXISTS
FOR (q:QAEntry) REQUIRE q.id IS UNIQUE;

// Index for scope lookups (generic — works for any app)
CREATE INDEX qa_entry_scope IF NOT EXISTS
FOR (q:QAEntry) ON (q.scope_type, q.scope_id);

// Index for timestamp ordering
CREATE INDEX qa_entry_asked_at IF NOT EXISTS
FOR (q:QAEntry) ON (q.asked_at);

// Index for session grouping (future chat sessions)
CREATE INDEX qa_entry_session IF NOT EXISTS
FOR (q:QAEntry) ON (q.session_id);
