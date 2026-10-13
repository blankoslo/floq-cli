use std::io;

use anyhow::Result;
use clap::{Parser, Subcommand};

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

/// Floq i din lokale terminal
#[derive(Parser)]
#[command(name = "floq", version, author = "Rust-gjengen", arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Brukerhåndtering
    #[command(name = "bruker", subcommand)]
    User(user::UserCommand),
    /// Vis prosjekter
    #[command(name = "prosjekter")]
    Projects(project::ProjectArgs),
    /// Før timer på et prosjekt
    #[command(name = "timeføring")]
    Timestamp(timestamp::TimestampArgs),
    /// Vis timeføring
    #[command(name = "timehistorikk")]
    History(timestamp::history::HistoryArgs),
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut session_handler = FloqSessionHandler::default();

    match cli.command {
        Commands::User(command) => user::execute(command, &mut io::stdout(), &mut session_handler).await,
        Commands::Projects(args) => project::execute(args, &mut io::stdout(), &mut session_handler).await,
        Commands::Timestamp(args) => timestamp::execute(args, &mut io::stdout(), &mut session_handler).await,
        Commands::History(args) => timestamp::history::execute(args, &mut io::stdout(), &mut session_handler).await,
    }
}
