use std::{env, io::ErrorKind};

use tokio::fs;

use anyhow::{Context, Result};
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

pub async fn load_config() -> Result<Option<UserConfig>> {
    let content = match fs::read_to_string(file_path()).await {
        Ok(s) => s,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(e).with_context(|| format!("Klarte ikke å lese konfigurasjonsfilen {}", file_path()));
        }
    };

    let config = toml::from_str::<UserConfig>(&content)
        .with_context(|| format!("Klarte ikke å tolke konfigurasjonsfilen {}", file_path()))?;

    Ok(Some(config))
}

pub async fn update_config(config: &UserConfig) -> Result<()> {
    let file_content = toml::to_string(config)
        .context("Klarte ikke å bygge inneholdet i konfigigurasjonsfilen, vennligst logg inn på nytt")?;

    match fs::create_dir(folder_path()).await {
        Ok(_) => Ok(()),
        Err(e) => match e.kind() {
            ErrorKind::AlreadyExists => Ok(()),
            _ => Err(e),
        },
    }
    .with_context(|| format!("Klarte ikke å opprette mappen {}", folder_path()))?;

    fs::write(file_path(), file_content)
        .await
        .with_context(|| format!("Klarte ikke å skrive til konfirgurasjonsfilen {}", file_path()))
}

pub async fn delete_config() -> Result<()> {
    match fs::remove_file(file_path()).await {
        Ok(_) => Ok(()),
        Err(e) => match e.kind() {
            ErrorKind::NotFound => Ok(()),
            _ => Err(e),
        },
    }
    .with_context(|| format!("Klarte ikke å slette konfirgurasjonsfilen {}", file_path()))
}
