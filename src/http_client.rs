use crate::session::FloqSession;

use std::option_env;

use anyhow::{Context, Result, anyhow};
use reqwest::{Client, StatusCode, header};
use serde::Deserialize;

const FLOQ_API_DOMAIN: Option<&str> = option_env!("FLOQ_API_DOMAIN");

pub fn floq_api_domain() -> &'static str {
    FLOQ_API_DOMAIN.unwrap_or("https://api-test.floq.no")
}

#[derive(Debug, Clone)]
pub struct FloqApiClient {
    pub client: reqwest::Client,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Employee {
    pub id: u16,
    #[allow(unused)]
    pub email: String,
    pub first_name: String,
    pub last_name: String,
}

impl FloqApiClient {
    pub fn from_session(session: &FloqSession) -> Self {
        let mut headers = header::HeaderMap::new();
        let mut auth_value = header::HeaderValue::from_str(&format!("Bearer {}", session.access_token()))
            .expect("Ugyldig access token, vennligst logg inn på nytt");
        auth_value.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth_value);
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(header::ACCEPT, header::HeaderValue::from_static("application/json"));

        Self {
            client: Client::builder()
                .default_headers(headers)
                .build()
                .expect("Klarte ikke å opprette HTTP klienten"),
        }
    }

    pub async fn get_logged_in_employee(&self) -> Result<Employee> {
        self.client
            .post(format!("{}/rpc/who_am_i", floq_api_domain()))
            .send()
            .await
            .handle_floq_response()
            .await
            .context("Noe gikk galt under henting av informasjon om deg")?
            .json::<Employee>()
            .await
            .handle_malformed_body()
            .context("Klarte ikke å lese responsen fra /rpc/who_am_i")
    }
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
