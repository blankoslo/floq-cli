mod http_client;

use chrono::{Duration, NaiveDate};
use dotenv::dotenv;
use std::{env, fmt};

use crate::http_client::projects::Project;
use clap::{App, AppSettings, Arg};
use http_client::HTTPClient;

fn main() {
    let matches = App::new("timetracker")
        .about("Timetracking in the terminal")
        .version("1.0")
        .author("The Rust Gang")
        .subcommand(App::new("demo").about("Get a demo of features not used elsewhere"))
        .subcommand(
            App::new("projects")
                .about("Lists name and code of projects")
                .arg(
                    Arg::new("all")
                        .about("Flag to list all project")
                        .short('a')
                        .long("all"),
                ),
        )
        .subcommand(App::new("history").about("Get the history for the current week"))
        .subcommand(
            App::new("track")
                .about("Track worked hours for a project")
                .setting(AppSettings::ArgRequiredElseHelp)
                .arg(Arg::new("project").about("the project to track").index(1))
                .arg(
                    Arg::new("hours")
                        .about("the number of hours to track")
                        .index(2)
                        .takes_value(true)
                        .multiple(true),
                ),
        )
        .get_matches();

    let client = new_client();

    match matches.subcommand() {
        Some(("demo", _)) => async_std::task::block_on(demo(client)).expect("Done"),
        Some(("projects", projects_matches)) => {
            let all = projects_matches.is_present("all");

            if all {
                async_std::task::block_on(print_all_projects(client)).expect("Done");
            } else {
                async_std::task::block_on(print_relevant_projects(client)).expect("Done");
            }
        }
        Some(("history", _)) => println!("History is not implemented yet..."),
        Some(("track", track_matches)) => {
            let project_code = track_matches.value_of("project").unwrap();
            let hours = track_matches
                .values_of("hours")
                .unwrap()
                .collect::<Vec<_>>()
                .join(", ");

            println!("Test {} {}", project_code, hours);
        }

        Some((_, _)) => unreachable!("Unknown commands should be handled by the library"),
        None => println!("No subcommand was used"), // If all subcommands are defined above, anything else is unreachable!()
    }
}

fn new_client() -> HTTPClient {
    dotenv().unwrap();
    let bearer_token = get_env_var("BEARER_TOKEN");
    let employee_id = get_env_var("EMPLOYEE_ID").parse().unwrap();

    HTTPClient::new(bearer_token, employee_id)
}

async fn demo(http_client: HTTPClient) -> Result<(), Box<dyn std::error::Error>> {
    let current_week_timetrackings = http_client.get_current_week_timetracking().await?;
    println!();
    println!("Current week timetracking:");
    println!("{:#?}", current_week_timetrackings);

    http_client
        .timetrack(
            "SVO1000".to_string(),
            NaiveDate::from_ymd(2020, 10, 9),
            Duration::hours(7) + Duration::minutes(30),
        )
        .await?;

    println!("Done timetracking!");
    Ok(())
}

async fn print_all_projects(http_client: HTTPClient) -> Result<(), Box<dyn std::error::Error>> {
    let ui_projects = UIProjects {
        projects: http_client.get_projects().await?,
    };
    println!("{}", ui_projects);
    Ok(())
}

async fn print_relevant_projects(
    http_client: HTTPClient,
) -> Result<(), Box<dyn std::error::Error>> {
    let relevant_projects = UIProjects {
        projects: http_client
            .get_current_timetracked_projects_for_employee()
            .await?,
    };
    println!("{}", relevant_projects);
    Ok(())
}

fn get_env_var(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("env var {} not defined", key))
}

pub struct UIProjects {
    projects: Vec<Project>,
}

impl fmt::Display for UIProjects {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let headings = ["PROSJEKT KODE", "KUNDE", "BESKRIVELSE"];

        let padding = 7;
        let id_width = headings[0].len() + padding;
        let mut customer_name_length = 0;

        for project in self.projects.iter() {
            if project.customer.name.len() > customer_name_length {
                customer_name_length = project.customer.name.len()
            }
        }

        let customer_width = customer_name_length + padding;

        let mut result = format!(
            "{:id_width$} {:customer_width$} {}\n",
            headings[0],
            headings[1],
            headings[2],
            id_width = id_width,
            customer_width = customer_width
        )
        .to_owned();

        for project in self.projects.iter() {
            let formatted_project = format!(
                "{:id_width$} {:customer_width$} {}\n",
                project.id,
                project.customer.name,
                project.name,
                id_width = id_width,
                customer_width = customer_width
            )
            .to_owned();
            // todo: handle this better?
            result += &*formatted_project;
        }
        write!(f, "{}", result)
    }
}

impl fmt::Display for Project {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}: {}", self.id, self.customer.id, self.name)
    }
}
