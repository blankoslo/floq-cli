use crate::{cmd::Subcommand, http_client::HTTPClient, print, user};

use std::{collections::HashMap, error::Error, fmt::Display, io::Write};

use async_trait::async_trait;
use chrono::{Datelike, Duration, NaiveDate, Utc};
use clap::{App, Arg, ArgMatches};

mod http;

const SUBCOMMAND_NAME: &'static str = "timeføring";

pub fn subcommand_app<'help>() -> App<'help> {
    App::new("timeføring")
        .about("Før timer på et prosjekt")
        .arg(Arg::new("prosjekt").about("Prosjektet du ønsker å føre timer på").index(1).required_unless_present("historikk"))
        .arg(
            Arg::new("dato")
                .long("dato")
                .short('d')
                .takes_value(true)
                .conflicts_with_all(&["fra", "til"])
                .about("Dagen det gjelder, settes til i dag hvis utelatt. Eksempel \"2021-03-01\""),
        )
        .arg(
            Arg::new("timer")
                .long("timer")
                .short('t')
                .takes_value(true)
                .default_value("7.5")
                .about("Antall timer du ønsker å føre")
        )
        .arg(
            Arg::new("fra")
                .long("fra")
                .takes_value(true)
                .requires("til")
                .conflicts_with("dato")
                .about(
                    "Første dagen å vise timer for, settes til mandag denne uken hvis utelatt. Eksempel \"2021-03-01\" ",
                ),
        )
        .arg(
            Arg::new("til")
                .long("til")
                .takes_value(true)
                .requires("fra")
                .conflicts_with("dato")
                .about(
                    "Siste dagen å vise timer for, settes til fredag denne uken hvis utelatt. Eksempel \"2021-03-05\"",
                ),
        )
        .arg(
            Arg::new("historikk")
                .long("historikk")
                .short('h')
                .about(
                    "Vis timeføring, periode kan velges parametere med \"dato\", med \"fra\" og \"til\" eller ingen for å vise denne uken"
                )
        )
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
        let user = user::load_user_from_config().await?;
        let client = HTTPClient::from_user(&user);

        if (matches.is_present("historikk")) {
            execute_history(matches, out, client).await
        } else {
            execute_timestamp(matches, out, client).await
        }
    }
}

async fn execute_timestamp<T: Write + Send>(matches: &ArgMatches, out: &mut T, client: HTTPClient) -> Result<(), Box<dyn Error>> {
    let project_id = matches.value_of("prosjekt").unwrap();
    let date: NaiveDate = matches.value_of("dato")
        .map(|date| date.parse::<NaiveDate>())
        .unwrap_or_else(|| Ok(Utc::now().date().naive_local()))?;
    let hours: f32 = matches.value_of("timer").unwrap().parse()?;
    let time = Duration::minutes((hours * 60.0) as i64);

    let current_time = client
        .get_timestamp_on_project_for_date(project_id, date)
        .await?;
    let time_diff = time - current_time;

    if time_diff.is_zero() {
        write!(
            out,
            "Ingen endring ble gjort siden det allerede er timeført {} den dagen",
            TimestampHours::from(time)
        )?;
        Ok(())
    } else {
        client
            .add_timestamp(project_id, date, time_diff)
            .await
            .map(|_| ())
    }   
}

