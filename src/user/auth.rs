use std::{collections::HashMap, sync::mpsc, time::SystemTime};
use std::{error::Error, time::Duration};

use chrono::{DateTime, NaiveDateTime, Utc};
use rouille::Response;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use rand::Rng;

const CLIENT_ID: &str = "1085640931155-rmrpe3dceqispubqi9nagk7ansqfjm41.apps.googleusercontent.com";
const CLIENT_SECRET: &str = "HSTMJ_7NYLkvdL-tqC4q1Squ";
const RESPONSE_TYPE: &str = "code";
const SCOPES: &str = "email profile";
const AUTHORIZATION_GRANT_TYPE: &str = "authorization_code";
const REFRESH_GRANT_TYPE: &str = "refresh_token";
const CODE_VERIFIER_LENGTH: usize = 128;
const CODE_CHALLENGE_METHOD: &str = "S256";

#[derive(Debug)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub issued_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
}

#[derive(Debug, Serialize)]
struct AuthorizationParams {
    client_id: &'static str,
    redirect_uri: String,
    response_type: &'static str,
    scope: &'static str,
    code_challenge: String,
    code_challenge_method: &'static str,
}

impl AuthorizationParams {
    fn new(port: u16, code_challenge: String) -> Self {
        Self {
            client_id: CLIENT_ID,
            redirect_uri: format!("http://127.0.0.1:{}", port),
            response_type: RESPONSE_TYPE,
            scope: SCOPES,
            code_challenge,
            code_challenge_method: CODE_CHALLENGE_METHOD,
        }
    }
}

#[derive(Debug, Serialize)]
struct TokenExchangeRequest {
    client_id: &'static str,
    client_secret: &'static str,
    code: String,
    grant_type: &'static str,
    redirect_uri: String,
    code_verifier: String,
}

impl TokenExchangeRequest {
    fn new(port: u16, exchange_code: String, code_verifier: String) -> Self {
        Self {
            client_id: CLIENT_ID,
            client_secret: CLIENT_SECRET,
            code: exchange_code,
            grant_type: AUTHORIZATION_GRANT_TYPE,
            redirect_uri: format!("http://127.0.0.1:{}", port),
            code_verifier,
        }
    }
}

#[derive(Debug, Deserialize)]
struct TokenExchangeResponse {
    access_token: String,
    expires_in: u16,
    refresh_token: String,
}

impl TokenExchangeResponse {
    fn into_oauth_token(self) -> OAuthTokens {
        let now: DateTime<Utc> = DateTime::from(SystemTime::now());
        let issued_at = now.naive_utc();
        let expires_at = issued_at + chrono::Duration::seconds(self.expires_in as i64);
        OAuthTokens {
            access_token: self.access_token,
            refresh_token: self.refresh_token,
            issued_at,
            expires_at,
        }
    }
}

// TODO code_challenge & code_challenge_method
pub async fn authorize() -> Result<OAuthTokens, Box<dyn Error>> {
    let (tx, rx) = mpsc::sync_channel::<Result<String, String>>(0);

    // used this guide by Google: https://developers.google.com/identity/protocols/oauth2/native-app
    let server = rouille::Server::new("0.0.0.0:0", move |request| {
        let mut params = serde_urlencoded::from_str::<HashMap<String, String>>(request.raw_query_string())
            .expect("Unable to parse callback request URL");
            
        let code = params.remove("code").expect("Required param 'code' is missing from callback URL");

        if params.remove("hd") == Some("blank.no".to_string()) {
            tx.send(Ok(code)).unwrap();
            Response::text("Thx m8!\n\n(You can close this tab)")
        } else {
            tx.send(Err("Param 'hd' was an unexpected value".to_string())).unwrap();
            Response::text("Wrong hosted domain, please log in using a 'blank.no' user")
        }
    })
    .map_err(|e| format!("{}", e))?;

    let port = server.server_addr().port();
    let (code_verifier, code_challenge) = generate_code_verifier_and_challenge();
    let authorization_params = AuthorizationParams::new(port, code_challenge);
    let request = surf::get("https://accounts.google.com/o/oauth2/v2/auth")
        .query(&authorization_params)
        .unwrap()
        .build();

    println!("Please open this link in your favorite browser, if you please:");
    println!("{}", request.url());

    let exchange_code = loop {
        match rx.try_iter().next() {
            Some(exchange_code) => break exchange_code.unwrap(),
            None => {
                std::thread::sleep(Duration::from_millis(250));
                server.poll();
            }
        }
    };

    let token_exchange_request = TokenExchangeRequest::new(port, exchange_code, code_verifier);
    let encoded_token_exchange_request = serde_urlencoded::to_string(token_exchange_request)?;
    let mut response = surf::post("https://oauth2.googleapis.com/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(encoded_token_exchange_request)
        .send()
        .await?;

    let tokens: TokenExchangeResponse = response.take_body().into_json().await?;
    Ok(tokens.into_oauth_token())
}

fn generate_code_verifier_and_challenge() -> (String, String) {
    let random_bytes: Vec<u8> = (0..CODE_VERIFIER_LENGTH).map(|_| rand::thread_rng().gen::<u8>()).collect();
    let code_verifier = base64::encode_config(
        &random_bytes,
        base64::URL_SAFE_NO_PAD,
    );
    let hashed = Sha256::digest(code_verifier.as_bytes());
    let code_challenge = base64::encode_config(&hashed, base64::URL_SAFE_NO_PAD);

    (code_verifier, code_challenge)
}

#[derive(Serialize)]
struct RefreshAccessTokenRequest<'a> {
    client_id: &'static str,
    client_secret: &'static str,
    grant_type: &'static str,
    refresh_token: &'a str,
}

impl<'a> RefreshAccessTokenRequest<'a> {
    fn new(refresh_token: &'a str) -> Self {
        Self {
            client_id: CLIENT_ID,
            client_secret: CLIENT_SECRET,
            grant_type: REFRESH_GRANT_TYPE,
            refresh_token,
        }
    }
}

#[derive(Deserialize)]
struct RefreshAccessTokenResponse {
    access_token: String,
    expires_in: u16,
}

impl RefreshAccessTokenResponse {
    fn into_oauth_token(self, refresh_token: &str) -> OAuthTokens {
        let now: DateTime<Utc> = DateTime::from(SystemTime::now());
        let issued_at = now.naive_utc();
        let expires_at = issued_at + chrono::Duration::seconds(self.expires_in as i64);

        OAuthTokens {
            access_token: self.access_token,
            refresh_token: refresh_token.to_string(),
            issued_at,
            expires_at,
        }
    }
}

pub async fn refresh_access_token(refresh_token: &str) -> Result<OAuthTokens, Box<dyn Error>> {
    let request = RefreshAccessTokenRequest::new(refresh_token);
    let encoded_request = serde_urlencoded::to_string(request)?;
    let request = surf::post("https://oauth2.googleapis.com/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(encoded_request);
    
    let mut response = request
        .send()
        .await?;

    if response.status().is_client_error() || response.status().is_server_error() {
        Err(format!("Got status {} from Google Auth", response.status()).into())
    } else {
        let tokens: RefreshAccessTokenResponse = response.take_body().into_json().await?;
        Ok(tokens.into_oauth_token(refresh_token))
    }
}
