use reqwest::blocking::Client;

pub mod timetrack;
pub mod projects;

pub struct HTTPClient {
    client: Client
}

impl HTTPClient {
    
    pub fn new() -> Self {
        Self {
            client: Client::new()
        }
    }
}