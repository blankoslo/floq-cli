use crate::user::User;

use std::option_env;

use anyhow::{anyhow, Context};
use surf::StatusCode;

pub const FLOQ_DOMAIN: Option<&str> = option_env!("FLOQ_DOMAIN");
pub const FLOQ_API_DOMAIN: Option<&str> = option_env!("FLOQ_API_DOMAIN");

pub fn floq_domain() -> &'static str {
    FLOQ_DOMAIN.unwrap_or("https://blank-test.floq.no")
}

pub fn floq_api_domain() -> &'static str {
    FLOQ_API_DOMAIN.unwrap_or("https://api-blank-test.floq.no")
}

pub struct HttpClient {
    pub access_token: String,
    pub employee_id: u16,
}

impl HttpClient {
    pub fn from_user(user: &User) -> Self {
        Self {
            access_token: user.access_token.clone(),
            employee_id: user.employee_id,
        }
    }
}

pub trait HandleInvalidToken {
    fn handle_floq_response(self) -> Result<surf::Response, anyhow::Error>;
}

impl HandleInvalidToken for surf::Result<surf::Response> {
    fn handle_floq_response(self) -> Result<surf::Response, anyhow::Error> {
        self
            .map_err(|e| e.downcast().unwrap_or_else(|e2| anyhow!(e2)))
            .and_then(|r| match r.status() {
                StatusCode::Unauthorized | StatusCode::Forbidden => Err(anyhow!(
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

pub trait HandleMalformedBody<T> {
    fn handle_malformed_body(self) -> Result<T, anyhow::Error>;
}

impl<T> HandleMalformedBody<T> for surf::Result<T> {
    fn handle_malformed_body(self) -> Result<T, anyhow::Error> {
        self
            .map_err(|e| e.downcast().unwrap_or_else(|e2| anyhow!(e2)))
            .with_context(|| "Klarte ikke å lese svaret fra Floq")
    }
}
