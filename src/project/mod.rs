use crate::config::{FloqConfig, ProjectAlias};
use crate::env::ENV;
use crate::http_client::{FloqApiClient, HandleInvalidToken, HandleMalformedBody};
use crate::print::TableMaker;
use crate::session::FloqSessionHandler;

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io::Write;

use chrono::{Datelike, Duration, NaiveDate, Utc};
use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};

use anyhow::{Context, Result, anyhow};
use tokio::task::JoinSet;

#[derive(Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Project {
    pub id: String,
    pub name: String,
    #[allow(unused)]
    active: bool,
    pub customer: Customer,
}

#[derive(Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Customer {
    #[allow(unused)]
    pub id: String,
    pub name: String,
}

#[derive(Parser)]
#[command(about, long_about = None)]
pub struct ProjectCommand {
    #[command(subcommand)]
    command: Option<ProjectSubcommand>,
}

#[derive(Subcommand)]
pub enum ProjectSubcommand {
    /// List opp mine prosjekter (default)
    Mine,
    /// List opp alle prosjekter
    Alle,
    /// Opprett prosjektalias
    Alias(AliasCommand),
}

#[derive(Parser)]
#[command(about, long_about = None)]
pub struct AliasCommand {
    #[command(subcommand)]
    command: Option<AliasSubcommand>,
}

#[derive(Subcommand)]
pub enum AliasSubcommand {
    /// List opp prosjektalias (default)
    List,
    /// Opprette et nytt prosjektalias
    Opprett(OpprettAliasArgs),
    /// Slett et eksisterende prosjektalias
    Slett(SlettAliasArgs),
}

#[derive(Args)]
pub struct OpprettAliasArgs {
    /// Prosjektet du ønsker å opprette et alias for
    prosjekt: String,

    /// Alias for prosjekt
    alias: String,
}

