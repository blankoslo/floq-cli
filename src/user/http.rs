use super::Employee;
use crate::http_client::floq_api_domain;

use std::error::Error;

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

pub async fn get_logged_in_employee(access_token: &str) -> Result<Employee, Box<dyn Error>> {
    let mut response = surf::post(format!("{}/rpc/who_am_i", floq_api_domain()))
        .header("Accept", "application/json")
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await?;

    let response: [EmployeeResponse; 1] = response.body_json().await.map_err(|e| {
        eprintln!("{:?}", e.status());
        eprintln!("{:?}", e);
        "Could not parse"
    })?;
    let [result] = response;

    Ok(result.into_employee())
}
