use std::error::Error;

use super::HTTPClient;

use serde::Deserialize;
use surf::Response;

#[derive(Deserialize, Debug)]
pub struct Project {
    id: String,
    name: String,
    active: bool,
    customer: Customer,
}

#[derive(Deserialize, Debug)]
pub struct Customer {
    id: String,
    name: String,
}

impl HTTPClient {
    pub async fn get_projects(&self) -> Result<Vec<Project>, Box<dyn Error>> {
        let mut response: Response =
            surf::get("https://api-blank.floq.no/projects?select=id,name,active,customer{id,name}")
                .header("Accept", "application/json")
                .header("Authorization", format!("Bearer {}", self.bearer_token))
                .send()
                .await?;

        let projects: Vec<Project> = response.body_json().await?;

        Ok(projects)
    }
}
