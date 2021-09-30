use crate::cmd::Subcommand;
use crate::http_client::HttpClient;
use crate::user;

use std::{error::Error, io::Write};

use async_trait::async_trait;
use clap::{App, Arg, ArgMatches};
use cli_table::{CellStruct, Table, WithTitle, format::{Border, Padding, Separator}, print_stdout};
use serde::Deserialize;

mod http;

const SUBCOMMAND_NAME: &str = "prosjekter";

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

        let projects = if matches.is_present("alle") {
            client.get_projects().await?
        } else {
            client
                .get_current_timestamped_projects_for_employee()
                .await?
        };

        let mut rows: Vec<ProjectTableRow> = projects
            .into_iter()
            .map(|p| ProjectTableRow::from(p))
            .collect();
        rows.sort();

        print_stdout(rows.with_title().border(Border::builder().build()).separator(Separator::builder().build()))?;

        Ok(())
    }
}

#[derive(Table, PartialEq, Eq, PartialOrd, Ord)]
struct ProjectTableRow {
    #[table(title = "ID", customize_fn = "style")]
    project_id: String,
    #[table(title = "KUNDE", customize_fn = "style")]
    customer_name: String,
    #[table(title = "BESKRIVELSE")]
    project_name: String,
}

impl From<Project> for ProjectTableRow {
    fn from(project: Project) -> Self {
        ProjectTableRow {
            project_id: project.id,
            customer_name: project.customer.name,
            project_name: project.name,
        }
    }
}

impl ProjectTableRow {
    fn style(cell: CellStruct, _: &String) -> CellStruct {
        cell.padding(Padding::builder().right(100).build())
    }
}
