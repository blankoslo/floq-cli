use crate::user::User;

use anyhow::{anyhow, Context};
use reqwest::{Response, StatusCode};
use serde::de::DeserializeOwned;

pub const FLOQ_DOMAIN: &str = env!("FLOQ_DOMAIN");
pub const FLOQ_API_DOMAIN: &str = env!("FLOQ_API_DOMAIN");

pub struct HttpClient {
    pub client: reqwest::Client,
    pub access_token: String,
    pub employee_id: u16,
}

impl HttpClient {
    pub fn from_user(user: &User) -> Self {
        Self {
            client: reqwest::Client::new(),
            access_token: user.access_token.clone(),
            employee_id: user.employee_id,
        }
    }
}

pub trait HandleInvalidToken {
    fn handle_floq_response(self) -> Result<Response, anyhow::Error>;
}

impl HandleInvalidToken for Result<Response, reqwest::Error> {
    fn handle_floq_response(self) -> Result<Response, anyhow::Error> {
        self.map_err(|e| anyhow!(e)).and_then(|r| match r.status() {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(anyhow!(
                "Ikke adgang til Floq API-et, venligst logg inn på nytt. Statuskode {}",
                r.status()
            )),
            s if s.is_client_error() || s.is_server_error() => Err(anyhow!(
                "Fikk en feilresponse fra Floq med statuskode {}",
                s
            )),
            _ => Ok(r),
        })
    }
}

pub trait ParseBody<T> {
    async fn parse_body(self) -> Result<T, anyhow::Error>;
}

impl<T: DeserializeOwned> ParseBody<T> for Response {
    async fn parse_body(self) -> Result<T, anyhow::Error> {
        let path = self.url().path().to_owned();
        let body = self
            .bytes()
            .await
            .with_context(|| format!("Klarte ikke å lese svaret fra '{path}'"))?;
        let body = serde_json::from_slice(body.as_ref())?;

        Ok(body)
    }
}
