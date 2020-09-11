mod http_client;

use std::env;
use http_client::HTTPClient;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct IP {
    origin: String
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    println!("{}", args[1]);
    println!("{}", args[2]);

    let http_client = HTTPClient {
        authorization: "hello".to_string()
    };
    let ip: IP = http_client.get()?;
    println!("{:#?}", ip);
    Ok(())
}