#[derive(Args)]
pub struct SlettAliasArgs {
    /// Prosjektaliaset du ønkser å slette
    alias: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct PrintProject {
    project: Project,
    alias: Option<String>,
}

pub async fn execute<T: Write + Send>(
    cmd: ProjectCommand,
    out: &mut T,
    session_handler: &mut FloqSessionHandler,
) -> Result<()> {
    match cmd.command {
        None | Some(ProjectSubcommand::Mine) => execute_list_mine(out, session_handler).await,
        Some(ProjectSubcommand::Alle) => execute_list_all(out, session_handler).await,
        Some(ProjectSubcommand::Alias(cmd)) => execute_alias(cmd, out, session_handler).await,
    }
}

async fn execute_list_all<T: Write + Send>(out: &mut T, session_handler: &mut FloqSessionHandler) -> Result<()> {
    let session = session_handler.open(out, Duration::minutes(1)).await?;
    let client = FloqApiClient::from_session(session);
    let mut project_alias = FloqConfig::project_alias().await?.into_vec();

    let mut projects: Vec<PrintProject> = client
        .get_projects()
        .await?
        .into_iter()
        .map(|project| {
            let alias_index = project_alias
                .iter()
                .enumerate()
                .find(|(_, pa)| pa.project_id == project.id);
            match alias_index {
                Some((i, _)) => PrintProject {
                    project,
                    alias: Some(project_alias.remove(i).alias),
                },
                None => PrintProject { project, alias: None },
            }
        })
        .collect();
    projects.sort();

    let mut table_maker = TableMaker::new();
    table_maker.static_titles(vec!["ID", "KUNDE", "BESKRIVELSE", "ALIAS"]);
    table_maker
        .with(Box::new(|p: &PrintProject| p.project.id.clone()))
        .with(Box::new(|p| p.project.customer.name.clone()))
        .with(Box::new(|p| p.project.name.clone()))
        .with(Box::new(|p| p.alias.clone().unwrap_or_default()));
    table_maker.into_table(&projects).print(out)?;

    Ok(())
}

async fn execute_list_mine<T: Write + Send>(out: &mut T, session_handler: &mut FloqSessionHandler) -> Result<()> {
    let session = session_handler.open(out, Duration::minutes(1)).await?;
    let client = FloqApiClient::from_session(session);

    let mut set: JoinSet<Result<Vec<PrintProject>, anyhow::Error>> = JoinSet::new();
    let c1 = client.clone();
    set.spawn(async move {
        let employee_id = c1.get_logged_in_employee().await?.id;
        Ok(c1
            .get_current_timestamped_projects_for_employee(employee_id)
            .await?
            .into_iter()
            .map(|project| PrintProject { project, alias: None })
            .collect())
    });
    set.spawn(async move {
        let mut project_alias = FloqConfig::project_alias().await?.into_vec();
        if project_alias.is_empty() {
            Ok(vec![])
        } else {
            let projects = client.get_projects().await?;
            Ok(projects
                .into_iter()
                .filter_map(|project| {
                    let alias_index = project_alias
                        .iter()
                        .enumerate()
                        .find(|(_, pa)| pa.project_id == project.id);
                    #[allow(clippy::manual_map)] // due to both mutable and immutable borrow if Option::map is used
                    match alias_index {
                        Some((i, _)) => Some(PrintProject {
                            project,
                            alias: Some(project_alias.remove(i).alias),
                        }),
                        None => None,
                    }
                })
                .collect())
        }
    });
    let mut projects: HashMap<String, PrintProject> = HashMap::new();
    while let Some(next_projects) = set.join_next().await {
        for next in next_projects?? {
            match projects.entry(next.project.id.clone()) {
                Entry::Occupied(mut entry) => {
                    let value = entry.get_mut();
                    if value.alias.is_none() && next.alias.is_some() {
                        value.alias = next.alias;
                    }
                }
                Entry::Vacant(entry) => {
                    entry.insert(next);
                }
            }
        }
    }
    let mut projects: Vec<PrintProject> = projects.into_values().collect();
    projects.sort();

    let mut table_maker = TableMaker::new();
    table_maker.static_titles(vec!["ID", "KUNDE", "BESKRIVELSE", "ALIAS"]);
    table_maker
        .with(Box::new(|p: &PrintProject| p.project.id.clone()))
        .with(Box::new(|p| p.project.customer.name.clone()))
        .with(Box::new(|p| p.project.name.clone()))
        .with(Box::new(|p| p.alias.clone().unwrap_or_default()));
    table_maker.into_table(&projects).print(out)?;

    Ok(())
}

async fn execute_alias<T: Write + Send>(
    cmd: AliasCommand,
    out: &mut T,
    session_handler: &mut FloqSessionHandler,
) -> Result<()> {
    match cmd.command {
        None | Some(AliasSubcommand::List) => {
            let project_alias = FloqConfig::project_alias().await?.into_vec();

            if project_alias.is_empty() {
                return Err(anyhow!("Du har ikke opprettet alias enda"));
            }

            let mut table_maker = TableMaker::new();
            table_maker.static_titles(vec!["PROSJEKTID", "ALIAS"]);
            table_maker
                .with(Box::new(|p: &ProjectAlias| p.project_id.clone()))
                .with(Box::new(|p| p.alias.clone()));
            table_maker.into_table(&project_alias).print(out)?;

            Ok(())
        }
        Some(AliasSubcommand::Opprett(OpprettAliasArgs { prosjekt, alias })) => {
            let session = session_handler.open(out, Duration::minutes(1)).await?;
            let http_client = FloqApiClient::from_session(session);

            let project = http_client.get_projects().await?;
            if !project.iter().any(|p| p.id == prosjekt) {
                return Err(anyhow!("Fant intet prosjekt med koden '{prosjekt}'"));
            }

            FloqConfig::add_alias(ProjectAlias {
                project_id: prosjekt,
                alias,
            })
            .await
        }
        Some(AliasSubcommand::Slett(SlettAliasArgs { alias })) => FloqConfig::remove_alias(&alias).await,
    }
}

impl FloqApiClient {
    pub async fn get_projects(&self) -> Result<Vec<Project>> {
        let url = format!("{}/projects?select=id,name,active,customer(id,name)", ENV.api_domain);

        self.client
            .get(url)
            .send()
            .await
            .handle_floq_response()
            .await
            .context("Noe gikk galt under henting av alle prosjekter")?
            .json()
            .await
            .handle_malformed_body()
            .context("Klarte ikke lese responsen fra /projects")
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

impl FloqApiClient {
    pub async fn get_current_timestamped_projects_for_employee(&self, employee_id: u16) -> Result<Vec<Project>> {
        let today = Utc::now().date_naive();

        self.get_timestamped_projects_for_employee(employee_id, today).await
    }

    pub async fn get_timestamped_projects_for_employee(
        &self,
        employee_id: u16,
        date: NaiveDate,
    ) -> Result<Vec<Project>> {
        let lower = date - Duration::weeks(2);
        let upper = date + Duration::days(1) * (6 - date.weekday().num_days_from_monday() as i32); // sunday of the same week as date

        let body = ProjectsForEmployeeRequest {
            employee_id,
            date_range: format!("({}, {})", lower.format("%Y-%m-%d"), upper.format("%Y-%m-%d")),
        }
        .serialize(serde_json::value::Serializer)?
        .to_string();

        let url = format!("{}/rpc/projects_info_for_employee_in_period", ENV.api_domain);

        Ok(self
            .client
            .post(url)
            .body(body)
            .send()
            .await
            .handle_floq_response()
            .await
            .context("Noe gikk galt under henting av dine prosjekter")?
            .json::<Vec<ProjectForEmployeeResponse>>()
            .await
            .handle_malformed_body()
            .context("Klarte ikke lese responsen fra /rpc/projects_info_for_employee_in_period")?
            .into_iter()
            .map(|r| r.into_project())
            .collect())
    }
}
