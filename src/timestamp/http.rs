use crate::env::ENV;
use crate::http_client::{FloqApiClient, HandleInvalidToken, HandleMalformedBody};

use anyhow::{Context, Result, anyhow};
use chrono::{Duration, NaiveDate};
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
pub struct ProjectTimeEntry {
    pub id: String,
    #[allow(unused)]
    pub project: String,
    #[allow(unused)]
    pub customer: String,
    pub minutes: i64,
}

impl FloqApiClient {
    pub async fn get_timestamp_on_project_for_date(
        &self,
        employee_id: u16,
        project_id: &str,
        date: &NaiveDate,
    ) -> Result<Duration> {
        let url = format!(
            "{}/time_entry?select=minutes&employee=eq.{}&project=eq.{}&date=eq.{}",
            ENV.api_domain,
            employee_id,
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

    pub async fn get_timestamps_for_date(&self, employee_id: u16, date: NaiveDate) -> Result<Vec<ProjectTimeEntry>> {
        let body = TimestampedProjectsRequest { employee_id, date }
            .serialize(serde_json::value::Serializer)?
            .to_string();
        let url = format!("{}/rpc/projects_for_employee_for_date", ENV.api_domain);

        self.client
            .post(url)
            .body(body)
            .send()
            .await
            .handle_floq_response()
            .await
            .context("Noe gikk galt under henting av dine timer for en dag")?
            .json::<Vec<ProjectTimeEntry>>()
            .await
            .handle_malformed_body()
            .context("Klarte ikke å lese responsen fra /rpc/projects_for_employee_for_date")
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

impl FloqApiClient {
    pub async fn set_timestamp(
        &self,
        employee_id: u16,
        project_id: &str,
        date: &NaiveDate,
        time: &Duration,
    ) -> Result<()> {
        let body = TimestampRequest {
            creator: employee_id,
            employee: employee_id,
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
                ENV.api_domain,
            ))
            .body(body)
            .header("Prefer", "resolution=merge-duplicates")
            .send()
            .await
            .handle_floq_response()
            .await
            .context("Noe gikk galt under føring av timer")?;

        match response.status() {
            // Upsert returns either 200 OK or 201 Created
            // depending on whether a new row was inserted or an existing row was updated.
            StatusCode::OK | StatusCode::CREATED => Ok(()),
            sc => Err(anyhow!(
                "Fikk en annen statuskode enn forventet fra POST /time_entry {}",
                sc
            )),
        }
    }

    pub async fn set_timestamp_and_get_previous(
        &self,
        employee_id: u16,
        project_id: &str,
        date: &NaiveDate,
        time: &Duration,
    ) -> Result<Duration> {
        let previous_time = self
            .get_timestamp_on_project_for_date(employee_id, project_id, date)
            .await?;
        self.set_timestamp(employee_id, project_id, date, time).await?;
        Ok(previous_time)
    }
}
