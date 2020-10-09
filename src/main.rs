mod http_client;

use http_client::timetrack::HTTPClient;
use serde::Deserialize;
use std::env;
use dotenv::dotenv;

#[derive(Deserialize, Debug)]
struct IP {
    origin: String
}
fn get_envvar(key: &str) -> String {
    let envvar = env::var(key);
    if envvar.is_err() {
        panic!("env var {} not defined", key)
    }
    return envvar.unwrap()
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let bearer_token = get_envvar("BEARER_TOKEN");
    let http_client = HTTPClient::new(bearer_token);
    let projects = http_client.get_current_week_timetracks(77)?;
    println!("{:#?}", projects);
    Ok(())
}
