use crate::http_client::FLOQ_DOMAIN;

use std::{collections::HashMap, sync::mpsc};
use std::{error::Error, time::Duration};

use chrono::{DateTime, FixedOffset, NaiveDateTime};
use rouille::{Request, Response};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct AuthorizedUser {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: NaiveDateTime,
}

pub async fn authorize() -> Result<AuthorizedUser, Box<dyn Error>> {
    let (tx, rx) = mpsc::sync_channel::<Result<AuthorizedUser, String>>(0);

    let server = rouille::Server::new("0.0.0.0:0", move |request| {
        match handle_callback(request) {
            Ok(tokens) => {
                tx.send(Ok(tokens)).unwrap();
                Response::text("Flott, da er du logget inn i floq cli!\n\n(Bare å lukke denne fanen)")
            },
            Err(e) => {
                eprintln!("Error on handle callback from Floq Auth: {}", e);
                tx.send(Err(e.to_string())).unwrap();
                Response::text("An error occurred while trying to handle Auth callback, see command output for more details")
            }
        }
    })
    .map_err(|e| format!("{}", e))?;

    let port = server.server_addr().port();

    println!("Venligst åpne denne lenken i nettleseren din:");
    println!("{}/login/oauth?to=http://localhost:{}", FLOQ_DOMAIN, port);

    loop {
        match rx.try_iter().next() {
            Some(Ok(tokens)) => break Ok(tokens),
            Some(Err(e)) => break Err(e.into()),
            None => {
                std::thread::sleep(Duration::from_millis(250));
                server.poll();
            }
        }
    }
}

fn handle_callback(request: &Request) -> Result<AuthorizedUser, &str> {
    let mut params =
        match serde_urlencoded::from_str::<HashMap<String, String>>(request.raw_query_string()) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Deserialization of query params failed: {:?}", e);
                return Err("Unable to parse callback request URL");
            }
        };

    let access_token = match params.remove("access_token") {
        Some(at) => at,
        None => return Err("Required param 'access_token' is missing from callback"),
    };

    let refresh_token = match params.remove("refresh_token") {
        Some(rt) => rt,
        None => return Err("Required param 'refresh_token' is missing from callback"),
    };

    let expires_at: String = match params.remove("expiry_date") {
        Some(ea) => ea,
        None => return Err("Required param 'expiry_date' is missing from callback"),
    };
    let expires_at: DateTime<FixedOffset> = match expires_at.parse() {
        Ok(ea) => ea,
        Err(e) => {
            eprintln!("Parse DateTime error: {:?}", e);
            return Err("Param 'expiry_date' is in an invalid format");
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

pub async fn refresh_access_token(refresh_token: &str) -> Result<AuthorizedUser, Box<dyn Error>> {
    let request_body = RefreshAccessTokenRequest { refresh_token };
    let request_body = serde_json::to_string(&request_body)?;
    let request = surf::post(format!("{}/login/oauth/refresh", FLOQ_DOMAIN))
        .header("Content-Type", "application/json")
        .body(request_body);

    let mut response = request.send().await?;

    if response.status().is_client_error() || response.status().is_server_error() {
        Err(format!("Got status {}", response.status()).into())
    } else {
        let tokens: RefreshAccessTokenResponse = response.body_json().await.map_err(|e| {
            println!("{:?}", e);
            e
        })?;

        Ok(tokens.into_authorized_user(refresh_token))
    }
}
