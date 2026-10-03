use super::history::{ProjectTimestamp, Timestamp};
use crate::http_client::{AuthorizedHttpClient, floq_api_domain};
use crate::http_client::{HandleInvalidToken, HandleMalformedBody};

use anyhow::{Context, Result, anyhow};
use chrono::{Duration, NaiveDate};
use futures::{StreamExt, stream::FuturesUnordered};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

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

impl AuthorizedHttpClient {
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

        Ok(self
            .client
            .get(url)
            .send()
            .await
            .handle_floq_response()
            .await
            .context("Noe gikk galt under henting av dine timer for et prosjekt")?
            .json::<Vec<TimeEntry>>()
            .await
            .handle_malformed_body()
            .context("Klarte ikke lese responsen fra /time_entry")?
            .first()
            .map(|e| Duration::minutes(e.minutes))
            .unwrap_or_else(Duration::zero))
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

        Ok(self
            .client
            .post(url)
            .body(body)
            .send()
            .await
            .handle_floq_response()
            .await
            .context("Noe gikk galt under henting av dine timer for en dag")?
            .json::<Vec<TimestampedProjectsResponse>>()
            .await
            .handle_malformed_body()
            .context("Klarte ikke å lese responsen fra /rpc/projects_for_employee_for_date")?
            .into_iter()
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

impl AuthorizedHttpClient {
    pub async fn internal_set_timestamp(
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

        let response = self
            .client
            .post(format!(
                "{}/time_entry?on_conflict=employee,project,date",
                floq_api_domain()
            ))
            .body(body)
            .header("Prefer", "resolution=merge-duplicates")
            .send()
            .await
            .handle_floq_response()
            .await
            .context("Noe gikk galt under føring av timer")?;

        match response.status() {
            // Upsert returns either 200 OK or 201 Created depending on whether a new row was inserted or an existing row was updated.
            StatusCode::OK | StatusCode::CREATED => Ok(()),
            sc => Err(anyhow!(
                "Fikk en annen statuskode enn forventet fra POST /time_entry {}",
                sc
            )),
        }
    }
}
