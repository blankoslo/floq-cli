use crate::http_client::{HandleInvalidToken, HttpClient, ParseBody, FLOQ_API_DOMAIN};
use crate::print::TableMaker;
use crate::user;

use std::io::Write;

use chrono::{Datelike, Duration, NaiveDate, Utc};
use clap::{Arg, ArgMatches, Command};
use serde::{Deserialize, Serialize};

use anyhow::{Context, Result};

pub const SUBCOMMAND_NAME: &str = "prosjekter";

pub fn subcommand_app() -> Command {
    Command::new("project")
        .about("Vis prosjekter")
        .arg(
            Arg::new("mine")
                .long("mine")
                .short('m')
                .default_value("true")
                .conflicts_with("alle")
                .help("Vis prosjekter du har ført timer på de siste to ukene"),
        )
        .arg(
            Arg::new("alle")
                .long("alle")
                .short('a')
                .conflicts_with("mine")
                .help("Vis alle prosjekter"),
        )
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

pub async fn execute<OUT: Write + Send>(args: &ArgMatches, mut out: OUT) -> Result<()> {
    let user = user::load_user_from_config(&mut out).await?;
    let client = HttpClient::from_user(&user);

    let all = args.contains_id("alle");
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
    table_maker.into_table(&projects).print(&mut out)?;

    Ok(())
}

impl HttpClient {
    pub async fn get_projects(&self) -> Result<Vec<Project>> {
        let url = format!(
            "{}/projects?select=id,name,active,customer{{id,name}}",
            FLOQ_API_DOMAIN
        );

        self.client
            .get(url)
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
            .handle_floq_response()
            .with_context(|| "Noe gikk galt under henting av alle prosjekter")?
            .parse_body()
            .await
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
    pub async fn get_current_timestamped_projects_for_employee(&self) -> Result<Vec<Project>> {
        let today = Utc::now().date();

        self.get_timestamped_projects_for_employee(today.naive_local())
            .await
    }

    pub async fn get_timestamped_projects_for_employee(
        &self,
        date: NaiveDate,
    ) -> Result<Vec<Project>> {
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
        let projects: Vec<ProjectForEmployeeResponse> = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", self.access_token))
            .body(body)
            .send()
            .await
            .handle_floq_response()
            .with_context(|| "Noe gikk galt under henting av dine prosjekter")?
            .parse_body()
            .await?;

        Ok(projects.into_iter().map(|r| r.into_project()).collect())
    }
}
