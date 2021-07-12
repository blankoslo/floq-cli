use crate::{cmd::Subcommand, http_client::HttpClient, time, user};

use std::{error::Error, fmt::Display, io::Write};

use async_trait::async_trait;
use chrono::{Datelike, Duration, NaiveDate, Utc};
use clap::{App, AppSettings, Arg, ArgMatches};
use futures::{stream::FuturesUnordered, StreamExt};

pub mod history;
mod http;

const SUBCOMMAND_NAME: &str = "timeføring";

pub fn subcommand_app<'help>() -> App<'help> {
    let days = time::Weekdays::all();

    let day_args: Vec<Arg> = days
        .iter()
        .map(|day| {
            let weekday = day.get_weekday();
            let conflicts: Vec<&str> = days
                .iter()
                .filter(|&d| d != day)
                .map(|d| d.get_weekday().full_name)
                .collect();

            Arg::new(weekday.full_name)
                .long(weekday.full_name)
                .visible_alias(weekday.short_name)
                .conflicts_with("dato")
                .conflicts_with_all(&conflicts)
                .about("Ukedag det skal føres timer på, relativt til dagens dato")
                .display_order(day.as_chrono_weekday().num_days_from_monday() as usize + 1)
        })
        .collect();

    App::new(SUBCOMMAND_NAME)
        .about("Før timer på et prosjekt")
        .setting(AppSettings::SubcommandsNegateReqs)
        .arg(Arg::new("prosjekt").about("Prosjektet du ønsker å føre timer på").required(true).index(1))
        .arg(
            Arg::new("timer")
                .long("timer")
                .short('t')
                .takes_value(true)
                .default_value("7.5")
                .hide_default_value(true)
                .about("Antall timer du ønsker å føre, settes til \"7.5\" hvis utelatt")
        )
        .arg(
            Arg::new("dato")
                .long("dato")
                .short('d')
                .takes_value(true)
                .about("Dagen det skal føres timer på, settes til i dag hvis utelatt.\nF.eks. \"--dato 2021-03-01\""),
        )
        .arg(
            Arg::new("fra")
                .long("fra")
                .takes_value(true)
                .requires("til")
                .conflicts_with("dato")
                .about(
                    "Brukes samme med --til for å føre timer i en periode, er inklusiv.\nF.eks. \"--fra 2021-03-01\" ",
                ),
        )
        .arg(
            Arg::new("til")
                .long("til")
                .takes_value(true)
                .requires("fra")
                .conflicts_with("dato")
                .about(
                    "Brukes samme med --fra for å føre timer i en periode, er inklusiv.\nF.eks. \"--til 2021-03-05\"",
                ),
        )
        .arg(
            Arg::new("forrige-uke")
                .long("forrige-uke")
                .conflicts_with_all(&["neste-uke", "dato", "fra", "til" ])
                .display_order(8) // one more than --søndag
                .about(
                    "Setter relativ dato til forrige uke. Brukes sammen med ukedagene til å velge en dag i forrige uke"
                )
        )
        .arg(
            Arg::new("neste-uke")
                .long("neste-uke")
                .conflicts_with_all(&["forrige-uke", "dato", "fra", "til" ])
                .display_order(9) // one more than --forrige-uke
                .about(
                    "Setter relativ dato til neste uke. Brukes sammen med ukedagene til å velge en dag i neste uke"
                )
        )
        .args(day_args)
}

pub fn subcommand<T: Write + Send>() -> Box<dyn Subcommand<T>> {
    Box::new(TimestampSubcommand)
}

struct TimestampSubcommand;

#[async_trait(?Send)]
impl<T: Write + Send> Subcommand<T> for TimestampSubcommand {
    fn matches(&self, matches: &ArgMatches) -> bool {
        matches.subcommand_name() == Some(SUBCOMMAND_NAME)
    }

    async fn execute(&self, matches: &ArgMatches, out: &mut T) -> Result<(), Box<dyn Error>> {
        let user = user::load_user_from_config(out).await?;
        let client = HttpClient::from_user(&user);

        execute(matches, out, client).await
    }
}

