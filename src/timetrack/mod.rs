use chrono::{Duration, NaiveDate};

mod http;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Timetrack {
    pub project_id: String,
    pub project_name: String,
    pub customer: String,
    pub date: NaiveDate,
    pub time: Duration,
}
