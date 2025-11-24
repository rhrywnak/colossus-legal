use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ClaimCreateRequest {
    pub text: String,
    pub made_by: Option<String>,
    pub first_made: Option<String>,
    pub category: Option<String>,
    pub verified: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClaimUpdateRequest {
    pub text: Option<String>,
    pub made_by: Option<String>,
    pub first_made: Option<String>,
    pub category: Option<String>,
    pub verified: Option<bool>,
}
