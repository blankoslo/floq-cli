use cmd::Subcommand;

use std::error::Error;
use std::io;

use async_std::task;
use clap::{App, AppSettings};

mod cmd;
mod http_client;
mod print;
mod project;
mod timestamp;
mod user;

fn main() -> Result<(), Box<dyn Error>> {
    let matches = App::new("floq")
        .about("Floq i din lokale terminal")
        .version("0.1")
        .author("Rust-gjengen")
        .setting(AppSettings::ArgRequiredElseHelp)
        .subcommand(user::subcommand_app())
        .subcommand(project::subcommand_app())
        .subcommand(timestamp::subcommand_app())
        .get_matches();

    let commands: Vec<Box<dyn Subcommand<_>>> = vec![
        user::subcommand(),
        project::subcommand(),
        timestamp::subcommand(),
    ];

    match matches.subcommand() {
        Some((_, sub_matches)) => {
            let command = commands.into_iter().find(|sc| sc.matches(&matches));

            match command {
                Some(sc) => {
                    task::block_on(async { sc.execute(sub_matches, &mut io::stdout()).await })
                }
                None => unreachable!("Unknown commands should be handled by the library"),
            }
        }
        None => unreachable!("Unknown commands should be handled by the library"),
    }
}
