mod http_client;

use http_client::timetrack::HTTPClient;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct IP {
    origin: String
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let http_client = HTTPClient::new();
    let projects = http_client.get_current_week_timetracks(77)?;
    println!("{:#?}", projects);
    Ok(())
}
