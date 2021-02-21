use std::env;
use std::error::Error;

use async_std::fs;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct UserConfig {
    pub employee_id: u16,
    pub email: String,
    pub name: String,
    pub refresh_token: String,
}

fn folder_path() -> String {
    env::var("HOME").expect("Did not find env var 'HOME'") + "/.floq"
}

fn file_path() -> String {
    env::var("HOME").expect("Did not find env var 'HOME'") + "/.floq/user-config.toml"
}

pub async fn load_config() -> Result<Option<UserConfig>, Box<dyn Error>> {
    let r = fs::read_to_string(file_path())
        .await
        .and_then(|s| toml::from_str::<UserConfig>(s.as_str()).map_err(|e| e.into()))
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => Ok(None),
            _ => Err(e),
        });

    match r {
        Ok(uc) => Ok(Some(uc)),
        Err(e) => e.map_err(|e| e.into()),
    }
}

pub async fn update_config(config: &UserConfig) -> Result<(), Box<dyn Error>> {
    let file_content = toml::to_string(config)?;

    match fs::create_dir(folder_path()).await {
        Ok(()) => (),
        Err(e) => match e.kind() {
            std::io::ErrorKind::AlreadyExists => (),
            e => panic!(e),
        },
    }

    fs::write(file_path(), file_content)
        .await
        .map_err(|e| e.into())
}
