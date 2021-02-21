use std::error::Error;

use super::HTTPClient;
use super::FLOQ_API_DOMAIN;

use chrono::{Datelike, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use surf::Response;

#[derive(Deserialize, Debug)]
pub struct Project {
    pub id: String,
    pub name: String,
    active: bool,
    pub customer: Customer,
}

#[derive(Deserialize, Debug)]
pub struct Customer {
    pub id: String,
    pub name: String,
}

impl HTTPClient {
    pub async fn get_projects(&self) -> Result<Vec<Project>, Box<dyn Error>> {
        let url = format!(
            "{}/projects?select=id,name,active,customer{{id,name}}",
            FLOQ_API_DOMAIN
        );
        let mut response: Response = surf::get(url)
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await?;

        let projects: Vec<Project> = response.body_json().await?;

        Ok(projects)
    }
}

#[derive(Serialize, Debug)]
struct ProjectsForEmployeeRequest {
    employee_id: u16,
    date_range: String,
}

#[derive(Deserialize, Debug)]
struct ProjectForEmployeeResponse {
    id: String,
    name: String,
    active: bool,
    customer_id: String,
    customer_name: String,
}

impl ProjectForEmployeeResponse {
    fn into_project(self) -> Project {
        Project {
            id: self.id,
            name: self.name,
            active: self.active,
            customer: Customer {
                id: self.customer_id,
                name: self.customer_name,
            },
        }
    }
}

impl HTTPClient {
    pub async fn get_current_timetracked_projects_for_employee(
        &self,
    ) -> Result<Vec<Project>, Box<dyn Error>> {
        let today = Utc::now().date();

        self.get_timetracked_projects_for_employee(today.naive_local())
            .await
    }

    pub async fn get_timetracked_projects_for_employee(
        &self,
        date: NaiveDate,
    ) -> Result<Vec<Project>, Box<dyn Error>> {
        let lower = date - Duration::weeks(2);
        let upper = date + Duration::days(1) * (6 - date.weekday().num_days_from_monday() as i32); // sunday of the same week as date

        let body = ProjectsForEmployeeRequest {
            employee_id: self.employee_id,
            date_range: format!(
                "({}, {})",
                lower.format("%Y-%m-%d"),
                upper.format("%Y-%m-%d")
            ),
        }
        .serialize(serde_json::value::Serializer)?
        .to_string();

        let url = format!(
            "{}/rpc/projects_info_for_employee_in_period",
            FLOQ_API_DOMAIN
        );
        let mut response: Response = surf::post(url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", self.access_token))
            .body(body)
            .send()
            .await?;

        let projects: Vec<ProjectForEmployeeResponse> = response.body_json().await?;

        Ok(projects.into_iter().map(|r| r.into_project()).collect())
    }
}
