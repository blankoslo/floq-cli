use std::error::Error;
use std::env;

use async_std::fs;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct UserConfig {
    employee_id: u16,
    email: String,
    name: String,
    refresh_token: String,
}

fn file_path() -> String {
    env::var("HOME").expect("Did not find env var 'HOME'") + "/.floq/user-config.toml"
}

pub async fn load_config() -> Result<Option<UserConfig>, Box<dyn Error>> {
    let r = fs::read_to_string(file_path()).await
        .map(|s| {
            toml::from_str::<UserConfig>(s.as_str())
            .map(|c| Some(c))
            .map_err(|e| e.into())
        })
        .map_err(|e| {
            match e.kind() {
                std::io::ErrorKind::NotFound => Ok(None),
                _ => Err(e)
            }
        });

    match r {
        Ok(ok) => ok,
        Err(e) => e.map_err(|e| e.into())
    }
}

pub async fn update_config(config: &UserConfig) -> Result<(), Box<dyn Error>> {
    let file_content = toml::to_string(config)?;

    fs::write(file_path(), file_content).await
        .map_err(|e| e.into())
}
