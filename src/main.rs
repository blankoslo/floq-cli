use cmd::Subcommand;

use std::io;

use anyhow::Result;
use async_std::task;
use clap::{App, AppSettings};

mod cmd;
mod http_client;
mod print;
mod project;
mod time;
mod timestamp;
mod user;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<()> {
    let matches = App::new("floq")
        .about("Floq i din lokale terminal")
        .version(VERSION)
        .author("Rust-gjengen")
        .setting(AppSettings::ArgRequiredElseHelp)
        .subcommand(user::subcommand_app().display_order(1))
        .subcommand(project::subcommand_app().display_order(2))
        .subcommand(timestamp::subcommand_app().display_order(3))
        .subcommand(timestamp::history::subcommand_app().display_order(4))
        .get_matches();

    let commands: [Box<dyn Subcommand<_>>; 4] = [
        user::subcommand(),
        project::subcommand(),
        timestamp::subcommand(),
        timestamp::history::subcommand(),
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
