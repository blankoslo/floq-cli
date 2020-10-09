pub mod projects;
pub mod timetrack;

pub struct HTTPClient {
    bearer_token: String,
}

impl HTTPClient {
    pub fn new(bearer_token: String) -> HTTPClient {
        HTTPClient { bearer_token }
    }
}
