use std::error::Error;
use std::time::SystemTime;

use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use futures::join;
use surf::Response;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Timetrack {
    id: String,
    project: String,
    customer: String,
    date: NaiveDate,
    time: Duration,
}

#[derive(Serialize, Debug)]
struct TimetrackedProjectsRequest {
    employee_id: u32,
    date: NaiveDate,
}

#[derive(Deserialize, Debug)]
struct TimetrackedProjectsResponse {
    id: String,
    project: String,
    customer: String,
    minutes: u16,
}

impl TimetrackedProjectsResponse {
    fn to_timetracked_project(&self, date: NaiveDate) -> Timetrack {
        Timetrack {
            id: self.id.clone(),
            project: self.project.clone(),
            customer: self.customer.clone(),
            date,
            time: Duration::minutes(self.minutes as i64),
        }
    }
}

pub async fn get_current_week_timetracking(
    employee_id: u32,
) -> Result<Vec<Timetrack>, Box<dyn Error>> {
    let now: DateTime<Utc> = DateTime::from(SystemTime::now());
    let today = now.date();
    let days_from_monday = today.weekday().num_days_from_monday();

    let monday = today.naive_local() - Duration::days(1) * days_from_monday as i32;

    let results = join!(
        get_timetracking_for_day(employee_id, monday),
        get_timetracking_for_day(employee_id, monday + Duration::days(1)),
        get_timetracking_for_day(employee_id, monday + Duration::days(2)),
        get_timetracking_for_day(employee_id, monday + Duration::days(3)),
        get_timetracking_for_day(employee_id, monday + Duration::days(4)),
        get_timetracking_for_day(employee_id, monday + Duration::days(5)),
        get_timetracking_for_day(employee_id, monday + Duration::days(6)),
    );
    let timetrackings: Vec<Vec<Timetrack>> = vec![
        results.0?, results.1?, results.2?, results.3?, results.4?, results.5?, results.6?,
    ];

    Ok(timetrackings.into_iter().flatten().collect())
}

pub async fn get_timetracking_for_day(
    employee_id: u32,
    date: NaiveDate,
) -> Result<Vec<Timetrack>, Box<dyn std::error::Error>> {
    let body = TimetrackedProjectsRequest { employee_id, date }
        .serialize(serde_json::value::Serializer)?
        .to_string();

    let mut response: Response = surf::post("https://api-blank.floq.no/rpc/projects_for_employee_for_date")
        .body(body)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("Authorization", format!("Bearer {}", super::super::TOKEN))
        .send()
        .await?;

    let response: Vec<TimetrackedProjectsResponse> = response.body_json().await?;

    Ok(response
        .iter()
        .map(|r| r.to_timetracked_project(date))
        .filter(|tp| tp.time != Duration::zero())
        .collect())
}

#[derive(Serialize, Debug)]
struct TimetrackRequest {
    creator: u32,
    employee: u32,
    project: String,
    date: NaiveDate,
    minutes: u16,
}

pub async fn timetrack(
    employee_id: u32,
    project_id: String,
    date: NaiveDate,
    time: Duration,
) -> Result<(), Box<dyn Error>> {
    let body = TimetrackRequest {
        creator: employee_id,
        employee: employee_id,
        project: project_id,
        date,
        minutes: time.num_minutes() as u16,
    }
    .serialize(serde_json::value::Serializer)?
    .to_string();

    surf::post("https://api-blank.floq.no/time_entry")
        .body(body)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", super::super::TOKEN))
        .send()
        .await?;
    
    // unsure if this will return error on 4xx or 5xx status codes

    Ok(())
}
