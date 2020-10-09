mod http_client;

use async_std::task;

use chrono::{Duration, NaiveDate};

pub const TOKEN: &str = "";
pub const EMPLOYEE_ID: u32 = 0;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    task::block_on(demo())
}

async fn demo() -> Result<(), Box<dyn std::error::Error>> {
    let projects = http_client::projects::get_projects().await?;
    println!("Projects:");
    println!("{:#?}", projects);

    let current_week_timetrackings =
        http_client::timetrack::get_current_week_timetracking(EMPLOYEE_ID).await?;
    println!();
    println!("Current week timetrackings:");
    println!("{:#?}", current_week_timetrackings);

    http_client::timetrack::timetrack(
        77,
        "SVO1000".to_string(),
        NaiveDate::from_ymd(2020, 10, 09),
        Duration::hours(7) + Duration::minutes(30),
    )
    .await?;

    println!("Done timetracking!");
    Ok(())
}
