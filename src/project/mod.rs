use crate::config::{FloqConfig, ProjectAlias};
use crate::env::ENV;
use crate::http_client::{FloqApiClient, HandleInvalidToken, HandleMalformedBody};
use crate::print::TableMaker;
use crate::session::FloqSessionHandler;

use std::io::Write;

use chrono::{Datelike, Duration, NaiveDate, Utc};
use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};

use anyhow::{Context, Result, anyhow};

#[derive(Deserialize, Debug)]
pub struct Project {
    pub id: String,
    pub name: String,
    #[allow(unused)]
    active: bool,
    pub customer: Customer,
}

#[derive(Deserialize, Debug)]
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
    /// List opp prosjekter (default)
    #[command(name = "list")]
    List(ProjectArgs),
    /// Opprett prosjektalias
    #[command(name = "alias")]
    Alias(AliasCommand),
}

#[derive(Args, Default)]
pub struct ProjectArgs {
    /// Vis alle prosjekter, i stedet for kun de du har ført timer på de siste to ukene
    #[arg(short, long)]
    alle: bool,
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
    #[command(name = "list")]
    List,
    /// Opprette et nytt prosjektalias
    #[command(name = "opprett")]
    Opprett(OpprettAliasArgs),
    /// Slett et eksisterende prosjektalias
    #[command(name = "slett")]
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

pub async fn execute<T: Write + Send>(
    cmd: ProjectCommand,
    out: &mut T,
    session_handler: &mut FloqSessionHandler,
) -> Result<()> {
    match cmd.command {
        None => execute_list(Default::default(), out, session_handler).await,
        Some(ProjectSubcommand::List(args)) => execute_list(args, out, session_handler).await,
        Some(ProjectSubcommand::Alias(cmd)) => execute_alias(cmd, out, session_handler).await,
    }
}

async fn execute_list<T: Write + Send>(
    args: ProjectArgs,
    out: &mut T,
    session_handler: &mut FloqSessionHandler,
) -> Result<()> {
    let session = session_handler.open(out, Duration::minutes(1)).await?;
    let client = FloqApiClient::from_session(session);

    let mut projects = if args.alle {
        client.get_projects().await?
    } else {
        let employee_id = client.get_logged_in_employee().await?.id;

        client
            .get_current_timestamped_projects_for_employee(employee_id)
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

async fn execute_alias<T: Write + Send>(
    cmd: AliasCommand,
    out: &mut T,
    session_handler: &mut FloqSessionHandler,
) -> Result<()> {
    match cmd.command {
        None | Some(AliasSubcommand::List) => {
            let alias = FloqConfig::from_file().await?.project_alias;

            let mut table_maker = TableMaker::new();
            table_maker.static_titles(vec!["PROSJEKTID", "ALIAS"]);
            table_maker
                .with(Box::new(|p: &ProjectAlias| p.project_id.clone()))
                .with(Box::new(|p| p.alias.clone()));
            table_maker.into_table(&alias).print(out)?;

            Ok(())
        }
        Some(AliasSubcommand::Opprett(OpprettAliasArgs { prosjekt, alias })) => {
            let session = session_handler.open(out, Duration::minutes(1)).await?;
            let http_client = FloqApiClient::from_session(session);

            let project = http_client.get_projects().await?;
            if !project.iter().any(|p| p.id == prosjekt) {
                return Err(anyhow!("Fant ingen project med koden '{prosjekt}'"));
            }
            let mut config = FloqConfig::from_file().await?;
            if let Some(pa) = config.find_project_alias_by_project_id(&prosjekt) {
                return Err(anyhow!(
                    "Prosjektet '{}' har allerede et alias '{}'",
                    pa.project_id,
                    pa.alias,
                ));
            }
            if let Some(pa) = config.find_project_alias_by_alias(&alias) {
                return Err(anyhow!(
                    "Alias '{}' er allerede i bruk for prosjektet '{}'",
                    pa.alias,
                    pa.project_id,
                ));
            }

            config.project_alias.push(ProjectAlias {
                project_id: prosjekt,
                alias,
            });
            config.save_to_file().await?;

            Ok(())
        }
        Some(AliasSubcommand::Slett(SlettAliasArgs { alias })) => {
            let mut config = FloqConfig::from_file().await?;
            if config.find_project_alias_by_alias(&alias).is_none() {
                return Err(anyhow!("Fant ingen konfiguerte alias med navnet '{alias}'"));
            }

            let project_alias = config
                .project_alias
                .into_iter()
                .filter(|pa| pa.alias != alias)
                .collect();
            config.project_alias = project_alias;
            config.save_to_file().await?;

            Ok(())
        }
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
