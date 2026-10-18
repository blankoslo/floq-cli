use crate::{
    config::FloqConfig, http_client::FloqApiClient, session::FloqSessionHandler, timestamp::time::DaySelection,
};

use std::io::Write;

use anyhow::{Result, anyhow};
use chrono::{Duration, Utc};
use clap::Args;
use futures::{StreamExt, stream::FuturesOrdered};

pub mod history;
mod http;
mod time;

#[derive(Args)]
pub struct TimestampArgs {
    /// Prosjektet (id eller alias) du ønsker å føre timer på
    prosjekt: String,

    /// Antall timer du ønsker å føre, settes til "7.5" hvis utelatt
    timer: Option<f32>,

    #[command(flatten)]
    dato: time::DaySelectionArgs,
}

pub async fn execute<T: Write + Send>(
    args: TimestampArgs,
    out: &mut T,
    session_handler: &mut FloqSessionHandler,
) -> Result<()> {
    let config = FloqConfig::from_file().await?;

    let project_id = match config.find_project_alias_by_alias(&args.prosjekt) {
        Some(pa) => pa.project_id.as_str(),
        None => args.prosjekt.as_str(),
    };

    let hours = args.timer.unwrap_or(7.5);
    let time = Duration::minutes((hours * 60.0) as i64);
    if time > Duration::days(1) {
        return Err(anyhow!("Det er ikke mulig å føre {} timer på én dag", hours));
    }

    let today = Utc::now().date_naive();
    let dates: Vec<(String, chrono::NaiveDate)> = match args.dato.to_day_selection()? {
        DaySelection::None => {
            // Default to today if no date selection is made
            vec![(today.to_string(), today)]
        }
        DaySelection::Weekdays { week, days } => {
            if let Some(days) = days {
                let week = week.resolve(today);
                days.into_iter()
                    .map(|weekday| (weekday.to_string(), week.resolve_weekday(weekday)))
                    .collect()
            } else {
                return Err(anyhow!("Du må velge ukedagen(e) du ønsker å føre timer på"));
            }
        }
        DaySelection::Dates(days) => days
            .into_iter()
            .map(|date| (date.format("%Y-%m-%d").to_string(), date))
            .collect(),
    };

    let session = session_handler.open(out, Duration::minutes(1)).await?;
    let client = FloqApiClient::from_session(session);

    let employee_id = client.get_logged_in_employee().await?.id;

    let mut futures: FuturesOrdered<_> = dates
        .into_iter()
        .map(async |(label, date)| {
            (
                label,
                client
                    .set_timestamp_and_get_previous(employee_id, project_id, &date, &time)
                    .await,
            )
        })
        .collect();
    while let Some((label, r)) = futures.next().await {
        let previous_time = r?;

        writeln!(
            out,
            "Førte {} ({}) på {} for {}",
            time::Hours(&time),
            time::HoursDiff(time - previous_time),
            config
                .find_project_alias_by_project_id(project_id)
                .map(|pa| pa.alias.as_str())
                .unwrap_or(project_id),
            label,
        )?;
    }
    Ok(())
}
