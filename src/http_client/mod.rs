pub mod projects;
pub mod timetrack;

use std::env;

pub const FLOQ_DOMAIN: &str = env!("FLOQ_DOMAIN");
pub const FLOQ_API_DOMAIN: &str = env!("FLOQ_API_DOMAIN");

pub struct HTTPClient {
    pub bearer_token: String,
}

impl HTTPClient {
    pub fn new(bearer_token: String) -> HTTPClient {
        HTTPClient { bearer_token }
    }
}
