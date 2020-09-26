use std::error::Error;

use serde::Deserialize;

use super::HTTPClient;

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
    pub fn get_projects(&self) -> Result<Vec<Project>, Box<dyn Error>> {
        let response: reqwest::blocking::Response = self.client.get("https://api-blank.floq.no/projects?select=id,name,active,customer{id,name}")
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", ))
            .send()?;

        let projects: Vec<Project> = response.json()?;

        Ok(projects)
    }
}
