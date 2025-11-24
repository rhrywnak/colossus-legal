use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Hearing {
    pub id: String,
    pub claim_id: Option<String>,
    pub scheduled_date: Option<NaiveDate>,
    pub location: Option<String>,
    pub judge: Option<String>,
    pub notes: Option<String>,
    pub outcome_summary: Option<String>,
}
