mod http_client;
mod print;
mod timetrack;
mod user;

use crate::http_client::projects::Project;
use crate::timetrack::Timetrack;

use std::error::Error;

use chrono::{Duration, Local, NaiveDate};
use clap::{App, AppSettings, Arg, ArgMatches};
use http_client::HTTPClient;

fn main() -> Result<(), Box<dyn Error>> {
    let matches = App::new("timetracker")
        .about("Timetracking in the terminal")
        .version("1.0")
        .author("The Rust Gang")
        .subcommand(App::new("auth").about("Authenticate against Floq"))
        .subcommand(
            App::new("projects")
                .about("Lists name and code of projects")
                .arg(
                    Arg::new("all")
                        .long("all")
                        .short('a')
                        .about("Flag to list all project"),
                ),
        )
        .subcommand(
            App::new("history")
                .about("Display tracked hours history for a time period")
                .arg(
                    Arg::new("from")
                        .long("from")
                        .short('f')
                        .takes_value(true)
                        .requires("to")
                        .about(
                            "First date to display tracked hours for, default is monday this week",
                        ),
                )
                .arg(
                    Arg::new("to")
                        .long("to")
                        .short('t')
                        .takes_value(true)
                        .requires("from")
                        .about(
                            "Last date to display tracked hours for, default is sunday this week",
                        ),
                ),
        )
        .subcommand(
            App::new("timetrack")
                .about("Track worked hours for a project")
                .setting(AppSettings::ArgRequiredElseHelp)
                .arg(Arg::new("project").about("Project to timetrack").index(1))
                .arg(
                    Arg::new("date")
                        .long("date")
                        .short('d')
                        .default_value(&Local::now().date().format("%Y-%m-%d").to_string())
                        .about("Day to timetrack, default value is today"),
                )
                .arg(
                    Arg::new("hours")
                        .long("hours")
                        .short('h')
                        .default_value("7.5")
                        .about("Number of hours to timetrack, default value is 7.5"),
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
        Some(("auth", _)) => {
            let user = user::authorize_user().await?;
            println!("Hi {}!", user.name);
            Ok(())
        }
        Some(("projects", projects_matches)) => {
            let user = user::load_user_from_config().await?;
            let client = HTTPClient::from_user(&user);

            let all = projects_matches.is_present("all");
            let mut projects = if all {
                client.get_projects().await?
            } else {
                client
                    .get_current_timetracked_projects_for_employee()
                    .await?
            };
            projects.sort_by(|p1, p2| p1.id.cmp(&p2.id));

            let mut table_maker = print::TableMaker::new(vec!["ID", "CUSTOMER", "DESCRIPTION"]);
            table_maker
                .with(|p: &Project| p.id.clone())
                .with(|p| p.customer.name.clone())
                .with(|p| p.name.clone());
            table_maker.into_table(&projects).printstd();

            Ok(())
        }
        Some(("history", matchers)) => {
            let user = user::load_user_from_config().await?;
            let client = HTTPClient::from_user(&user);

            let from = matchers
                .value_of("from")
                .map(|from| from.parse::<NaiveDate>());
            let to = matchers
                .value_of("to")
                .map(|from| from.parse::<NaiveDate>());

            let mut timetrackings = if from.is_some() && to.is_some() {
                client
                    .get_timetracking_for_period(from.unwrap()?, to.unwrap()?)
                    .await?
            } else {
                client.get_current_week_timetracking().await?
            };
            timetrackings.sort_by(|t0, t1| t0.date.cmp(&t1.date));

            let mut table_maker = print::TableMaker::new(vec!["DATE", "PROJECT", "TIME"]);
            table_maker
                .with(|tt: &Timetrack| tt.date.format("%Y-%m-%d").to_string())
                .with(|tt| tt.project_id.clone())
                .with(|tt|
                    format!(
                        "{}.{}",
                        tt.time.num_hours(),
                        (tt.time - Duration::hours(tt.time.num_hours())).num_minutes() / 6,
                    )
                );
            table_maker.into_table(&timetrackings).printstd();

            Ok(())
        }
        Some(("timetrack", track_matches)) => {
            let user = user::load_user_from_config().await?;
            let client = HTTPClient::from_user(&user);

            let project_id = track_matches.value_of("project").unwrap();
            let date: NaiveDate = track_matches.value_of("date").unwrap().parse()?;
            let hours: f32 = track_matches.value_of("hours").unwrap().parse()?;
            let time = Duration::minutes((hours * 60.0) as i64);

            client.timetrack(project_id, date, time).await.map(|_| ())
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
