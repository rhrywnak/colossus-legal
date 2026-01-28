use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Response DTO for a single harm/damage item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarmDto {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subcategory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_reference: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub caused_by_allegations: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub damages_for_counts: Vec<String>,
}

/// Response for GET /harms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarmsResponse {
    pub harms: Vec<HarmDto>,
    pub total: usize,
    pub total_damages: f64,
    pub by_category: HashMap<String, f64>,
}
