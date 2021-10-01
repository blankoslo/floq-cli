use super::Employee;
use crate::http_client::{floq_api_domain, HandleInvalidToken, HandleMalformedBody};

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

pub async fn get_logged_in_employee(access_token: &str) -> Result<Employee> {
    let mut response = surf::post(format!("{}/rpc/who_am_i", floq_api_domain()))
        .header("Accept", "application/json")
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .handle_floq_response()
        .with_context(|| "Noe gikk galt under henting av informasjon om deg")?;

    let response: [EmployeeResponse; 1] = response
        .body_json()
        .await
        .handle_malformed_body()
        .with_context(|| "Klarte ikke å lese responsen fra /rpc/who_am_i")?;
    let [result] = response;

    Ok(result.into_employee())
}
