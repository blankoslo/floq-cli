use crate::user::User;

use std::option_env;

pub const FLOQ_DOMAIN: Option<&str> = option_env!("FLOQ_DOMAIN");
pub const FLOQ_API_DOMAIN: Option<&str> = option_env!("FLOQ_API_DOMAIN");

pub fn floq_domain() -> &'static str {
    FLOQ_DOMAIN.unwrap_or("https://blank-test.floq.no")
}

pub fn floq_api_domain() -> &'static str {
    FLOQ_API_DOMAIN.unwrap_or("https://api-blank-test.floq.no")
}

pub struct HttpClient {
    pub access_token: String,
    pub employee_id: u16,
}

impl HttpClient {
    pub fn from_user(user: &User) -> Self {
        Self {
            access_token: user.access_token.clone(),
            employee_id: user.employee_id,
        }
    }
}
