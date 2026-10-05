use super::Employee;
use crate::http_client::{HandleInvalidToken, HandleMalformedBody, UnauthorizedHttpClient, floq_api_domain};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Deserialize)]
struct EmployeeResponse {
    id: u16,
    email: String,
    first_name: String,
    last_name: String,
    // more fields are available
}

impl EmployeeResponse {
    fn into_employee(self) -> Employee {
        Employee {
            id: self.id,
            email: self.email,
            name: format!("{} {}", self.first_name, self.last_name),
        }
    }
}

impl UnauthorizedHttpClient {
    pub async fn get_logged_in_employee(&self, access_token: &str) -> Result<Employee> {
        Ok(self
            .client
            .post(format!("{}/rpc/who_am_i", floq_api_domain()))
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .handle_floq_response()
            .await
            .context("Noe gikk galt under henting av informasjon om deg")?
            .json::<EmployeeResponse>()
            .await
            .handle_malformed_body()
            .context("Klarte ikke å lese responsen fra /rpc/who_am_i")?
            .into_employee())
    }
}
