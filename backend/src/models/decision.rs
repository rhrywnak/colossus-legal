use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Decision {
    pub id: String,
    pub claim_id: Option<String>,
    pub decided_on: Option<NaiveDate>,
    pub outcome: Option<String>,
    pub judge: Option<String>,
    pub summary: Option<String>,
    pub notes: Option<String>,
}
