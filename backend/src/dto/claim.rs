use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClaimDto {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClaimCreateRequest {
    pub title: String,
    pub description: Option<String>,
    pub status: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClaimUpdateRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
}
