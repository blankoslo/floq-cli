use std::time::SystemTime;

use chrono::{Datelike, DateTime, Duration, NaiveDate, Utc};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Timetrack {
    id: String,
    project: String,
    customer: String,
    time: Duration,
}

#[derive(Serialize, Debug)]
struct Request {
    employee_id: u32,
    date: NaiveDate,
}

#[derive(Deserialize, Debug)]
struct Response {
    id: String,
    project: String,
    customer: String,
    minutes: u32,
}

impl Response {
    fn to_timetracked_project(&self) -> Timetrack {
        Timetrack {
            id: self.id.clone(),
            project: self.project.clone(),
            customer: self.customer.clone(),
            time: Duration::minutes(self.minutes.into()),
        }
    }
}

pub struct HTTPClient {
    client: Client
}

impl HTTPClient {
    pub fn new() -> Self {
        Self {
            client: Client::new()
        }
    }

    pub fn get_current_week_timetracks(
        &self,
        employee_id: u32,
    ) -> Result<Vec<Timetrack>, Box<dyn std::error::Error>> {
        let now: DateTime<Utc> = DateTime::from(SystemTime::now());
        let today = now.date();
        let days_from_monday = today
            .weekday()
            .num_days_from_monday();

        let monday = today.naive_local() - Duration::days(1) * days_from_monday as i32;

        let results: Result<Vec<Vec<Timetrack>>, Box<dyn std::error::Error>> =
            (0..7).map(|i| monday + Duration::days(1) * i as i32)
                .map(|day| self.get_timetracks_for_day(employee_id, day))
                .collect();

        Ok(results?.into_iter().flatten().collect())
    }

    pub fn get_timetracks_for_day(
        &self,
        employee_id: u32,
        date: NaiveDate,
    ) -> Result<Vec<Timetrack>, Box<dyn std::error::Error>> {

        let body = Request {
            employee_id,
            date,
        }.serialize(serde_json::value::Serializer)?.to_string();

        let response: reqwest::blocking::Response = self.client.post("https://api-blank.floq.no/rpc/projects_for_employee_for_date")
            .body(body)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJyb2xlIjoiZW1wbG95ZWUiLCJlbWFpbCI6InRyb25kLm95ZG5hQGJsYW5rLm5vIiwiaWF0IjoxNTk5NTYzOTkxLCJleHAiOjE2MDAxNjg3OTF9.rjdN1vvCpo0s9KkLJW3aySh8921VzT63czRrprE7JdA"))
            .send()?;

        let response: Vec<Response> = response.json()?;

        Ok(response.iter()
            .map(|r| r.to_timetracked_project())
            .filter(|tp| tp.time != Duration::zero())
            .collect())
    }
}

