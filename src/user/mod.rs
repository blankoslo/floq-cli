use crate::cmd::Subcommand;

use std::{error::Error, io::Write};

use async_trait::async_trait;
use chrono::{Duration, Utc};
use clap::{App, AppSettings, ArgMatches};

mod auth;
mod config;
mod http;

const SUBCOMMAND_NAME: &str = "bruker";

pub fn subcommand_app<'help>() -> App<'help> {
    App::new(SUBCOMMAND_NAME)
        .about("Brukerhåndtering")
        .setting(AppSettings::ArgRequiredElseHelp)
        .subcommand(App::new("logg-inn").about("Logg inn i Floq"))
        .subcommand(
            App::new("logg-ut").about("Logg ut av Floq (sletter din lokale brukerkonfigurasjon)"),
        )
}

pub fn subcommand<T: Write + Send>() -> Box<dyn Subcommand<T>> {
    Box::new(UserSubcommand {})
}

struct UserSubcommand;

#[async_trait(?Send)]
impl<T: Write + Send> Subcommand<T> for UserSubcommand {
    fn matches(&self, matches: &ArgMatches) -> bool {
        matches.subcommand_name() == Some(SUBCOMMAND_NAME)
    }

    async fn execute(&self, matches: &ArgMatches, out: &mut T) -> Result<(), Box<dyn Error>> {
        match matches.subcommand() {
            Some(("logg-inn", _)) => {
                authorize_user(out).await?;
                Ok(())
            }
            Some(("logg-ut", _)) => {
                config::delete_config().await?;
                writeln!(out, "Ha det bra!")?;
                Ok(())
            }
            _ => unreachable!("Unknown commands should be handled by the library"),
        }
    }
}

pub struct User {
    pub employee_id: u16,
    pub email: String,
    pub name: String,
    pub access_token: String,
}
pub struct Employee {
    id: u16,
    email: String,
    name: String,
}

pub async fn authorize_user<OUT: Write + Send>(out: &mut OUT) -> Result<User, Box<dyn Error>> {
    let authorized_user = auth::authorize(out).await?;

    let employee = http::get_logged_in_employee(&authorized_user.access_token).await?;

    let config = config::UserConfig {
        employee_id: employee.id,
        email: employee.email.clone(),
        name: employee.name.clone(),
        access_token: authorized_user.access_token.clone(),
        access_token_expires: authorized_user.expires_at,
        refresh_token: authorized_user.refresh_token,
    };
    config::update_config(&config).await?;

    writeln!(out, "Hei, {}!", employee.name)?;
    writeln!(out)?;

    Ok(User {
        employee_id: employee.id,
        email: employee.email,
        name: employee.name,
        access_token: authorized_user.access_token,
    })
}

pub async fn load_user_from_config<OUT: Write + Send>(
    out: &mut OUT,
) -> Result<User, Box<dyn Error>> {
    let config = config::load_config().await?;
    let now = Utc::now().naive_utc();

    match config {
        None => {
            writeln!(
                out,
                "Fant ingen konfigurasjon så starter løpet for autentisering nå:"
            )?;
            authorize_user(out).await
        }
        Some(mut c) if c.access_token_expires < now - Duration::minutes(1) => {
            let authorized_user = auth::refresh_access_token(&c.refresh_token).await?;

            c.access_token = authorized_user.access_token;
            c.access_token_expires = authorized_user.expires_at;

            config::update_config(&c).await?;

            Ok(User {
                employee_id: c.employee_id,
                email: c.email,
                name: c.name,
                access_token: c.access_token,
            })
        }
        Some(c) => Ok(User {
            employee_id: c.employee_id,
            email: c.email,
            name: c.name,
            access_token: c.access_token,
        }),
    }
}
