mod http_client;

use async_std::task;

use chrono::{Duration, NaiveDate};

use dotenv::dotenv;
use http_client::HTTPClient;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv()?;
    let bearer_token = get_env_var("BEARER_TOKEN");
    let employee_id = get_env_var("EMPLOYEE_ID").parse()?;

    let http_client = HTTPClient::new(bearer_token);

    task::block_on(demo(http_client, employee_id))
}

async fn demo(http_client: HTTPClient, employee_id: u32) -> Result<(), Box<dyn std::error::Error>> {
    let projects = http_client.get_projects().await?;
    println!("Projects:");
    println!("{:#?}", projects);

    let current_week_timetrackings = http_client.get_current_week_timetracking(employee_id).await?;
    println!();
    println!("Current week timetracking:");
    println!("{:#?}", current_week_timetrackings);

    http_client
        .timetrack(
            77,
            "SVO1000".to_string(),
            NaiveDate::from_ymd(2020, 10, 9),
            Duration::hours(7) + Duration::minutes(30),
        )
        .await?;

    println!("Done timetracking!");
    Ok(())
}

fn get_env_var(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("env var {} not defined", key))
}
