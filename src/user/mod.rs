mod auth;
mod config;
mod http;

use std::error::Error;

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

pub async fn get_or_authorize_user() -> Result<User, Box<dyn Error>> {
    if let Some(config) = config::load_config().await? {
        let authorized_user = auth::refresh_access_token(&config.refresh_token).await?;

        Ok(User {
            employee_id: config.employee_id,
            email: config.email,
            name: config.name,
            access_token: authorized_user.access_token,
        })
    } else {
        let authorized_user = auth::authorize().await?;

        let employee = http::get_logged_in_employee(&authorized_user.access_token).await?;

        let config = config::UserConfig {
            employee_id: employee.id,
            email: employee.email.clone(),
            name: employee.name.clone(),
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
}
