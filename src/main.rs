mod http_client;

use chrono::{Duration, NaiveDate};
use dotenv::dotenv;
use std::env;

use clap::{App, AppSettings, Arg};
use http_client::HTTPClient;

fn main() {
    let matches = App::new("timetracker")
        .about("Timetracking in the terminal")
        .version("1.0")
        .author("The Rust Gang")
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

    match matches.subcommand() {
        Some(("projects", projects_matches)) => {
            let all = projects_matches.is_present("all");

            if all {
                let client = get_client();
                async_std::task::block_on(demo(client)).expect("Done");
            } else {
                println!("get_projects is not implemented yet...");
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

fn get_client() -> HTTPClient {
    dotenv().unwrap();
    let bearer_token = get_env_var("BEARER_TOKEN");
    let employee_id = get_env_var("EMPLOYEE_ID").parse().unwrap();

    HTTPClient::new(bearer_token, employee_id)
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
            NaiveDate::from_ymd(2020, 10, 9),
            Duration::hours(7) + Duration::minutes(30),
        )
        .await?;

    println!("Done timetracking!");
    Ok(())
}

fn get_env_var(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("env var {} not defined", key))
}
