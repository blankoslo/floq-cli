mod http_client;
mod user;

use crate::http_client::projects::Project;

use std::{error::Error, fmt};

use chrono::{Duration, Utc};
use clap::{App, AppSettings, Arg, ArgMatches};
use http_client::HTTPClient;

fn main() -> Result<(), Box<dyn Error>> {
    let matches = App::new("timetracker")
        .about("Timetracking in the terminal")
        .version("1.0")
        .author("The Rust Gang")
        .subcommand(App::new("demo").about("Get a demo of features not used elsewhere"))
        .subcommand(App::new("auth").about("Authenticate again Floq"))
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

    async_std::task::block_on(async {
        match perform_command(matches).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    })
}

async fn perform_command(matches: ArgMatches) -> Result<(), Box<dyn Error>> {
    match matches.subcommand() {
        Some(("demo", _)) => {
            let user = user::load_user_from_config().await?;
            let client = HTTPClient::from_user(&user);

            demo(client).await
        }
        Some(("auth", _)) => {
            let user = user::authorize_user().await?;
            println!("Hi {}!", user.name);
            Ok(())
        },
        Some(("projects", projects_matches)) => {
            let user = user::load_user_from_config().await?;
            let client = HTTPClient::from_user(&user);

            let all = projects_matches.is_present("all");
            if all {
                print_all_projects(client).await
            } else {
                print_relevant_projects(client).await
            }
        }
        Some(("history", _)) => {
            println!("History is not implemented yet...");
            Ok(())
        }
        Some(("track", track_matches)) => {
            let project_code = track_matches.value_of("project").unwrap();
            let hours = track_matches
                .values_of("hours")
                .unwrap()
                .collect::<Vec<_>>()
                .join(", ");

            println!("Test {} {}", project_code, hours);
            Ok(())
        }

        Some((_, _)) => {
            unreachable!("Unknown commands should be handled by the library");
        }
        None => {
            println!("No subcommand was used");
            Ok(())
        } // If all subcommands are defined above, anything else is unreachable!()
    }
}

async fn demo(http_client: HTTPClient) -> Result<(), Box<dyn std::error::Error>> {
    let projects = http_client.get_projects().await?;
    println!("Projects:");
    println!("{:#?}", projects);

    let relevant_projects = http_client
        .get_current_timetracked_projects_for_employee()
        .await?;
    println!();
    println!("Relevant projects:");
    println!("{:#?}", relevant_projects);

    let current_week_timetrackings = http_client.get_current_week_timetracking().await?;
    println!();
    println!("Current week timetracking:");
    println!("{:#?}", current_week_timetrackings);

    http_client
        .timetrack(
            "SVO1000".to_string(),
            Utc::now().date().naive_utc(),
            Duration::hours(7) + Duration::minutes(30),
        )
        .await?;

    println!("Done timetracking!");

    Ok(())
}

async fn print_all_projects(http_client: HTTPClient) -> Result<(), Box<dyn std::error::Error>> {
    let ui_projects = UIProjects(http_client.get_projects().await?);
    println!("{}", ui_projects);
    Ok(())
}

async fn print_relevant_projects(
    http_client: HTTPClient,
) -> Result<(), Box<dyn std::error::Error>> {
    let relevant_projects = UIProjects(
        http_client
            .get_current_timetracked_projects_for_employee()
            .await?,
    );
    println!("{}", relevant_projects);
    Ok(())
}

struct UIProjects(Vec<Project>);

impl fmt::Display for UIProjects {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let headings = ["PROSJEKT KODE", "KUNDE", "BESKRIVELSE"];

        let padding = 7;
        let id_width = headings[0].len() + padding;
        let customer_name_length = self
            .0
            .iter()
            .map(|p| p.customer.name.len())
            .max()
            .unwrap_or(0);
        let customer_width = customer_name_length + padding;

        writeln!(
            f,
            "{:id_width$} {:customer_width$} {}",
            headings[0],
            headings[1],
            headings[2],
            id_width = id_width,
            customer_width = customer_width
        )?;

        for project in self.0.iter() {
            writeln!(
                f,
                "{:id_width$} {:customer_width$} {}",
                project.id,
                project.customer.name,
                project.name,
                id_width = id_width,
                customer_width = customer_width
            )?;
        }
        writeln!(f)
    }
}
