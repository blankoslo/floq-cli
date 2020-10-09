mod http_client;

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
                        .long("all")
                ),
        )
        .subcommand(
            App::new("history")
                .about("Get the history for the current week")
        )
        .subcommand(
            App::new("track")
                .about("Track worked hours for a project")
                .setting(AppSettings::ArgRequiredElseHelp)
                .arg(Arg::new("project").about("the config file to use").index(1))
                .arg(Arg::new("hours").about("the asd file to use").index(2).takes_value(true).multiple(true)))
        .get_matches();

    match matches.subcommand() {
        Some(("projects", projects_matches)) => {
            let all = projects_matches.is_present("all");
            println!("Cloning {}", all);

            if all {
                println!("get_all_projects is not implemented yet...")
            } else {
                println!("get_projects is not implemented yet...")
            }
        }
        Some(("history", _)) => {
            println!("History is not implemented yet...")
        }
        Some(("track", track_matches)) => {
            let project_code = track_matches.value_of("project").unwrap();
            let hours = track_matches.values_of("hours").unwrap().collect::<Vec<_>>().join(", ");

            println!("Test {} {}", project_code, hours);
        }

        Some((_, _)) => unreachable!("Unknown commands should be handled by the library"),
        None => println!("No subcommand was used"), // If all subcommands are defined above, anything else is unreachable!()
    }
}



fn get_env_var(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("env var {} not defined", key))
}

async fn setup() -> Result<(), Box<dyn std::error::Error>> {
    dotenv()?;
    let bearer_token = get_env_var("BEARER_TOKEN");
    let http_client = HTTPClient::new(bearer_token);
    let projects = http_client.get_current_week_timetracking(77).await?;
    println!("{:#?}", projects);
    Ok(())
}