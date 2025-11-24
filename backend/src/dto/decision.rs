use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct DecisionCreateRequest {
    pub id: Option<String>,
    pub claim_id: Option<String>,
    pub decided_on: Option<NaiveDate>,
    pub outcome: Option<String>,
    pub judge: Option<String>,
    pub summary: Option<String>,
    pub notes: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DecisionUpdateRequest {
    pub claim_id: Option<String>,
    pub decided_on: Option<NaiveDate>,
    pub outcome: Option<String>,
    pub judge: Option<String>,
    pub summary: Option<String>,
    pub notes: Option<String>,
}
