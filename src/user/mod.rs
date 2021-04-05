use crate::cmd::Subcommand;

use std::{error::Error, io::Write};

use async_trait::async_trait;
use chrono::{Duration, Utc};
use clap::{App, ArgMatches};

mod auth;
mod config;
mod http;

const SUBCOMMAND_NAME: &str = "logg-inn";

pub fn subcommand_app<'help>() -> App<'help> {
    App::new(SUBCOMMAND_NAME).about("Logg inn i Floq")
}

pub fn subcommand<T: Write + Send>() -> Box<dyn Subcommand<T>> {
    Box::new(LoginSubcommand {})
}

struct LoginSubcommand;

#[async_trait(?Send)]
impl<T: Write + Send> Subcommand<T> for LoginSubcommand {
    fn matches(&self, matches: &ArgMatches) -> bool {
        matches.subcommand_name() == Some(SUBCOMMAND_NAME)
    }

    async fn execute(&self, _matches: &ArgMatches, out: &mut T) -> Result<(), Box<dyn Error>> {
        let user = authorize_user().await?;
        write!(out, "Hei {}!", user.name)?;
        Ok(())
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

pub async fn authorize_user() -> Result<User, Box<dyn Error>> {
    let authorized_user = auth::authorize().await?;

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

    Ok(User {
        employee_id: employee.id,
        email: employee.email,
        name: employee.name,
        access_token: authorized_user.access_token,
    })
}

pub async fn load_user_from_config() -> Result<User, Box<dyn Error>> {
    let mut config = config::load_config()
        .await?
        .ok_or("No user configuration found")?;
    let now = Utc::now().naive_utc();

    if config.access_token_expires < now - Duration::minutes(1) {
        let authorized_user = auth::refresh_access_token(&config.refresh_token).await?;

        config.access_token = authorized_user.access_token;
        config.access_token_expires = authorized_user.expires_at;

        config::update_config(&config).await?;

        Ok(User {
            employee_id: config.employee_id,
            email: config.email,
            name: config.name,
            access_token: config.access_token,
        })
    } else {
        Ok(User {
            employee_id: config.employee_id,
            email: config.email,
            name: config.name,
            access_token: config.access_token,
        })
    }
}
