use crate::{http_client::FloqApiClient, session::FloqSessionHandler, time};

use std::{fmt::Display, io::Write};

use anyhow::{Result, anyhow};
use chrono::{Datelike, Duration, NaiveDate, Utc};
use clap::Args;
use futures::{StreamExt, stream::FuturesUnordered};

pub mod history;
mod http;

#[derive(Args)]
pub struct TimestampArgs {
    /// Prosjektet du ønsker å føre timer på
    prosjekt: String,

    /// Antall timer du ønsker å føre, settes til "7.5" hvis utelatt
    #[arg(short, long, default_value_t = 7.5, hide_default_value = true)]
    timer: f32,

    /// Dagen det skal føres timer på, settes til i dag hvis utelatt.
    /// F.eks. "--dato 2021-03-01"
    #[arg(short, long, value_parser = time::parse_date, verbatim_doc_comment)]
    dato: Option<NaiveDate>,

    /// Brukes samme med --til for å føre timer i en periode, er inklusiv.
    /// F.eks. "--fra 2021-03-01"
    #[arg(long, requires = "til", conflicts_with = "dato", value_parser = time::parse_date, verbatim_doc_comment)]
    fra: Option<NaiveDate>,

    /// Brukes samme med --fra for å føre timer i en periode, er inklusiv.
    /// F.eks. "--til 2021-03-05"
    #[arg(long, requires = "fra", conflicts_with = "dato", value_parser = time::parse_date, verbatim_doc_comment)]
    til: Option<NaiveDate>,

    /// Ukedag det skal føres timer på, relativt til dagens dato. Kan kombineres med andre ukedager
    #[arg(long, visible_alias = "man")]
    mandag: bool,

    /// Ukedag det skal føres timer på, relativt til dagens dato. Kan kombineres med andre ukedager
    #[arg(long, visible_alias = "tir")]
    tirsdag: bool,

    /// Ukedag det skal føres timer på, relativt til dagens dato. Kan kombineres med andre ukedager
    #[arg(long, visible_alias = "ons")]
    onsdag: bool,

    /// Ukedag det skal føres timer på, relativt til dagens dato. Kan kombineres med andre ukedager
    #[arg(long, visible_alias = "tor")]
    torsdag: bool,

    /// Ukedag det skal føres timer på, relativt til dagens dato. Kan kombineres med andre ukedager
    #[arg(long, visible_alias = "fre")]
    fredag: bool,

    /// Ukedag det skal føres timer på, relativt til dagens dato. Kan kombineres med andre ukedager
    #[arg(long, visible_alias = "lør")]
    lørdag: bool,

    /// Ukedag det skal føres timer på, relativt til dagens dato. Kan kombineres med andre ukedager
    #[arg(long, visible_alias = "søn")]
    søndag: bool,

    /// Setter relativ dato til forrige uke. Brukes sammen med ukedagene til å velge dager i forrige uke
    #[arg(long, conflicts_with_all = ["neste_uke", "dato", "fra", "til"])]
    forrige_uke: bool,

    /// Setter relativ dato til neste uke. Brukes sammen med ukedagene til å velge dager i neste uke
    #[arg(long, conflicts_with_all = ["forrige_uke", "dato", "fra", "til"])]
    neste_uke: bool,
}

impl TimestampArgs {
    fn selected_weekdays(&self) -> Vec<time::Weekdays> {
        // same order as `time::Weekdays::all()`
        let flags = [
            self.mandag,
            self.tirsdag,
            self.onsdag,
            self.torsdag,
            self.fredag,
            self.lørdag,
            self.søndag,
        ];

        time::Weekdays::all()
            .into_iter()
            .zip(flags)
            .filter_map(|(day, is_selected)| is_selected.then_some(day))
            .collect()
    }

    /// --dato, --fra/--til and the weekday flags are different ways of selecting days, so they can't be combined.
    /// Weekday conflicts are checked here, the rest are handled by clap.
    fn validate(&self) -> Result<()> {
        let weekdays: Vec<&str> = self
            .selected_weekdays()
            .iter()
            .map(|day| day.get_weekday().full_name)
            .collect();

        let Some(weekday) = weekdays.first() else {
            if self.forrige_uke || self.neste_uke {
                let flag = if self.forrige_uke { "forrige-uke" } else { "neste-uke" };
                return Err(anyhow!(
                    "--{flag} må brukes sammen med en ukedag, f.eks. \"--{flag} --mandag\""
                ));
            }
            return Ok(());
        };

        if self.dato.is_some() {
            return Err(anyhow!("--{weekday} kan ikke brukes sammen med --dato"));
        }
        if self.fra.is_some() || self.til.is_some() {
            return Err(anyhow!("--{weekday} kan ikke brukes sammen med --fra/--til"));
        }

        Ok(())
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
        let weekdays: time::Weekdays = self.0.into();

        write!(
            f,
            "{} ({})",
            self.0.format("%Y-%m-%d"),
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

pub async fn execute<T: Write + Send>(
    args: TimestampArgs,
    out: &mut T,
    session_handler: &mut FloqSessionHandler,
) -> Result<()> {
    args.validate()?;

    let session = session_handler.open(out, Duration::minutes(1)).await?;
    let client = FloqApiClient::from_session(session);
    let project_id = args.prosjekt.as_str();

    let hours = args.timer;
    let time = Duration::minutes((hours * 60.0) as i64);
    if time > Duration::days(1) {
        return Err(anyhow!("Det er ikke mulig å føre {} timer på én dag", hours));
    }

    let weekdays = args.selected_weekdays();
    let dates = if let (Some(from), Some(to)) = (args.fra, args.til) {
        from.iter_days().take_while(|d| d <= &to).collect()
    } else if let Some(date) = args.dato {
        vec![date]
    } else if !weekdays.is_empty() {
        let today = Utc::now().date_naive();
        let base_date = if args.forrige_uke {
            today - Duration::weeks(1)
        } else if args.neste_uke {
            today + Duration::weeks(1)
        } else {
            today
        };
        let monday = base_date - Duration::days(base_date.weekday().num_days_from_monday() as i64);

        weekdays
            .iter()
            .map(|weekday| monday + Duration::days(weekday.as_chrono_weekday().num_days_from_monday() as i64))
            .collect()
    } else {
        // default to todays date
        vec![Utc::now().date_naive()]
    };

    let employee_id = client.get_logged_in_employee().await?.id;

    let mut futures: FuturesUnordered<_> = dates
        .iter()
        .map(|date| client.set_timetsamp(employee_id, project_id, &time, date))
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

impl FloqApiClient {
    async fn set_timetsamp<'a>(
        &self,
        employee_id: u16,
        project_id: &'a str,
        time: &'a Duration,
        date: &'a NaiveDate,
    ) -> Result<SetTimestampResult<'a>> {
        let current_time = self
            .get_timestamp_on_project_for_date(employee_id, project_id, date)
            .await?;
        let time_diff = *time - current_time;

        if !time_diff.is_zero() {
            self.internal_set_timestamp(employee_id, project_id, date, *time)
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
}
