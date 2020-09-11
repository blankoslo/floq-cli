// Working with subcommands is simple. There are a few key points to remember when working with
// subcommands in clap. First, they are really just Apps. This means they can have their own
// settings, version, authors, args, and even their own subcommands. The next thing to remember is
// that subcommands are set up in a tree like hierarchy.
//
// An ASCII art depiction may help explain this better. Using a fictional version of git as the demo
// subject. Imagine the following are all subcommands of git (note, the author is aware these aren't
// actually all subcommands in the real git interface, but it makes explanation easier)
//
//            Top Level App (git)                         TOP
//                           |
//    -----------------------------------------
//   /             |                \          \
// clone          push              add       commit      LEVEL 1
//   |           /    \            /    \       |
//  url      origin   remote    ref    name   message     LEVEL 2
//           /                  /\
//        path            remote  local                   LEVEL 3
//
// Given the above fictional subcommand hierarchy, valid runtime uses would be (not an all inclusive
// list):
//
// $ git clone url
// $ git push origin path
// $ git add ref local
// $ git commit message
//
// Notice only one command per "level" may be used. You could not, for example, do:
//
// $ git clone url push origin path
//
// It's also important to know that subcommands each have their own set of matches and may have args
// with the same name as other subcommands in a different part of the tree hierarchy (i.e. the arg
// names aren't in a flat namespace).
//
// In order to use subcommands in clap, you only need to know which subcommand you're at in your
// tree, and which args are defined on that subcommand.
//
// Let's make a quick program to illustrate. We'll be using the same example as above but for
// brevity sake we won't implement all of the subcommands, only a few.

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
        ("clone", Some(clone_matches)) => {
            // Now we have a reference to clone's matches
            println!("Cloning {}", clone_matches.value_of("repo").unwrap());
        }
        ("push", Some(push_matches)) => {
            // Now we have a reference to push's matches
            match push_matches.subcommand() {
                ("remote", Some(remote_matches)) => {
                    // Now we have a reference to remote's matches
                    println!("Pushing to {}", remote_matches.value_of("repo").unwrap());
                }
                ("local", _) => {
                    println!("'git push local' was used");
                }
                _ => unreachable!(),
            }
        }
        ("add", Some(add_matches)) => {
            // Now we have a reference to add's matches
            println!(
                "Adding {}",
                add_matches
                    .values_of("stuff")
                    .unwrap()
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        (_, None) => println!("No subcommand was used"), // If no subcommand was used it'll match the tuple ("", None)
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachabe!()
    }

    // Continued program logic goes here...
}

fn get_env_var(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("env var {} not defined", key))
}
