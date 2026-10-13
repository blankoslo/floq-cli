use super::{TimestampDate, TimestampHours};
use crate::{http_client::FloqApiClient, print, session::FloqSessionHandler, time};

use std::{collections::HashMap, io::Write};

use anyhow::Result;
use chrono::{Datelike, Duration, NaiveDate, Utc, Weekday};
use clap::Args;

#[derive(Args)]
pub struct HistoryArgs {
    /// Dagen du ønsker å vise timer for.
    /// F.eks. "--dato 2021-03-01"
    #[arg(short, long, value_parser = time::parse_date, verbatim_doc_comment)]
    dato: Option<NaiveDate>,

    /// Første dagen å vise timer for, settes til mandag denne uken hvis utelatt.
    /// Er inklusiv. F.eks. "--fra 2021-03-01"
    #[arg(long, requires = "til", conflicts_with = "dato", value_parser = time::parse_date, verbatim_doc_comment)]
    fra: Option<NaiveDate>,

    /// Siste dagen å vise timer for, settes til fredag denne uken hvis utelatt.
    /// Er inklusiv. F.eks. "--til 2021-03-05"
    #[arg(long, requires = "fra", conflicts_with = "dato", value_parser = time::parse_date, verbatim_doc_comment)]
    til: Option<NaiveDate>,

    /// Vis timer ført i forrige uke
    #[arg(long, conflicts_with_all = ["dato", "fra", "til", "neste_uke"])]
    forrige_uke: bool,

    /// Vis timer ført for neste uke
    #[arg(long, conflicts_with_all = ["dato", "fra", "til", "forrige_uke"])]
    neste_uke: bool,

    /// Snu om på tabellen slik at rader går fra å være per prosjekt til per dag og prosjekt.
    /// Dette blir gjort automatisk hvis det skal vises timer for mer enn én uke.
    #[arg(long, conflicts_with_all = ["ikke_snu_tabell", "dato"], verbatim_doc_comment)]
    snu_tabell: bool,

    /// Ikke snu om på tabellen slik at rader går fra å være per prosjekt til per dag og prosjekt.
    /// Stopper det fra å bli gjort automatisk hvis det skal vises timer for mer enn én uke.
    #[arg(long, conflicts_with_all = ["snu_tabell", "dato"], verbatim_doc_comment)]
    ikke_snu_tabell: bool,
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

pub async fn execute<T: Write + Send>(
    args: HistoryArgs,
    out: &mut T,
    session_handler: &mut FloqSessionHandler,
) -> Result<()> {
    let session = session_handler.open(out, Duration::minutes(1)).await?;
    let client = FloqApiClient::from_session(session);

    let employee_id = client.get_logged_in_employee().await?.id;

    if let Some(date) = args.dato {
        let mut timestamps = client.get_timestamps_for_date(employee_id, date).await?;
        timestamps.sort_by(|t0, t1| t0.project_id.cmp(&t1.project_id));

        let mut table_maker = print::TableMaker::new();

        table_maker.titles(vec!["PROSJEKT".to_string(), TimestampDate(&date).to_string()]);

        table_maker.with(Box::new(|pt: &ProjectTimestamp| pt.project_id.clone()));
        table_maker.with(Box::new(move |pt| TimestampHours(&pt.timestamp.time).to_string()));

        table_maker.into_table(timestamps.as_slice()).print(out)?;
    } else {
        let from = if let Some(from) = args.fra {
            from
        } else {
            let base_date = if args.forrige_uke {
                Utc::now().date_naive() - Duration::weeks(1)
            } else if args.neste_uke {
                Utc::now().date_naive() + Duration::weeks(1)
            } else {
                // default to monday this week
                Utc::now().date_naive()
            };
            let days_from_monday = base_date.weekday().num_days_from_monday() as i64;

            base_date - Duration::days(days_from_monday)
        };

        let to = if let Some(to) = args.til {
            to
        } else {
            let base_date = if args.forrige_uke {
                Utc::now().date_naive() - Duration::weeks(1)
            } else if args.neste_uke {
                Utc::now().date_naive() + Duration::weeks(1)
            } else {
                // default to monday this week
                Utc::now().date_naive()
            };
            let days_from_monday = base_date.weekday().num_days_from_monday() as i64;

            base_date + Duration::days(6 - days_from_monday)
        };

        // only one of these two can be true, if none are then we let the number of days in period decide
        let turn_table = args.snu_tabell;
        let dont_turn_table = args.ikke_snu_tabell;
        if turn_table || (!dont_turn_table && to - from > Duration::days(6)) {
            // auto transpose if more than one week
            let mut timestamps = client.get_timestamps_for_period(employee_id, from, to).await?;
            timestamps.sort_by_key(|t| t.timestamp.date);

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
            let mut timestamps = client
                .get_sum_timestamps_by_project_for_period(employee_id, from, to)
                .await?;
            timestamps.sort_by(|t0, t1| t0.project_id.cmp(&t1.project_id));
            let timestamped_dates: HashMap<NaiveDate, ()> = timestamps
                .iter()
                .flat_map(|pt| pt.timestamps.iter().map(|t| t.date))
                .map(|d| (d, ()))
                .collect();
            let mut skipped_days = vec![];

            let mut table_maker = print::TableMaker::new();

            let titles =
                from.iter_days()
                    .take_while(|d| d <= &to)
                    .fold(vec!["PROSJEKT".to_string()], |mut titles, next| {
                        // skip days in weekend if no timestamp
                        let weekday = next.weekday();
                        let is_weekend = weekday == Weekday::Sat || weekday == Weekday::Sun;
                        if !is_weekend || timestamped_dates.contains_key(&next) {
                            titles.push(TimestampDate(&next).to_string());
                        } else {
                            skipped_days.push(next);
                        }
                        titles
                    });
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

impl FloqApiClient {
    pub async fn get_sum_timestamps_by_project_for_period(
        &self,
        employee_id: u16,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<ProjectTimestamps>> {
        let project_timestamps = self.get_timestamps_for_period(employee_id, from, to).await?;

        let project_to_timestamps: HashMap<String, ProjectTimestamps> =
            project_timestamps.into_iter().fold(HashMap::new(), |mut res, next| {
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

        Ok(project_to_timestamps.into_values().collect())
    }
}
