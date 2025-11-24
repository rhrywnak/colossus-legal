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
