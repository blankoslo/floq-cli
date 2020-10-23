mod floqtt_tui;
mod http_client;
use chrono::{Duration, NaiveDate};

use clap::{App, AppSettings, Arg};
use dotenv::dotenv;
use floqtt_tui::FloqTTTUI;
use http_client::HTTPClient;
use std::env;
use dotenv::dotenv;
use clap::{App, AppSettings, Arg};

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
                let client = new_client();
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

fn new_client() -> HTTPClient {
    dotenv().unwrap();
    let bearer_token = get_env_var("BEARER_TOKEN");
    let employee_id = get_env_var("EMPLOYEE_ID").parse().unwrap();

    HTTPClient::new(bearer_token, employee_id)
    let http_client = HTTPClient::new(bearer_token);
    let employee_id = get_env_var("EMPLOYEE_ID").parse()?;
    let time_trackings = http_client
        .get_current_week_timetracking(employee_id)
        .await?;

    let mut tui = FloqTTTUI::new(time_trackings);
    tui.start();
    //   demo(http_client, employee_id).await?;
    Ok(())
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
