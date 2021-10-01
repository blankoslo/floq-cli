use super::{TimestampDate, TimestampHours};
use crate::{cmd::Subcommand, http_client::HttpClient, print, user};

use std::{collections::HashMap, io::Write};

use anyhow::Result;
use async_trait::async_trait;
use chrono::{Datelike, Duration, NaiveDate, Utc, Weekday};
use clap::{App, Arg, ArgMatches};

const SUBCOMMAND_NAME: &str = "timehistorikk";

pub fn subcommand_app<'help>() -> App<'help> {
    App::new(SUBCOMMAND_NAME)
    .about("Vis timeføring")
    .arg(
        Arg::new("dato")
            .long("dato")
            .short('d')
            .takes_value(true)
            .display_order(1)
            .about("Dagen du ønsker å vise timer for.\nF.eks. \"--dato 2021-03-01\""),
    )
    .arg(
        Arg::new("fra")
            .long("fra")
            .takes_value(true)
            .requires("til")
            .conflicts_with("dato")
            .display_order(2)
            .about(
                "Første dagen å vise timer for, settes til mandag denne uken hvis utelatt.\nEr inklusiv. F.eks. \"--fra 2021-03-01\" ",
            ),
    )
    .arg(
        Arg::new("til")
            .long("til")
            .takes_value(true)
            .requires("fra")
            .conflicts_with("dato")
            .display_order(3)
            .about(
                "Siste dagen å vise timer for, settes til fredag denne uken hvis utelatt.\nEr inklusiv. F.eks. \"--til 2021-03-05\"",
            ),
    )
    .arg(
        Arg::new("forrige-uke")
            .long("forrige-uke")
            .conflicts_with_all(&["dato", "fra", "til", "neste-uke"])
            .display_order(4)
            .about("Vis timer ført i forrige uke.")
    )
    .arg(
        Arg::new("neste-uke")
            .long("neste-uke")
            .conflicts_with_all(&["dato", "fra", "til", "forrige-uke"])
            .display_order(5)
            .about("Vis timer ført for neste uke.")
    )
    .arg(
        Arg::new("snu-tabell")
            .long("snu-tabell")
            .conflicts_with("ikke-snu-tabell")
            .display_order(6)
            .about(
"Snu om på tabellen slik at rader går fra å være per prosjekt til per dag og prosjekt.
Dette blir gjort automatisk hvis det skal vises timer for mer enn én uke."
            )
    )
    .arg(
        Arg::new("ikke-snu-tabell")
            .long("ikke-snu-tabell")
            .conflicts_with("snu-tabell")
            .display_order(7)
            .about(
"Ikke snu om på tabellen slik at rader går fra å være per prosjekt til per dag og prosjekt.
Stopper det fra å bli gjort automatisk hvis det skal vises timer for mer enn én uke."
            )
    )
}

pub fn subcommand<T: Write + Send>() -> Box<dyn Subcommand<T>> {
    Box::new(TimestampHistorySubcommand)
}

struct TimestampHistorySubcommand;

#[async_trait(?Send)]
impl<T: Write + Send> Subcommand<T> for TimestampHistorySubcommand {
    fn matches(&self, matches: &ArgMatches) -> bool {
        matches.subcommand_name() == Some(SUBCOMMAND_NAME)
    }

