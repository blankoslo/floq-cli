pub mod projects;
pub mod timetrack;

pub struct HTTPClient {
    bearer_token: String,
    employee_id: u32,
}

impl HTTPClient {
    pub fn new(bearer_token: String, employee_id: u32) -> HTTPClient {
        HTTPClient {
            bearer_token,
            employee_id,
        }
    }
}
