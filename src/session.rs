use std::io::Write;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};

use crate::auth::{AuthResponse, FloqAuth};
use crate::config::{config_dir, modify_file, read_file};

static SESSION_FILE: LazyLock<PathBuf> = LazyLock::new(|| {
    let mut file = config_dir();
    file.push("session.toml");
    file
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloqSession {
    access_token: String,
    access_token_expires: DateTime<Utc>,
    refresh_token: Option<String>,
    id_token: Option<String>,
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
            id_token: auth_response.id_token,
            scopes: auth_response.scopes,
        }
    }
}

impl FloqSession {
    async fn from_file(issuer: &str) -> Result<Option<Self>> {
        let mut sessions = read_file(SESSION_FILE.as_path()).await?;

        Ok(sessions.remove(issuer).unwrap_or_default())
    }

    async fn save_to_file(&self, issuer: &str) -> Result<()> {
        let session = self.clone();
        let issuer = issuer.to_string();
        modify_file::<FloqSession>(SESSION_FILE.as_path(), move |sessions| {
            sessions.insert(issuer, session);
        })
        .await
        .context("Kunne ikke lagre oppdatert sesjonsfil")
    }

    async fn delete_from_file(issuer: &str) -> Result<()> {
        let issuer = issuer.to_string();
        modify_file::<FloqSession>(SESSION_FILE.as_path(), move |sessions| {
            sessions.remove(&issuer);
        })
        .await
        .context("Kunne ikke lagre sesjonsfil etter sletting av sesjon")
    }
}

#[derive(Debug, Clone)]
pub struct FloqSessionHandler {
    auth: FloqAuth,
    session: Option<FloqSession>,
}

impl Default for FloqSessionHandler {
    fn default() -> Self {
        Self::new(FloqAuth::default()).unwrap()
    }
}

impl FloqSessionHandler {
    pub fn new(auth: FloqAuth) -> Result<Self> {
        Ok(Self { auth, session: None })
    }

    async fn authenticate<OUT: Write + Send>(&mut self, out: &mut OUT) -> Result<&FloqSession> {
        let auth_response = self.auth.authenticate(out).await?;
        let session: FloqSession = auth_response.into();
        session.save_to_file(self.auth.issuer()).await?;
        self.session = Some(session);
        Ok(self.session.as_ref().unwrap())
    }

    async fn resume(&mut self, leeway: TimeDelta) -> Result<Option<&FloqSession>> {
        if self.session.is_none() {
            self.session = FloqSession::from_file(self.auth.issuer()).await?;
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
                    session.save_to_file(self.auth.issuer()).await?;
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
            self.session = FloqSession::from_file(self.auth.issuer()).await?;
        }

        let terminated = if let Some(session) = self.session.take() {
            if let Some(refresh_token) = &session.refresh_token {
                self.auth.revoke(refresh_token).await?;
            }

            FloqSession::delete_from_file(self.auth.issuer()).await?;

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
