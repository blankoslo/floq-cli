use std::io::Write;

use anyhow::Result;
use clap::{ArgMatches, Command};

use crate::{http_client::FloqApiClient, session::FloqSessionHandler};

pub const SUBCOMMAND_NAME: &str = "bruker";

pub fn subcommand_app() -> Command {
    Command::new(SUBCOMMAND_NAME)
        .about("Brukerhåndtering")
        .arg_required_else_help(true)
        .subcommand(Command::new("logg-inn").about("Logg inn i Floq"))
        .subcommand(Command::new("logg-ut").about("Logg ut av Floq (sletter din lokale sesjon)"))
}

pub async fn execute<T: Write + Send>(
    matches: &ArgMatches,
    out: &mut T,
    session_handler: &mut FloqSessionHandler,
) -> Result<()> {
    match matches.subcommand() {
        Some(("logg-inn", _)) => {
            let session = session_handler.reauthenticate(out).await?;
            let client = FloqApiClient::from_session(session);

            let employee = client.get_logged_in_employee().await?;

            writeln!(out, "Hei, {} {}!", employee.first_name, employee.last_name)?;
            writeln!(out)?;
            Ok(())
        }
        Some(("logg-ut", _)) => {
            session_handler.terminate().await?;
            writeln!(out, "Ha det bra!")?;
            writeln!(out)?;
            Ok(())
        }
        _ => unreachable!("Unknown commands should be handled by the library"),
    }
}
