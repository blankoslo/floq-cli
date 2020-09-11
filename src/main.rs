use clap::{App, AppSettings, Arg};
use http_client::timetrack::HTTPClient;
use serde::Deserialize;
use std::env;
use dotenv::dotenv;

#[derive(Deserialize, Debug)]
struct IP {
    origin: String
}
fn get_envvar(key: &str) -> String {
    let envvar = env::var(key);
    if envvar.is_err() {
        panic!("env var {} not defined", key)
    }
    return envvar.unwrap()
}

fn setup() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let bearer_token = get_envvar("BEARER_TOKEN");
    let http_client = HTTPClient::new(bearer_token);
    let projects = http_client.get_current_week_timetracks(77)?;
    println!("{:#?}", projects);
    Ok(())
}

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

            println!("Test {} {}", project_code, hours)
        }

        Some((_, _)) => unreachable!("Unknown commands should be handled by the library"),
        None => println!("No subcommand was used"), // If all subcommands are defined above, anything else is unreachable!()
    }

    // An alternative to checking the name is matching on known names. Again notice that only the
    // direct children are matched here.
    match matches.subcommand_name() {
        Some("clone") => println!("'git clone' was used"),
        Some("push") => println!("'git push' was used"),
        Some("add") => println!("'git add' was used"),
        None => println!("No subcommand was used"),
        _ => unreachable!(), // Assuming you've listed all direct children above, this is unreachable
    }

    // You could get the independent subcommand matches, although this is less common
    if let Some(clone_matches) = matches.subcommand_matches("clone") {
        // Now we have a reference to clone's matches
        println!("Cloning repo: {}", clone_matches.value_of("repo").unwrap());
    }

    // The most common way to handle subcommands is via a combined approach using
    // `ArgMatches::subcommand` which returns a tuple of both the name and matches
    match matches.subcommand() {
        ("projects", Some(projects_matches)) => {
            let all = projects_matches.is_present("all");
            println!("Cloning {}", all);

            if all {
                println!("get_projects is not implemented yet...")
            } else {
                println!("get_all_projects is not implemented yet...")
            }
        }
        ("history", _) => {
            println!("History is not implemented yet...")
        }
        ("track", Some(track_matches)) => {
            // Now we have a reference to add's matches
            println!(
                "Tracking {}",
                track_matches
                    .values_of("stuff")
                    .unwrap()
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        (_, None) => println!("No subcommand was used"), // If no subcommand was used it'll match the tuple ("", None)
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachabe!()
    }
}

fn get_env_var(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("env var {} not defined", key))
}
