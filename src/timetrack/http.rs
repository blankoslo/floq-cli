use crate::http_client::HTTPClient;
use crate::http_client::FLOQ_API_DOMAIN;

use std::error::Error;

use chrono::{Datelike, Duration, NaiveDate, Utc};
use futures::future;
use serde::{Deserialize, Serialize};
use surf::Response;

use super::Timetrack;

#[derive(Serialize, Debug)]
struct TimetrackedProjectsRequest {
    employee_id: u16,
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
            project_id: self.id.clone(),
            project_name: self.project.clone(),
            customer: self.customer.clone(),
            date,
            time: Duration::minutes(self.minutes as i64),
        }
    }
}

impl HTTPClient {
    pub async fn get_current_week_timetracking(&self) -> Result<Vec<Timetrack>, Box<dyn Error>> {
        let today = Utc::now().date();
        let days_from_monday = today.weekday().num_days_from_monday();

        let monday = today.naive_local() - Duration::days(1) * days_from_monday as i32;
        let sunday = monday + Duration::days(6);

        self.get_timetracking_for_period(monday, sunday).await
    }

    pub async fn get_timetracking_for_period(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<Timetrack>, Box<dyn Error>> {
        let difference = to.signed_duration_since(from).num_days();

        let results: Vec<Result<Vec<Timetrack>, Box<dyn Error>>> = future::join_all(
            (0..=difference).map(|i| self.get_timetracking_for_day(from + Duration::days(i))),
        )
        .await
        .into_iter()
        .collect();

        let results: Result<Vec<Vec<Timetrack>>, Box<dyn Error>> = results.into_iter().collect();

        results.map(|values| values.into_iter().flatten().collect())
    }

    pub async fn get_timetracking_for_day(
        &self,
        date: NaiveDate,
    ) -> Result<Vec<Timetrack>, Box<dyn std::error::Error>> {
        let body = TimetrackedProjectsRequest {
            employee_id: self.employee_id,
            date,
        }
        .serialize(serde_json::value::Serializer)?
        .to_string();

        let url = format!("{}/rpc/projects_for_employee_for_date", FLOQ_API_DOMAIN);
        let mut response: Response = surf::post(url)
            .body(body)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await?;

        let response: Vec<TimetrackedProjectsResponse> = response.body_json().await?;

        Ok(response
            .iter()
            .map(|r| r.to_timetracked_project(date))
            .filter(|tp| tp.time != Duration::zero())
            .collect())
    }
}

#[derive(Serialize, Debug)]
struct TimetrackRequest<'a> {
    creator: u16,
    employee: u16,
    project: &'a str,
    date: NaiveDate,
    minutes: u16,
}

impl HTTPClient {
    pub async fn timetrack(
        &self,
        project_id: &str,
        date: NaiveDate,
        time: Duration,
    ) -> Result<(), Box<dyn Error>> {
        let body = TimetrackRequest {
            creator: self.employee_id,
            employee: self.employee_id,
            project: project_id,
            date,
            minutes: time.num_minutes() as u16,
        }
        .serialize(serde_json::value::Serializer)?
        .to_string();

        surf::post(format!("{}/time_entry", FLOQ_API_DOMAIN))
            .body(body)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await?;

        // TODO handle 4xx and 5xx status codes

        Ok(())
    }
}