    async fn execute(&self, matches: &ArgMatches, out: &mut T) -> Result<()> {
        let user = user::load_user_from_config(out).await?;
        let client = HttpClient::from_user(&user);

        execute(matches, out, client).await
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

async fn execute<T: Write + Send>(
    matches: &ArgMatches,
    out: &mut T,
    client: HttpClient,
) -> Result<()> {
    if matches.is_present("dato") {
        let date = matches.value_of("dato").unwrap().parse()?;

        let mut timestamps = client.get_timestamps_for_date(date).await?;
        timestamps.sort_by(|t0, t1| t0.project_id.cmp(&t1.project_id));

        let mut table_maker = print::TableMaker::new();

        table_maker.titles(vec![
            "PROSJEKT".to_string(),
            TimestampDate(&date).to_string(),
        ]);

        table_maker.with(Box::new(|pt: &ProjectTimestamp| pt.project_id.clone()));
        table_maker.with(Box::new(move |pt| {
            TimestampHours(&pt.timestamp.time).to_string()
        }));

        table_maker.into_table(timestamps.as_slice()).print(out)?;
    } else {
        let from = if let Some(from) = matches.value_of("fra") {
            from.parse::<NaiveDate>()?
        } else {
            let base_date = if matches.is_present("forrige-uke") {
                Utc::now().date() - Duration::weeks(1)
            } else if matches.is_present("neste_uke") {
                Utc::now().date() + Duration::weeks(1)
            } else {
                // default to monday this week
                Utc::now().date()
            };
            let days_from_monday = base_date.weekday().num_days_from_monday() as i64;

            base_date.naive_local() - Duration::days(days_from_monday)
        };

        let to = if let Some(to) = matches.value_of("til") {
            to.parse::<NaiveDate>()?
        } else {
            let base_date = if matches.is_present("forrige-uke") {
                Utc::now().date() - Duration::weeks(1)
            } else if matches.is_present("neste_uke") {
                Utc::now().date() + Duration::weeks(1)
            } else {
                // default to monday this week
                Utc::now().date()
            };
            let days_from_monday = base_date.weekday().num_days_from_monday() as i64;

            base_date.naive_local() + Duration::days(6 - days_from_monday as i64)
        };

        // only one of these two can be true, if none are then we let the number of days in period decide
        let turn_table = matches.is_present("snu-tabell");
        let dont_turn_table = matches.is_present("ikke-snu-tabell");
        if turn_table || (!dont_turn_table && to - from > Duration::days(6)) {
            // auto transpose if more than one week
            let mut timestamps = client.get_timestamps_for_period(from, to).await?;
            timestamps.sort_by(|t0, t1| t0.timestamp.date.cmp(&t1.timestamp.date));

            let mut table_maker = print::TableMaker::new();
            table_maker.static_titles(vec!["DATO", "PROSJEKT", "TIMER"]);
            table_maker.with(Box::new(|pt: &ProjectTimestamp| {
                TimestampDate(&pt.timestamp.date).to_string()
            }));
            table_maker.with(Box::new(|pt: &ProjectTimestamp| pt.project_id.clone()));
            table_maker.with(Box::new(|pt: &ProjectTimestamp| {
                TimestampHours(&pt.timestamp.time).to_string()
            }));

            table_maker.into_table(timestamps.as_slice()).print(out)?;
        } else {
            let mut timestamps = get_timestamps_for_period(client, from, to).await?;
            timestamps.sort_by(|t0, t1| t0.project_id.cmp(&t1.project_id));
            let timestamped_dates: HashMap<NaiveDate, ()> = timestamps
                .iter()
                .flat_map(|pt| pt.timestamps.iter().map(|t| t.date))
                .map(|d| (d, ()))
                .collect();
            let mut skipped_days = vec![];

            let mut table_maker = print::TableMaker::new();

            let titles = from.iter_days().take_while(|d| d <= &to).fold(
                vec!["PROSJEKT".to_string()],
                |mut titles, next| {
                    // skip days in weekend if no timestamp
                    let weekday = next.weekday();
                    let is_weekend = weekday == Weekday::Sat || weekday == Weekday::Sun;
                    if !is_weekend || timestamped_dates.contains_key(&next) {
                        titles.push(TimestampDate(&next).to_string());
                    } else {
                        skipped_days.push(next);
                    }
                    titles
                },
            );
            table_maker.titles(titles);

            table_maker.with(Box::new(|pt: &ProjectTimestamps| pt.project_id.clone()));
            from.iter_days()
                .take_while(|d| d <= &to)
                .filter(|d| !skipped_days.contains(d))
                .for_each(|d| {
                    table_maker.with(Box::new(move |pt| {
                        pt.find_timestamp_for_date(&d)
                            .map(|ts| TimestampHours(&ts.time).to_string())
                            .unwrap_or_default()
                    }));
                });

            table_maker.into_table(timestamps.as_slice()).print(out)?;
        }
    }

    Ok(())
}

pub async fn get_timestamps_for_period(
    client: HttpClient,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<ProjectTimestamps>> {
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
