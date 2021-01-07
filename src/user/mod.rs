pub mod auth;
pub mod config;

use std::error::Error;

/*
pub async fn get_or_authorize_user() -> Result<config::UserConfig, Box<dyn Error>> {
    if let Some(c) = config::load_config().await? {
        Ok(c)
    } else {
        let tokens = auth::authorize().await?;

        
    }
}
*/
