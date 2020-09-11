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
                .about("Track worked hours for projects")
                .setting(AppSettings::ArgRequiredElseHelp)
                .arg(
                    Arg::new("hours")
                        .short('h')
                        .long("hours")
                        .about("Number of hours to track")
                        .takes_value(true)
                        .multiple(true),
                ),
        )
        .get_matches();

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
