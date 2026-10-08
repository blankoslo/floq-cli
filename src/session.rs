use std::io::Write;
use std::time::Duration;
use std::{collections::HashMap, env};

use anyhow::Result;
use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::auth::{AuthResponse, FloqAuth};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloqSession {
    access_token: String,
    access_token_expires: DateTime<Utc>,
    refresh_token: Option<String>,
    scopes: Option<Vec<String>>,
}

impl FloqSession {
    pub fn access_token(&self) -> &str {
        &self.access_token
    }
}

impl From<AuthResponse> for FloqSession {
    fn from(auth_response: AuthResponse) -> Self {
        FloqSession {
            access_token: auth_response.access_token,
            access_token_expires: Utc::now() + auth_response.expires_in.unwrap_or(Duration::from_secs(3600)),
            refresh_token: auth_response.refresh_token,
            scopes: auth_response.scopes,
        }
    }
}

fn home_path() -> String {
    env::var("HOME")
        .or_else(|_| env::var("HOMEPATH"))
        .expect("Did not find env var 'HOME' or 'HOMEPATH'")
}

fn folder_path() -> String {
    home_path() + "/.floq"
}

async fn read_sessions(path: &str) -> Result<HashMap<String, FloqSession>> {
    match tokio::fs::read_to_string(&path).await {
        Ok(f) => toml::from_str(&f).map_err(anyhow::Error::new),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Ok(HashMap::default())
            } else {
                Err(anyhow::Error::new(e))
            }
        }
    }
}

async fn save_sessions(path: &str, sessions: &HashMap<String, FloqSession>) -> Result<()> {
    let contents = toml::to_string(&sessions)?;
    tokio::fs::create_dir_all(folder_path()).await?;
    let mut options = tokio::fs::OpenOptions::new();
    options.create(true).write(true).truncate(true);
    #[cfg(unix)]
    options.mode(0o600);

    let mut file = options.open(&path).await?;
    file.write_all(contents.as_bytes()).await?;

    Ok(())
}

impl FloqSession {
    async fn from_file(path: &str, issuer: &str) -> Result<Option<Self>> {
        let sessions = read_sessions(path).await?;

        Ok(sessions.get(issuer).cloned())
    }

    async fn save_to_file(&self, path: &str, issuer: &str) -> Result<()> {
        let mut sessions = read_sessions(path).await?;

        sessions.insert(issuer.to_string(), self.clone());

        save_sessions(path, &sessions).await?;

        Ok(())
    }

    async fn delete_from_file(path: &str, issuer: &str) -> Result<()> {
        let mut sessions = read_sessions(path).await?;

        sessions.remove(issuer);

        save_sessions(path, &sessions).await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FloqSessionHandler {
    file_path: String,
    auth: FloqAuth,
    session: Option<FloqSession>,
}

impl Default for FloqSessionHandler {
    fn default() -> Self {
        Self::new(folder_path() + "/session.toml", FloqAuth::default()).unwrap()
    }
}

impl FloqSessionHandler {
    pub fn new(file_path: String, auth: FloqAuth) -> Result<Self> {
        Ok(Self {
            file_path,
            auth,
            session: None,
        })
    }

    async fn authenticate<OUT: Write + Send>(&mut self, out: &mut OUT) -> Result<&FloqSession> {
        let auth_response = self.auth.authenticate(out).await?;
        let session: FloqSession = auth_response.into();
        session.save_to_file(&self.file_path, self.auth.issuer()).await?;
        self.session = Some(session);
        Ok(self.session.as_ref().unwrap())
    }

    async fn resume(&mut self, leeway: TimeDelta) -> Result<Option<&FloqSession>> {
        if self.session.is_none() {
            self.session = FloqSession::from_file(&self.file_path, self.auth.issuer()).await?;
        }

        if let Some(session) = &mut self.session {
            if session.access_token_expires > Utc::now() + leeway {
                Ok(Some(session))
            } else {
                // Access token is expired or about to expire, refresh it
                if let Some(refresh_token) = session.refresh_token.take() {
                    *session = self.auth.refresh(&refresh_token).await?.into();
                    // refresh may return a response with no new refresh token, in which case we must retain the old one.
                    if session.refresh_token.is_none() {
                        session.refresh_token = Some(refresh_token);
                    }
                    session.save_to_file(&self.file_path, self.auth.issuer()).await?;
                    Ok(Some(session))
                } else {
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    pub async fn open<OUT: Write + Send>(&mut self, out: &mut OUT, leeway: TimeDelta) -> Result<&FloqSession> {
        match self.resume(leeway).await {
            Ok(Some(_)) => {
                // Instance of NLL problem case #3: we cannot return the session from self.resume without causing an
                // issue with the mutable borrow of self for self.authenticate. As a workaround, we can safely unwrap
                // a fresh session from self.session if resume returns Some.
                return Ok(self.session.as_ref().unwrap());
            }
            Ok(None) => {}
            Err(err) => {
                writeln!(
                    out,
                    "Kunne ikke gjenopprette sesjonen, du må logge inn på nytt: {:?}",
                    err
                )?;
            }
        }
        self.authenticate(out).await
    }

    pub async fn terminate(&mut self) -> Result<bool> {
        if self.session.is_none() {
            self.session = FloqSession::from_file(&self.file_path, self.auth.issuer()).await?;
        }

        let session = self.session.take();
        let terminated = if let Some(session) = session {
            if let Some(refresh_token) = &session.refresh_token {
                self.auth.revoke(refresh_token).await?;
            }

            FloqSession::delete_from_file(&self.file_path, self.auth.issuer()).await?;

            true
        } else {
            false
        };
        Ok(terminated)
    }

    pub async fn reauthenticate<OUT: Write + Send>(&mut self, out: &mut OUT) -> Result<&FloqSession> {
        // First, terminate existing session (if any) to revoke its refresh token
        self.terminate().await?;
        self.authenticate(out).await
    }
}
