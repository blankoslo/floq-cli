#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    collections::HashMap,
    env,
    error::Error,
    fmt::Display,
    io::{self, Read, Seek, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::{sync::OnceCell, task};

use crate::env::ENV;

static CONFIG_FILE: OnceCell<PathBuf> = OnceCell::const_new();

async fn get_config_file_path() -> &'static Path {
    CONFIG_FILE
        .get_or_init(async || {
            let mut file = config_dir();
            file.push("config.toml");
            file
        })
        .await
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FloqConfig {
    pub project_alias: Vec<ProjectAlias>,
}

impl FloqConfig {
    pub async fn from_file() -> Result<Self> {
        let config_file_path = get_config_file_path().await;

        match read_file(config_file_path).await {
            Ok(mut configs) => Ok(configs.remove(ENV.issuer).unwrap_or_default()),
            Err(ReadFileError::Deseralization(_)) => match tokio::fs::remove_file(config_file_path).await {
                Ok(_) => Ok(Default::default()),
                Err(e) => Err(e).context(format!(
                    "Kunne ikke slette ugyldig konfigurasjonsfil, venligst gjør dette manuelt. {}",
                    config_file_path.display(),
                )),
            },
            Err(e) => Err(e).context("Kunne ikke lese konfigurasjonsfil"),
        }
    }

    pub async fn save_to_file(&self) -> Result<()> {
        let config = self.clone();
        modify_file(get_config_file_path().await, move |configs| {
            configs.insert(ENV.issuer.to_string(), config);
        })
        .await
        .context("Kunne ikke lagre oppdatert konfigurasjonsfil")
    }

    pub fn find_project_alias_by_alias(&self, alias: impl AsRef<str>) -> Option<&ProjectAlias> {
        self.project_alias.iter().find(|pa| pa.alias == alias.as_ref())
    }

    pub fn find_project_alias_by_project_id(&self, project_id: impl AsRef<str>) -> Option<&ProjectAlias> {
        self.project_alias
            .iter()
            .find(|pa| pa.project_id == project_id.as_ref())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAlias {
    pub project_id: String,
    pub alias: String,
}

pub fn config_dir() -> PathBuf {
    let mut home = env::home_dir().expect("Did not find env var 'HOME' or 'HOMEPATH'");
    home.push(".floq");
    home
}

#[derive(Debug)]
pub enum ReadFileError {
    Io(io::Error),
    Deseralization(toml::de::Error),
    Join(task::JoinError),
}

impl Display for ReadFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadFileError::Io(error) => write!(f, "IO: {error}"),
            ReadFileError::Deseralization(error) => write!(f, "Deseralization: {error}"),
            ReadFileError::Join(error) => write!(f, "Join: {error}"),
        }
    }
}

impl Error for ReadFileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ReadFileError::Io(error) => Some(error),
            ReadFileError::Deseralization(error) => Some(error),
            ReadFileError::Join(error) => Some(error),
        }
    }
}

impl From<io::Error> for ReadFileError {
    fn from(value: io::Error) -> Self {
        ReadFileError::Io(value)
    }
}

impl From<toml::de::Error> for ReadFileError {
    fn from(value: toml::de::Error) -> Self {
        ReadFileError::Deseralization(value)
    }
}

impl From<task::JoinError> for ReadFileError {
    fn from(value: task::JoinError) -> Self {
        ReadFileError::Join(value)
    }
}

pub async fn read_file<T: DeserializeOwned + Send + 'static>(
    path: &Path,
) -> std::result::Result<HashMap<String, T>, ReadFileError> {
    // tokio doesn't support file locking, so we use spawn_blocking + std::fs for this operation.
    // See: https://github.com/tokio-rs/tokio/issues/7523
    let path = path.to_path_buf();
    task::spawn_blocking(move || -> std::result::Result<HashMap<String, T>, ReadFileError> {
        match std::fs::File::open(&path) {
            Ok(mut file) => {
                file.lock_shared()?;
                let mut contents = String::new();
                file.read_to_string(&mut contents)?;
                let sessions: HashMap<String, T> = toml::from_str(&contents)?;
                Ok(sessions)
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Ok(HashMap::default())
                } else {
                    Err(e)?
                }
            }
        }
    })
    .await?
}

#[derive(Debug)]
pub enum ModifyFileError {
    Io(io::Error),
    Deseralization(toml::de::Error),
    Seralization(toml::ser::Error),
    Join(task::JoinError),
}

impl Display for ModifyFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModifyFileError::Io(error) => write!(f, "IO: {error}"),
            ModifyFileError::Deseralization(error) => write!(f, "Deseralization: {error}"),
            ModifyFileError::Seralization(error) => write!(f, "Seralization: {error}"),
            ModifyFileError::Join(error) => write!(f, "Join: {error}"),
        }
    }
}

impl Error for ModifyFileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ModifyFileError::Io(error) => Some(error),
            ModifyFileError::Deseralization(error) => Some(error),
            ModifyFileError::Seralization(error) => Some(error),
            ModifyFileError::Join(error) => Some(error),
        }
    }
}

impl From<io::Error> for ModifyFileError {
    fn from(value: io::Error) -> Self {
        ModifyFileError::Io(value)
    }
}

impl From<toml::de::Error> for ModifyFileError {
    fn from(value: toml::de::Error) -> Self {
        ModifyFileError::Deseralization(value)
    }
}

impl From<toml::ser::Error> for ModifyFileError {
    fn from(value: toml::ser::Error) -> Self {
        ModifyFileError::Seralization(value)
    }
}

impl From<task::JoinError> for ModifyFileError {
    fn from(value: task::JoinError) -> Self {
        ModifyFileError::Join(value)
    }
}

pub async fn modify_file<T: Serialize + DeserializeOwned + Send>(
    path: &Path,
    f: impl FnOnce(&mut HashMap<String, T>) + Send + 'static,
) -> std::result::Result<(), ModifyFileError> {
    // tokio doesn't support file locking, so we use spawn_blocking + std::fs for this operation.
    // See: https://github.com/tokio-rs/tokio/issues/7523
    let path = path.to_path_buf();
    task::spawn_blocking(move || -> std::result::Result<(), ModifyFileError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut options = std::fs::OpenOptions::new();
        options.create(true).read(true).write(true);
        #[cfg(unix)]
        options.mode(0o600);

        let mut file = options.open(path)?;
        file.lock()?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let mut sessions: HashMap<String, T> = toml::from_str(&contents)?;
        f(&mut sessions);
        let contents = toml::to_string(&sessions)?;
        file.set_len(0)?;
        file.seek(std::io::SeekFrom::Start(0))?;
        file.write_all(contents.as_bytes())?;

        Ok(())
    })
    .await?
}
