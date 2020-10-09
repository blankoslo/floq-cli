use std::error::Error;
use std::time::SystemTime;

use super::HTTPClient;

use chrono::{DateTime, Datelike, Duration, Utc};
use serde::Deserialize;
use surf::Response;

#[derive(Deserialize, Debug)]
pub struct Project {
    id: String,
    name: String,
    active: bool,
    customer: Customer,
}

#[derive(Deserialize, Debug)]
pub struct Customer {
    id: String,
    name: String,
}

impl HTTPClient {
    pub async fn get_projects(&self) -> Result<Vec<Project>, Box<dyn Error>> {
        let mut response: Response =
            surf::get("https://api-blank.floq.no/projects?select=id,name,active,customer{id,name}")
                .header("Accept", "application/json")
                .header("Authorization", format!("Bearer {}", self.bearer_token))
                .send()
                .await?;

        let projects: Vec<Project> = response.body_json().await?;

        Ok(projects)
    }

    pub async fn get_current_timetracked_projects_for_employee(
        &self,
        employee_id: u32,
    ) -> Result<Vec<Project>, Box<dyn Error>> {
        let now: DateTime<Utc> = DateTime::from(SystemTime::now());
        let today = now.date();
        let days_from_monday = today.weekday().num_days_from_monday();

        let sunday = today.naive_local() + Duration::days(1) * days_from_monday as i32;

        let timetrackings = self.get_timetracking_for_day(employee_id, sunday).await?;

        Ok(timetrackings
            .into_iter()
            .map(|t| Project {
                id: t.id,
                name: t.project,
                customer: Customer {
                    id: t.customer.clone(),
                    name: format!("Customer {}", t.customer),
                },
                active: true,
            })
            .collect())
    }
}
