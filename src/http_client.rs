use crate::user::User;

use std::option_env;

use anyhow::{Context, anyhow};
use reqwest::{Client, StatusCode, header};

const FLOQ_DOMAIN: Option<&str> = option_env!("FLOQ_DOMAIN");
const FLOQ_API_DOMAIN: Option<&str> = option_env!("FLOQ_API_DOMAIN");

pub fn floq_domain() -> &'static str {
    FLOQ_DOMAIN.unwrap_or("https://test.floq.no")
}

pub fn floq_api_domain() -> &'static str {
    FLOQ_API_DOMAIN.unwrap_or("https://api-test.floq.no")
}

#[derive(Debug, Clone)]
pub struct UnauthorizedHttpClient {
    pub client: reqwest::Client,
}

impl UnauthorizedHttpClient {
    pub fn new() -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json"),
        );

        Self {
            client: Client::builder()
                .default_headers(headers)
                .build()
                .expect("Klarte ikke å opprette HTTP klienten"),
        }
    }

    pub fn into_authorized_with_user(self, user: &User) -> AuthorizedHttpClient {
        let mut headers = header::HeaderMap::new();
        let mut auth_value =
            header::HeaderValue::from_str(&format!("Bearer {}", user.access_token))
                .expect("Ugyldig access token, vennligst logg inn på nytt");
        auth_value.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth_value);
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json"),
        );

        AuthorizedHttpClient {
            client: Client::builder()
                .default_headers(headers)
                .build()
                .expect("Klarte ikke å opprette HTTP klienten"),
            employee_id: user.employee_id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthorizedHttpClient {
    pub client: reqwest::Client,
    pub employee_id: u16,
}

pub trait HandleInvalidToken {
    async fn handle_floq_response(self) -> Result<reqwest::Response, anyhow::Error>;
}

impl HandleInvalidToken for reqwest::Result<reqwest::Response> {
    async fn handle_floq_response(self) -> Result<reqwest::Response, anyhow::Error> {
        match self {
            Ok(response) => match response.status() {
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(anyhow!(
                    "Ikke adgang til Floq API-et, venligst logg inn på nytt. Statuskode {}",
                    response.status()
                )),
                s if s.is_client_error() || s.is_server_error() => Err(anyhow!(
                    "Fikk en feilrespons fra Floq API med statuskode {}: {}",
                    s,
                    response
                        .text()
                        .await
                        .unwrap_or_else(|_| "<klarte ikke å lese resons body som tekst>".into())
                )),
                _ => Ok(response),
            },
            Err(e) => Err(e).context("Klarte ikke å tolke reponsen fra Floq API"),
        }
    }
}

pub trait HandleMalformedBody<T> {
    fn handle_malformed_body(self) -> Result<T, anyhow::Error>;
}

impl<T> HandleMalformedBody<T> for reqwest::Result<T> {
    fn handle_malformed_body(self) -> Result<T, anyhow::Error> {
        self.context("Klarte ikke å lese svaret fra Floq API")
    }
}
