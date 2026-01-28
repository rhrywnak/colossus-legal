use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct PersonCreateRequest {
    pub id: Option<String>,
    pub name: String,
    pub role: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub affiliation: Option<String>,
    pub notes: Option<String>,
    pub date_of_birth: Option<NaiveDate>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PersonUpdateRequest {
    pub name: Option<String>,
    pub role: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub affiliation: Option<String>,
    pub notes: Option<String>,
    pub date_of_birth: Option<NaiveDate>,
}

/// Response DTO for a single person
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonDto {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Response for GET /persons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonsResponse {
    pub persons: Vec<PersonDto>,
    pub total: usize,
}