async fn execute_history<T: Write + Send>(matches: &ArgMatches, out: &mut T, client: HTTPClient) -> Result<(), Box<dyn Error>> {
    if matches.is_present("dato") {
        write!(out, "{:?}", matches)?;

        let date = matches.value_of("dato").unwrap().parse()?;

        let mut timestamps = client.get_timestamps_for_date(date).await?;
        timestamps.sort_by(|t0, t1| t0.project_id.cmp(&t1.project_id));

        let mut table_maker = print::TableMaker::new();

        table_maker.titles(vec![
            "PROSJEKT".to_string(),
            date.format("%Y-%m-%d (%a)").to_string()
        ]);
    
        table_maker.with(Box::new(|pt: &ProjectTimestamp| pt.project_id.clone()));
        table_maker.with(Box::new(move |pt| TimestampHours::from(pt.timestamp.time).to_string()));

        table_maker.into_table(timestamps.as_slice()).print(out)?;
    } else {
        let from = matches
            .value_of("fra")
            .map(|from| from.parse::<NaiveDate>())
            .unwrap_or_else(|| {
                let today = Utc::now().date();
                let days_from_monday = today.weekday().num_days_from_monday();
        
                Ok(today.naive_local() - Duration::days(1) * days_from_monday as i32)
            })?;
        let to = matches
            .value_of("til")
            .map(|from| from.parse::<NaiveDate>())
            .unwrap_or_else(|| {
                let today = Utc::now().date();
                let days_from_monday = today.weekday().num_days_from_monday();
                let friday = today + Duration::days(4 - days_from_monday as i64);
    
                Ok(friday.naive_local())
            })?;

        let mut timestamps = get_timestamps_for_period(client, from, to).await?;
        timestamps.sort_by(|t0, t1| t0.project_id.cmp(&t1.project_id));

        let mut table_maker = print::TableMaker::new();

        let titles = from.iter_days().take_while(|d| d <= &to).fold(
            vec!["PROSJEKT".to_string()],
            |mut titles, next| {
                titles.push(next.format("%Y-%m-%d (%a)").to_string());
                titles
            },
        );
        table_maker.titles(titles);
    
        table_maker.with(Box::new(|pt: &ProjectTimestamps| pt.project_id.clone()));
        from.iter_days().take_while(|d| d <= &to).for_each(|d| {
            table_maker.with(Box::new(move |pt| {
                pt.find_timestamp_for_date(&d)
                    .map(|ts| TimestampHours::from(ts.time).to_string())
                    .unwrap_or(String::new())
            }));
        });

        table_maker.into_table(timestamps.as_slice()).print(out)?;
    }

    Ok(())
}

pub struct ProjectTimestamp {
    pub project_id: String,
    pub project_name: String,
    pub customer_name: String,
    pub timestamp: Timestamp,
}

impl ProjectTimestamp {
    fn into_project_timestamps(self) -> ProjectTimestamps {
        ProjectTimestamps {
            project_id: self.project_id,
            project_name: self.project_name,
            customer_name: self.customer_name,
            timestamps: vec![self.timestamp],
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ProjectTimestamps {
    pub project_id: String,
    pub project_name: String,
    pub customer_name: String,
    pub timestamps: Vec<Timestamp>,
}

impl ProjectTimestamps {
    pub fn find_timestamp_for_date(&self, date: &NaiveDate) -> Option<&Timestamp> {
        self.timestamps.iter().find(|t| t.date == *date)
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Timestamp {
    pub date: NaiveDate,
    pub time: Duration,
}

impl Timestamp {
    pub fn is_time_zero(&self) -> bool {
        self.time == Duration::zero()
    }
}

pub struct TimestampHours(Duration);

impl From<Duration> for TimestampHours {
    fn from(duration: Duration) -> Self {
        Self(duration)
    }
}

impl Display for TimestampHours {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}t",
            self.0.num_hours(),
            (self.0 - Duration::hours(self.0.num_hours())).num_minutes() / 6
        )
    }
}

pub async fn get_timestamps_for_period(
    client: HTTPClient,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<ProjectTimestamps>, Box<dyn Error>> {
    let project_timestamps = client.get_timestamps_for_period(from, to).await?;

    let project_to_timestamps: HashMap<String, ProjectTimestamps> = project_timestamps
        .into_iter()
        .fold(HashMap::new(), |mut res, next| {
            let key = next.project_id.clone();
            let value = match res.remove(&next.project_id) {
                Some(mut pt) => {
                    pt.timestamps.push(next.timestamp);
                    pt
                }
                None => next.into_project_timestamps(),
            };

            res.insert(key, value);
            res
        });

    Ok(project_to_timestamps.into_iter().map(|(_k, v)| v).collect())
}
