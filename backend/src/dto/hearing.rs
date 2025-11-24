use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct HearingCreateRequest {
    pub id: Option<String>,
    pub claim_id: Option<String>,
    pub scheduled_date: Option<NaiveDate>,
    pub location: Option<String>,
    pub judge: Option<String>,
    pub notes: Option<String>,
    pub outcome_summary: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HearingUpdateRequest {
    pub claim_id: Option<String>,
    pub scheduled_date: Option<NaiveDate>,
    pub location: Option<String>,
    pub judge: Option<String>,
    pub notes: Option<String>,
    pub outcome_summary: Option<String>,
}
