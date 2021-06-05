use crate::cmd::Subcommand;
use crate::http_client::HttpClient;
use crate::http_client::FLOQ_API_DOMAIN;
use crate::print::TableMaker;
use crate::user;

use std::{error::Error, io::Write};

use async_trait::async_trait;
use chrono::{Datelike, Duration, NaiveDate, Utc};
use clap::{App, Arg, ArgMatches};
use serde::{Deserialize, Serialize};
use surf::Response;

const SUBCOMMAND_NAME: &str = "prosjekter";

pub fn subcommand_app<'help>() -> App<'help> {
    App::new(SUBCOMMAND_NAME)
        .about("Vis prosjekter")
        .arg(
            Arg::new("mine")
                .long("mine")
                .short('m')
                .default_value("true")
                .conflicts_with("alle")
                .about("Vis prosjekter du har ført timer på de siste to ukene"),
        )
        .arg(
            Arg::new("alle")
                .long("alle")
                .short('a')
                .conflicts_with("mine")
                .about("Vis alle prosjekter"),
        )
}

pub fn subcommand<T: Write + Send>() -> Box<dyn Subcommand<T>> {
    Box::new(ProjectsSubcommand {})
}

struct ProjectsSubcommand;

#[async_trait(?Send)]
impl<T: Write + Send> Subcommand<T> for ProjectsSubcommand {
    fn matches(&self, matches: &ArgMatches) -> bool {
        matches.subcommand_name() == Some(SUBCOMMAND_NAME)
    }

    async fn execute(&self, matches: &clap::ArgMatches, out: &mut T) -> Result<(), Box<dyn Error>> {
        let user = user::load_user_from_config(out).await?;
        let client = HttpClient::from_user(&user);

        let all = matches.is_present("alle");
        let mut projects = if all {
            client.get_projects().await?
        } else {
            client
                .get_current_timestamped_projects_for_employee()
                .await?
        };
        projects.sort_by(|p1, p2| p1.id.cmp(&p2.id));

        let mut table_maker = TableMaker::new();
        table_maker.static_titles(vec!["ID", "KUNDE", "BESKRIVELSE"]);
        table_maker
            .with(Box::new(|p: &Project| p.id.clone()))
            .with(Box::new(|p| p.customer.name.clone()))
            .with(Box::new(|p| p.name.clone()));
        table_maker.into_table(&projects).print(out)?;

        Ok(())
    }
}

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

impl HttpClient {
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

impl HttpClient {
    pub async fn get_current_timestamped_projects_for_employee(
        &self,
    ) -> Result<Vec<Project>, Box<dyn Error>> {
        let today = Utc::now().date();

        self.get_timestamped_projects_for_employee(today.naive_local())
            .await
    }

    pub async fn get_timestamped_projects_for_employee(
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
