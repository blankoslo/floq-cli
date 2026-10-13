use std::io::Write;

use anyhow::Result;
use clap::Subcommand;

use crate::{http_client::FloqApiClient, session::FloqSessionHandler};

#[derive(Subcommand)]
pub enum UserCommand {
    /// Logg inn i Floq
    #[command(name = "logg-inn")]
    Login,
    /// Logg ut av Floq (sletter din lokale sesjon)
    #[command(name = "logg-ut")]
    Logout,
}

pub async fn execute<T: Write + Send>(
    command: UserCommand,
    out: &mut T,
    session_handler: &mut FloqSessionHandler,
) -> Result<()> {
    match command {
        UserCommand::Login => {
            let session = session_handler.reauthenticate(out).await?;
            let client = FloqApiClient::from_session(session);

            let employee = client.get_logged_in_employee().await?;

            writeln!(out, "Hei, {} {}!", employee.first_name, employee.last_name)?;
            writeln!(out)?;
            Ok(())
        }
        UserCommand::Logout => {
            session_handler.terminate().await?;
            writeln!(out, "Ha det bra!")?;
            writeln!(out)?;
            Ok(())
        }
    }
}
