use std::io;

use anyhow::Result;
use clap::Command;

use crate::session::FloqSessionHandler;

mod auth;
mod env;
mod http_client;
mod print;
mod project;
mod session;
mod time;
mod timestamp;
mod user;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let matches = Command::new("floq")
        .about("Floq i din lokale terminal")
        .version(VERSION)
        .author("Rust-gjengen")
        .arg_required_else_help(true)
        .subcommand(user::subcommand_app().display_order(1))
        .subcommand(project::subcommand_app().display_order(2))
        .subcommand(timestamp::subcommand_app().display_order(3))
        .subcommand(timestamp::history::subcommand_app().display_order(4))
        .get_matches();

    let mut session_handler = FloqSessionHandler::default();

    match matches.subcommand() {
        Some((project::SUBCOMMAND_NAME, sub_matches)) => {
            project::execute(sub_matches, &mut io::stdout(), &mut session_handler).await
        }
        Some((user::SUBCOMMAND_NAME, sub_matches)) => {
            user::execute(sub_matches, &mut io::stdout(), &mut session_handler).await
        }
        Some((timestamp::SUBCOMMAND_NAME, sub_matches)) => {
            timestamp::execute(sub_matches, &mut io::stdout(), &mut session_handler).await
        }
        Some((timestamp::history::SUBCOMMAND_NAME, sub_matches)) => {
            timestamp::history::execute(sub_matches, &mut io::stdout(), &mut session_handler).await
        }
        _ => unreachable!("Unknown commands should be handled by the library"),
    }
}
