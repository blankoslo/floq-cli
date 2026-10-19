use crate::{
    http_client::FloqApiClient,
    session::FloqSessionHandler,
    timestamp::time::{DaySelection, DaySelectionArgs, Days, Hours, RelativeWeek},
};

use std::io::Write;

use anyhow::Result;
use chrono::{Datelike, Duration, Utc};
use clap::{Args, ValueEnum};
use futures::{StreamExt, stream::FuturesOrdered};

#[derive(Args)]
pub struct HistoryArgs {
    #[command(flatten)]
    dato: DaySelectionArgs,

    /// Overskriv layout på tabellen. Standard for ukesvisning er per prosjekt, mens for datoer er det per dag.
    #[arg(short, long, value_enum, verbatim_doc_comment)]
    layout: Option<TableLayout>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum TableLayout {
    /// Tabellen viser en rad per prosjekt, og en kolonne for hver dag.
    PerProsjekt,
    /// Tabellen viser en rad per dag, og en kolonne for hvert prosjekt.
    PerDag,
}

pub async fn execute<T: Write + Send>(
    args: HistoryArgs,
    out: &mut T,
    session_handler: &mut FloqSessionHandler,
) -> Result<()> {
    let today = Utc::now().date_naive();

    let (dates, layout, force_show_weekend_dates, show_total_per_project, week) = match args.dato.to_day_selection()? {
        DaySelection::None => {
            // No week or days provided. Default to showing the current week
            let week = RelativeWeek::Current.resolve(today);
            let days = Days::whole_week();

            let force_show_weekend_dates = false;
            let show_total_per_project = true;
            let dates = days
                .into_iter()
                .map(|weekday| (weekday.to_string(), week.resolve_weekday(weekday)))
                .collect::<Vec<_>>();
            let layout = args.layout.unwrap_or(TableLayout::PerProsjekt);

            (
                dates,
                layout,
                force_show_weekend_dates,
                show_total_per_project,
                Some(week),
            )
        }
        DaySelection::Weekdays { week, days } => {
            let week = week.resolve(today);
            let (force_show_weekend_dates, days) = match days {
                Some(days) => (
                    // Always show weekend dates if weekend days were explicitly selected
                    days.has_weekend(),
                    days,
                ),
                None => (false, Days::whole_week()),
            };

            let show_total_per_project = true;
            let dates = days
                .into_iter()
                .map(|weekday| (weekday.to_string(), week.resolve_weekday(weekday)))
                .collect::<Vec<_>>();
            let layout = args.layout.unwrap_or(TableLayout::PerProsjekt);

            (
                dates,
                layout,
                force_show_weekend_dates,
                show_total_per_project,
                Some(week),
            )
        }
        DaySelection::Dates(days) => {
            let force_show_weekend_dates = false;
            // Showing total is not really necessary for single dates
            let show_total_per_project = days.is_range();
            let dates = days
                .into_iter()
                .map(|date| (date.format("%Y-%m-%d").to_string(), date))
                .collect::<Vec<_>>();
            let layout = args.layout.unwrap_or(TableLayout::PerDag);

            (dates, layout, force_show_weekend_dates, show_total_per_project, None)
        }
    };

    let session = session_handler.open(out, Duration::minutes(1)).await?;
    let client = FloqApiClient::from_session(session);

    let employee_id = client.get_logged_in_employee().await?.id;

    struct DataEntry {
        project_id: String,
        time: Duration,
    }

    let data: Vec<(String, Vec<DataEntry>)> = {
        let mut futures: FuturesOrdered<_> = dates
            .into_iter()
            .map(async |(label, date)| (label, date, client.get_timestamps_for_date(employee_id, date).await))
            .collect();

        let mut data = Vec::new();
        while let Some((label, date, timestamps_for_date)) = futures.next().await {
            let entries = timestamps_for_date?
                .into_iter()
                .map(|t| DataEntry {
                    project_id: t.id,
                    time: Duration::minutes(t.minutes),
                })
                .filter(|e| e.time != Duration::zero())
                .collect::<Vec<_>>();

            if !matches!(date.weekday(), chrono::Weekday::Sat | chrono::Weekday::Sun)
                || force_show_weekend_dates
                || !entries.is_empty()
            {
                data.push((label, entries));
            }
        }
        data
    };

    let projects = {
        let mut projects = Vec::new();
        for (_, entries) in &data {
            for entry in entries {
                if !projects.contains(&entry.project_id.as_str()) {
                    projects.push(entry.project_id.as_str());
                }
            }
        }
        projects.sort();
        projects
    };

    // Show total per day if there are multiple projects
    // Alternatively, if there is no data (in which case projects.len() == 0), show the total to highlight the absence of project-specific entries
    let show_total_per_date = projects.len() != 1;
    // header + projects + total (if show_total_per_date)
    let project_axis_len = 1 + projects.len() + if show_total_per_date { 1 } else { 0 };
    // header + data rows + total (if show_total_per_project)
    let date_axis_len = 1 + data.len() + if show_total_per_project { 1 } else { 0 };

    if let Some(week) = &week {
        writeln!(out, "Viser timer for uke {}", week.week_number())?;
    }

    match layout {
        TableLayout::PerDag => {
            let mut rows = Vec::<prettytable::Row>::with_capacity(date_axis_len);
            rows.push({
                let mut cells = Vec::<prettytable::Cell>::with_capacity(project_axis_len);
                cells.push(prettytable::cell!("Dato"));
                for project in &projects {
                    cells.push(prettytable::cell!(project));
                }
                if show_total_per_date {
                    cells.push(prettytable::cell!("Total"));
                }
                prettytable::Row::new(cells)
            });

            for (label, entries) in &data {
                rows.push({
                    let mut cells = Vec::<prettytable::Cell>::with_capacity(project_axis_len);
                    cells.push(prettytable::cell!(label));
                    for project in &projects {
                        let time = entries
                            .iter()
                            .find(|e| e.project_id == *project)
                            .map(|e| e.time)
                            .unwrap_or(Duration::zero());
                        if time != Duration::zero() {
                            cells.push(prettytable::cell!(r->Hours(time)))
                        } else {
                            cells.push(prettytable::cell!());
                        }
                    }
                    if show_total_per_date {
                        let total_time = entries.iter().map(|e| e.time).sum::<Duration>();
                        cells.push(prettytable::cell!(r->Hours(total_time)));
                    }
                    prettytable::Row::new(cells)
                });
            }

            if show_total_per_project {
                rows.push({
                    let mut cells = Vec::<prettytable::Cell>::with_capacity(project_axis_len);
                    cells.push(prettytable::cell!("TOTALT"));
                    for project in &projects {
                        let total_time = data
                            .iter()
                            .flat_map(|(_, entries)| {
                                entries.iter().filter(|e| e.project_id == *project).map(|e| e.time)
                            })
                            .sum::<Duration>();
                        cells.push(prettytable::cell!(r->Hours(total_time)));
                    }
                    if show_total_per_date {
                        let total_time = data
                            .iter()
                            .flat_map(|(_, entries)| entries.iter().map(|e| e.time))
                            .sum::<Duration>();
                        cells.push(prettytable::cell!(r->Hours(total_time)));
                    }
                    prettytable::Row::new(cells)
                });
            }

            let table = prettytable::Table::init(rows);
            table.print(out)?;
        }
        TableLayout::PerProsjekt => {
            let mut rows = Vec::<prettytable::Row>::with_capacity(project_axis_len);
            rows.push({
                let mut cells = Vec::<prettytable::Cell>::with_capacity(date_axis_len);
                cells.push(prettytable::cell!("Prosjekt"));
                for (label, _) in &data {
                    cells.push(prettytable::cell!(label));
                }
                if show_total_per_project {
                    cells.push(prettytable::cell!("Total"));
                }
                prettytable::Row::new(cells)
            });

            for project in &projects {
                rows.push({
                    let mut cells = Vec::<prettytable::Cell>::with_capacity(date_axis_len);
                    cells.push(prettytable::Cell::new(project));
                    for (_, entries) in &data {
                        let time = entries
                            .iter()
                            .find(|e| e.project_id == *project)
                            .map(|e| e.time)
                            .unwrap_or(Duration::zero());
                        if time != Duration::zero() {
                            cells.push(prettytable::cell!(r->Hours(time)));
                        } else {
                            cells.push(prettytable::cell!());
                        }
                    }
                    if show_total_per_project {
                        let total_time = data
                            .iter()
                            .flat_map(|(_, entries)| {
                                entries.iter().filter(|e| e.project_id == *project).map(|e| e.time)
                            })
                            .sum::<Duration>();
                        cells.push(prettytable::cell!(r->Hours(total_time)));
                    }
                    prettytable::Row::new(cells)
                });
            }

            if show_total_per_date {
                rows.push({
                    let mut cells = Vec::<prettytable::Cell>::with_capacity(date_axis_len);
                    cells.push(prettytable::cell!("Total"));
                    for (_, entries) in &data {
                        let total_time = entries.iter().map(|e| e.time).sum::<Duration>();
                        cells.push(prettytable::cell!(r->Hours(total_time)));
                    }
                    if show_total_per_project {
                        let total_time = data
                            .iter()
                            .flat_map(|(_, entries)| entries.iter().map(|e| e.time))
                            .sum::<Duration>();
                        cells.push(prettytable::cell!(r->Hours(total_time)));
                    }
                    prettytable::Row::new(cells)
                });
            }

            let table = prettytable::Table::init(rows);
            table.print(out)?;
        }
    };

    Ok(())
}
