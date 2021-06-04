use std::error::Error;
use std::{env, io::ErrorKind};

use async_std::fs;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct UserConfig {
    pub employee_id: u16,
    pub email: String,
    pub name: String,
    pub access_token: String,
    pub access_token_expires: NaiveDateTime,
    pub refresh_token: String,
}

fn home_path() -> String {
    env::var("HOME")
        .or_else(|_| env::var("HOMEPATH"))
        .expect("Did not find env var 'HOME' or 'HOMEPATH'")
}

fn folder_path() -> String {
    home_path() + "/.floq"
}

fn file_path() -> String {
    home_path() + "/.floq/user-config.toml"
}

pub async fn load_config() -> Result<Option<UserConfig>, Box<dyn Error>> {
    let r = fs::read_to_string(file_path())
        .await
        .and_then(|s| toml::from_str::<UserConfig>(s.as_str()).map_err(|e| e.into()))
        .map_err(|e| match e.kind() {
            ErrorKind::NotFound => Ok(None),
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
            ErrorKind::AlreadyExists => (),
            _ => return Err(Box::new(e)),
        },
    }

    fs::write(file_path(), file_content)
        .await
        .map_err(|e| e.into())
}

pub async fn delete_config() -> Result<(), Box<dyn Error>> {
    let r = fs::remove_file(file_path())
        .await
        .map_err(|e| match e.kind() {
            ErrorKind::NotFound => Ok(()),
            _ => Err(e),
        });

    match r {
        Ok(_) => Ok(()),
        Err(e) => e.map_err(|e| e.into()),
    }
}
