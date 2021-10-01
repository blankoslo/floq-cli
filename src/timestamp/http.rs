use super::history::{ProjectTimestamp, Timestamp};
use crate::http_client::floq_api_domain;
use crate::http_client::{HandleInvalidToken, HandleMalformedBody, HttpClient};

use anyhow::{anyhow, Context, Result};
use chrono::{Duration, NaiveDate};
use futures::{stream::FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};
use surf::{Response, StatusCode};

#[derive(Deserialize, Debug)]
struct TimeEntry {
    minutes: i64,
}

#[derive(Serialize, Debug)]
struct TimestampedProjectsRequest {
    employee_id: u16,
    date: NaiveDate,
}

#[derive(Deserialize, Debug)]
struct TimestampedProjectsResponse {
    id: String,
    project: String,
    customer: String,
    minutes: i64,
}

impl TimestampedProjectsResponse {
    fn to_project_timestamp(&self, date: NaiveDate) -> ProjectTimestamp {
        ProjectTimestamp {
            project_id: self.id.clone(),
            project_name: self.project.clone(),
            customer_name: self.customer.clone(),
            timestamp: Timestamp {
                date,
                time: Duration::minutes(self.minutes),
            },
        }
    }
}

impl HttpClient {
    pub async fn get_timestamp_on_project_for_date(
        &self,
        project_id: &str,
        date: &NaiveDate,
    ) -> Result<Duration> {
        let url = format!(
            "{}/time_entry?select=minutes&employee=eq.{}&project=eq.{}&date=eq.{}",
            floq_api_domain(),
            self.employee_id,
            project_id,
            date.format("%Y-%m-%d"),
        );
        let mut response: Response = surf::get(url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
            .handle_floq_response()
            .with_context(|| "Noe gikk galt under henting av dine timer for et prosjekt")?;

        let minutes = response
            .body_json::<Vec<TimeEntry>>()
            .await
            .handle_malformed_body()
            .with_context(|| "Klarte ikke lese responsen fra /time_entry")?
            .iter()
            .map(|entry| entry.minutes)
            .sum();

        Ok(Duration::minutes(minutes))
    }

    pub async fn get_timestamps_for_period(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<ProjectTimestamp>> {
        let difference = to.signed_duration_since(from).num_days();

        let mut futures: FuturesUnordered<_> = (0..=difference)
            .map(|i| self.get_timestamps_for_date(from + Duration::days(i)))
            .collect();

        let mut results: Vec<Vec<ProjectTimestamp>> = vec![];
        while let Some(r) = futures.next().await {
            results.push(r?);
        }

        Ok(results.into_iter().flatten().collect())
    }

    pub async fn get_timestamps_for_date(&self, date: NaiveDate) -> Result<Vec<ProjectTimestamp>> {
        let body = TimestampedProjectsRequest {
            employee_id: self.employee_id,
            date,
        }
        .serialize(serde_json::value::Serializer)?
        .to_string();

        let url = format!("{}/rpc/projects_for_employee_for_date", floq_api_domain());
        let mut response: Response = surf::post(url)
            .body(body)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
            .handle_floq_response()
            .with_context(|| "Noe gikk galt under henting av dine timer for en dag")?;

        let response: Vec<TimestampedProjectsResponse> = response
            .body_json()
            .await
            .handle_malformed_body()
            .with_context(|| {
                "Klarte ikke å lese responsen fra /rpc/projects_for_employee_for_date"
            })?;

        Ok(response
            .iter()
            .map(|r| r.to_project_timestamp(date))
            .filter(|tp| !tp.timestamp.is_time_zero())
            .collect())
    }
}

#[derive(Serialize, Debug)]
struct TimestampRequest<'a> {
    creator: u16,
    employee: u16,
    project: &'a str,
    date: &'a NaiveDate,
    minutes: i64,
}

impl HttpClient {
    pub async fn add_timestamp(
        &self,
        project_id: &str,
        date: &NaiveDate,
        time: Duration,
    ) -> Result<()> {
        let body = TimestampRequest {
            creator: self.employee_id,
            employee: self.employee_id,
            project: project_id,
            date,
            minutes: time.num_minutes(),
        }
        .serialize(serde_json::value::Serializer)?
        .to_string();

        let response = surf::post(format!("{}/time_entry", floq_api_domain()))
            .body(body)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
            .handle_floq_response()
            .with_context(|| "Noe gikk galt under føring av timer")?;

        match response.status() {
            StatusCode::Created => Ok(()),
            sc => Err(anyhow!(
                "Fikk en annen statuskode enn forventet fra POST /time_entry {}",
                sc
            )),
        }
    }
}