pub struct TimestampHours<'a>(&'a Duration);

impl<'a> Display for TimestampHours<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}t",
            self.0.num_hours(),
            (*self.0 - Duration::hours(self.0.num_hours())).num_minutes() / 6
        )
    }
}
pub struct TimestampDate<'a>(&'a NaiveDate);

impl<'a> Display for TimestampDate<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let weekdays : time::Weekdays =  self.0.into();

        write!(
            f,
            "{} ({})",
            self.0.format("%Y-%m-%d").to_string(),
            weekdays.get_weekday().short_name
        )
    }
}

struct SetTimestampResult<'a> {
    project_id: &'a str,
    time: &'a Duration,
    date: &'a NaiveDate,
    time_diff: Duration,
}

async fn execute<T: Write + Send>(
    matches: &ArgMatches,
    out: &mut T,
    client: HttpClient,
) -> Result<(), Box<dyn Error>> {
    let project_id = matches.value_of("prosjekt").unwrap();

    let hours: f32 = matches.value_of("timer").unwrap().parse()?;
    let time = Duration::minutes((hours * 60.0) as i64);
    if time > Duration::days(1) {
        return Err(format!("Det er ikke mulig å føre {} timer på én dag", hours).into());
    }

    let dates = if matches.is_present("fra") {
        let from: NaiveDate = matches.value_of("fra").unwrap().parse::<NaiveDate>()?;
        let to: NaiveDate = matches.value_of("til").unwrap().parse::<NaiveDate>()?;

        from.iter_days().take_while(|d| d <= &to).collect()
    } else if matches.is_present("dato") {
        let date: NaiveDate = matches
            .value_of("dato")
            .map(|date| date.parse::<NaiveDate>())
            .unwrap_or_else(|| Ok(Utc::now().date().naive_local()))?;

        vec![date]
    } else {
        let weekdays = time::Weekdays::all();
        let weekday = weekdays
            .iter()
            .find(|w| matches.is_present(w.get_weekday().full_name))
            .expect("Unknown flags and options should be handled by the library");

        let today = Utc::now().date();
        let base_date = if matches.is_present("forrige-uke") {
            today - Duration::weeks(1)
        } else if matches.is_present("neste-uke") {
            today + Duration::weeks(1)
        } else {
            today
        };

        let days_from_monday = base_date.weekday().num_days_from_monday();
        let days_until_date =
            weekday.as_chrono_weekday().num_days_from_monday() as i64 - days_from_monday as i64;
        let date = base_date + Duration::days(days_until_date);

        vec![date.naive_local()]
    };

    let mut futures: FuturesUnordered<_> = dates
        .iter()
        .map(|date| set_timetsamp(project_id, &time, date, &client))
        .collect();
    while let Some(r) = futures.next().await {
        let set_timestamp_result = r?;

        if set_timestamp_result.time_diff.is_zero() {
            writeln!(
                out,
                "Du har allerede ført {} på {} for {}",
                TimestampHours(set_timestamp_result.time),
                set_timestamp_result.project_id,
                TimestampDate(set_timestamp_result.date),
            )?;
        } else {
            writeln!(
                out,
                "Førte {} på {} for {}",
                TimestampHours(set_timestamp_result.time),
                set_timestamp_result.project_id,
                TimestampDate(set_timestamp_result.date),
            )?;
        }
    }
    Ok(())
}

async fn set_timetsamp<'a>(
    project_id: &'a str,
    time: &'a Duration,
    date: &'a NaiveDate,
    client: &HttpClient,
) -> Result<SetTimestampResult<'a>, Box<dyn Error>> {
    let current_time = client
        .get_timestamp_on_project_for_date(project_id, date)
        .await?;
    let time_diff = *time - current_time;

    if !time_diff.is_zero() {
        client
            .add_timestamp(project_id, date, time_diff)
            .await
            .map(|_| ())?;
    }

    Ok(SetTimestampResult {
        project_id,
        time,
        date,
        time_diff,
    })
}
