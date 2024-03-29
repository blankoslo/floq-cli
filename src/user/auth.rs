use crate::http_client::{floq_domain, HandleInvalidToken, HandleMalformedBody};

use std::io::Write;
use std::time::Duration;
use std::{collections::HashMap, sync::mpsc};

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, FixedOffset, NaiveDateTime};
use rouille::{Request, Response};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct AuthorizedUser {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: NaiveDateTime,
}

pub async fn authorize<OUT: Write + Send>(out: &mut OUT) -> Result<AuthorizedUser> {
    let (tx, rx) = mpsc::sync_channel::<Result<AuthorizedUser>>(0);

    let server = rouille::Server::new("0.0.0.0:0", move |request| {
        match handle_callback(request) {
            Ok(tokens) => {
                tx.send(Ok(tokens)).unwrap();
                Response::text("Flott, da er du logget inn i floq cli!\n\n(Bare å lukke denne fanen)")
            },
            Err(e) => {
                eprintln!("Error on handle callback from Floq Auth: {}", e);
                tx.send(Err(e)).unwrap();
                Response::text("An error occurred while trying to handle Auth callback, see command output for more details")
            }
        }
    })
    .map_err(|e| anyhow!("{}", e))?;

    let port = server.server_addr().port();

    writeln!(out)?;
    writeln!(out, "Vennligst åpne denne lenken i nettleseren din:")?;
    writeln!(
        out,
        "{}/login/oauth?to=http://localhost:{}",
        floq_domain(),
        port
    )?;
    writeln!(out)?;

    loop {
        match rx.try_iter().next() {
            Some(Ok(tokens)) => break Ok(tokens),
            Some(Err(e)) => break Err(e),
            None => {
                std::thread::sleep(Duration::from_millis(250));
                server.poll();
            }
        }
    }
}

fn handle_callback(request: &Request) -> Result<AuthorizedUser> {
    let mut params =
        match serde_urlencoded::from_str::<HashMap<String, String>>(request.raw_query_string()) {
            Ok(p) => p,
            Err(e) => {
                return Err(anyhow!("Unable to parse callback request URL"))
                    .with_context(|| format!("Deserialization of query params failed: {:?}", e));
            }
        };

    let access_token = match params.remove("access_token") {
        Some(at) => at,
        None => {
            return Err(anyhow!(
                "Required param 'access_token' is missing from callback"
            ))
        }
    };

    let refresh_token = match params.remove("refresh_token") {
        Some(rt) => rt,
        None => {
            return Err(anyhow!(
                "Required param 'refresh_token' is missing from callback"
            ))
        }
    };

    let expires_at: String = match params.remove("expiry_date") {
        Some(ea) => ea,
        None => {
            return Err(anyhow!(
                "Required param 'expiry_date' is missing from callback"
            ))
        }
    };
    let expires_at: DateTime<FixedOffset> = match expires_at.parse() {
        Ok(ea) => ea,
        Err(e) => {
            eprintln!("Parse DateTime error: {:?}", e);
            return Err(anyhow!("Param 'expiry_date' is in an invalid format"));
        }
    };
    let expires_at = expires_at.naive_utc();

    Ok(AuthorizedUser {
        access_token,
        refresh_token,
        expires_at,
    })
}

#[derive(Serialize)]
struct RefreshAccessTokenRequest<'a> {
    refresh_token: &'a str,
}

#[derive(Deserialize)]
struct RefreshAccessTokenResponse {
    access_token: String,
    expiry_date: DateTime<FixedOffset>,
}

impl RefreshAccessTokenResponse {
    fn into_authorized_user(self, refresh_token: &str) -> AuthorizedUser {
        AuthorizedUser {
            access_token: self.access_token,
            refresh_token: refresh_token.to_string(),
            expires_at: self.expiry_date.naive_utc(),
        }
    }
}

pub async fn refresh_access_token(refresh_token: &str) -> Result<AuthorizedUser> {
    let request_body = RefreshAccessTokenRequest { refresh_token };
    let request_body = serde_json::to_string(&request_body)?;
    let request = surf::post(format!("{}/login/oauth/refresh", floq_domain()))
        .header("Content-Type", "application/json")
        .body(request_body);

    let mut response = request.send()
        .await
        .handle_floq_response()
        .with_context(|| "Noe gikk galt under oppdatering av innloggingsinformasjonen, vennligst logg inn på nytt")?;

    let tokens: RefreshAccessTokenResponse = response
        .body_json()
        .await
        .handle_malformed_body()
        .with_context(|| "Klarte ikke å lese responsen fra /login/oauth/refresh")?;

    Ok(tokens.into_authorized_user(refresh_token))
}
