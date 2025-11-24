use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Claim {
    pub id: String,
    pub text: String,
    pub made_by: Option<String>,
    pub first_made: Option<NaiveDate>,
    pub category: Option<String>,
    pub verified: Option<bool>,
}
