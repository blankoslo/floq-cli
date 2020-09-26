mod http_client;

use http_client::HTTPClient;

use chrono::{Duration, NaiveDate};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let http_client = HTTPClient::new();

    let projects = http_client.get_projects()?; 
    println!("Projects:");
    println!("{:#?}", projects);

    let current_week_timetrackings = http_client.get_current_week_timetracking(77)?;
    println!();
    println!("Current week timetrackings:");
    println!("{:#?}", current_week_timetrackings);

    http_client.timetrack(77, "SVO1000".to_string(), NaiveDate::from_ymd(2020, 09, 25), Duration::minutes(30))?;
    Ok(())
}
