pub mod projects;
pub mod timetrack;

use std::env;

use crate::user::User;

pub const FLOQ_DOMAIN: &str = env!("FLOQ_DOMAIN");
pub const FLOQ_API_DOMAIN: &str = env!("FLOQ_API_DOMAIN");

pub struct HTTPClient {
    access_token: String,
    employee_id: u16,
}

impl HTTPClient {
    pub fn from_user(user: &User) -> Self {
        Self {
            access_token: user.access_token.clone(),
            employee_id: user.employee_id,
        }
    }
}
