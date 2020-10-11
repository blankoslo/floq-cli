use std::error::Error;
use std::thread;

use serde::Serialize;

const CLIENT_ID: &str = "1085640931155-rmrpe3dceqispubqi9nagk7ansqfjm41";
const CLIENT_SCRET: &str = "HSTMJ_7NYLkvdL-tqC4q1Squ";
const RESPONSE_TYPE: &str =  "code";
const SCOPES: &str = "email profile";

pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Serialize)]
struct QueryParams {
    client_id: &'static str,
    redirect_uri: String,
    response_type: &'static str,
    scopes: &'static str,
}

// TODO code_challenge & code_challenge_method
pub async fn authorize() -> Result<OAuthTokens, Box<dyn Error>> {
    // used this guide by Google: https://developers.google.com/identity/protocols/oauth2/native-app#android
    
    let server = rouille::Server::new("0.0.0.0:0", move |request| {
        rouille::Response::text("hello world")
    }).map_err(|e| format!("{}", e))?;

    println!("Started server listening on port {}", server.server_addr().port());

    let params = QueryParams { 
        client_id : CLIENT_ID,
        redirect_uri: format!("http://127.0.0.1:{}", &server.server_addr().port()),
        response_type: RESPONSE_TYPE,
        scopes: SCOPES,
    };
    let request = surf::get("https://accounts.google.com/o/oauth2/v2/auth")
    .query(&params)?;

    println!("{:?}", request);
    let mut response: surf::Response = request.send().await?;

    print!("{}", response.body_string().await?);

    Ok(OAuthTokens {
        access_token: "".to_string(),
        refresh_token: "".to_string(),
    })
}
