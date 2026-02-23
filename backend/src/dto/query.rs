use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── List endpoint response ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryListResponse {
    pub categories: Vec<QueryCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryCategory {
    pub name: String,
    pub description: String,
    pub queries: Vec<QueryInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryInfo {
    pub id: String,
    pub title: String,
    pub description: String,
    pub category: String,
}

// ─── Run endpoint response ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResultResponse {
    pub query_id: String,
    pub title: String,
    pub description: String,
    pub columns: Vec<String>,
    pub rows: Vec<HashMap<String, serde_json::Value>>,
    pub row_count: usize,
}
