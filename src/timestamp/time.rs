use std::{borrow::Borrow, fmt::Display, range::RangeInclusive, str::FromStr};

use anyhow::{Result, anyhow};
use chrono::{Datelike, NaiveDate};
use clap::{Args, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Weekday(chrono::Weekday);

impl Weekday {
    pub fn full_name(&self) -> &'static str {
        match self.0 {
            chrono::Weekday::Mon => "Mandag",
            chrono::Weekday::Tue => "Tirsdag",
            chrono::Weekday::Wed => "Onsdag",
            chrono::Weekday::Thu => "Torsdag",
            chrono::Weekday::Fri => "Fredag",
            chrono::Weekday::Sat => "Lørdag",
            chrono::Weekday::Sun => "Søndag",
        }
    }

    pub fn is_weekend(&self) -> bool {
        matches!(self.0, chrono::Weekday::Sat | chrono::Weekday::Sun)
    }
}

impl Display for Weekday {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.full_name())
    }
}

impl FromStr for Weekday {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mandag" | "man" => Ok(Weekday(chrono::Weekday::Mon)),
            "tirsdag" | "tir" => Ok(Weekday(chrono::Weekday::Tue)),
            "onsdag" | "ons" => Ok(Weekday(chrono::Weekday::Wed)),
            "torsdag" | "tor" => Ok(Weekday(chrono::Weekday::Thu)),
            "fredag" | "fre" => Ok(Weekday(chrono::Weekday::Fri)),
            "lørdag" | "lør" => Ok(Weekday(chrono::Weekday::Sat)),
            "søndag" | "søn" => Ok(Weekday(chrono::Weekday::Sun)),
            _ => Err(anyhow::anyhow!("Ugyldig ukedag: {}", s)),
        }
    }
}

impl Ord for Weekday {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.num_days_from_monday().cmp(&other.0.num_days_from_monday())
    }
}

impl PartialOrd for Weekday {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<D: Datelike> From<&D> for Weekday {
    fn from(dl: &D) -> Self {
        Weekday(dl.weekday())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayArg {
    Date(NaiveDate),
    Weekday(Weekday),
}

impl FromStr for DayArg {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(weekday) = Weekday::from_str(s) {
            Ok(DayArg::Weekday(weekday))
        } else if let Ok(date) = NaiveDate::from_str(s) {
            Ok(DayArg::Date(date))
        } else {
            Err(anyhow!(
                "Dag må være en ukedag eller en dato i formatet YYYY-MM-DD, f.eks. 2021-03-01"
            ))
        }
    }
}

impl Display for DayArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DayArg::Date(date) => write!(f, "{}", date.format("%Y-%m-%d")),
            DayArg::Weekday(weekday) => write!(f, "{}", weekday.full_name()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum WeekArg {
    /// Forrige uke
    Forrige = -1,
    /// Neste uke
    Neste = 1,
}

#[derive(Debug, Clone, Args)]
pub struct DaySelectionArgs {
    /// Dagen(e) du ønsker å føre/vise timer for. Kan være enten en ukedag eller en dato (YYYY-MM-DD). Kan settes flere ganger for å føre/vise timer for flere dager.
    /// F.eks. "--dag 2021-03-01" eller "--dag ons"
    #[arg(short, long, verbatim_doc_comment)]
    dag: Vec<DayArg>,

    /// Første dagen å føre/vise timer for. Kan være enten en ukedag eller en dato (YYYY-MM-DD).
    /// Er inklusiv. F.eks. "--fra 2021-03-01 --til 2021-03-05" eller "--fra man --til fre"
    #[arg(long, requires = "til", conflicts_with = "dag", verbatim_doc_comment)]
    fra: Option<DayArg>,

    /// Siste dagen å føre/vise timer for. Kan være enten en ukedag eller en dato (YYYY-MM-DD).
    /// Er inklusiv. F.eks. "--til 2021-03-05" eller "--fra man --til fre"
    #[arg(long, requires = "fra", conflicts_with = "dag", verbatim_doc_comment)]
    til: Option<DayArg>,

    /// Uke å føre/vise timer for. Brukes sammen med ukedager for å føre/vise timer i enten forrige uke eller neste uke.
    #[arg(short, long, value_enum, verbatim_doc_comment)]
    uke: Option<WeekArg>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Week(chrono::IsoWeek);

impl Week {
    pub fn resolve_weekday(&self, weekday: Weekday) -> chrono::NaiveDate {
        NaiveDate::from_isoywd_opt(self.0.year(), self.0.week(), weekday.0).unwrap()
    }

    pub fn week_number(&self) -> u32 {
        self.0.week()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelativeWeek {
    Previous = -1,
    Current = 0,
    Next = 1,
}

impl RelativeWeek {
    fn from_arg(week_arg: Option<WeekArg>) -> Self {
        match week_arg {
            Some(WeekArg::Forrige) => RelativeWeek::Previous,
            None => RelativeWeek::Current,
            Some(WeekArg::Neste) => RelativeWeek::Next,
        }
    }

    pub fn resolve(&self, today: NaiveDate) -> Week {
        Week((today + chrono::Duration::weeks(*self as i64)).iso_week())
    }
}

pub trait DateOps: Sized + Ord + Copy {
    fn succ_opt(&self) -> Option<Self>;

    fn signed_days_since(&self, other: Self) -> i64;
}

impl DateOps for Weekday {
    fn succ_opt(&self) -> Option<Self> {
        if self.0 == chrono::Weekday::Sun {
            None
        } else {
            Some(Weekday(self.0.succ()))
        }
    }

    fn signed_days_since(&self, other: Self) -> i64 {
        let self_days = self.0.num_days_from_monday() as i64;
        let other_days = other.0.num_days_from_monday() as i64;
        self_days - other_days
    }
}

impl DateOps for NaiveDate {
    fn succ_opt(&self) -> Option<Self> {
        NaiveDate::succ_opt(self)
    }

    fn signed_days_since(&self, other: Self) -> i64 {
        self.signed_duration_since(other).num_days()
    }
}

#[derive(Debug, Clone)]
pub enum Days<T> {
    Specific(Vec<T>),
    RangeInclusive(RangeInclusive<T>),
}

impl<T> Days<T> {
    pub fn is_range(&self) -> bool {
        match self {
            Days::Specific(_) => false,
            Days::RangeInclusive(_) => true,
        }
    }
}

impl Days<Weekday> {
    pub fn has_weekend(&self) -> bool {
        match self {
            Self::Specific(days) => days.iter().any(|d| d.is_weekend()),
            Self::RangeInclusive(range) => range.last.is_weekend(),
        }
    }

    pub fn whole_week() -> Self {
        Self::RangeInclusive(RangeInclusive {
            start: Weekday(chrono::Weekday::Mon),
            last: Weekday(chrono::Weekday::Sun),
        })
    }
}

pub struct InclusiveDateRangeIter<T: DateOps> {
    current: T,
    to: T,
    completed: bool,
}

impl<T: DateOps> Iterator for InclusiveDateRangeIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.completed {
            return None;
        }
        let current = self.current;
        if current == self.to {
            self.completed = true;
        } else {
            self.current = if let Some(next) = self.current.succ_opt() {
                next
            } else {
                self.completed = true;
                return None;
            };
        }
        Some(current)
    }
}

impl<T: DateOps> ExactSizeIterator for InclusiveDateRangeIter<T> {
    fn len(&self) -> usize {
        if self.completed {
            0
        } else {
            (self.to.signed_days_since(self.current) + 1).try_into().unwrap_or(0)
        }
    }
}

pub enum DateSelectionIntoIter<T: DateOps> {
    Specific(std::vec::IntoIter<T>),
    RangeInclusive(InclusiveDateRangeIter<T>),
}

impl<T: DateOps> Iterator for DateSelectionIntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            DateSelectionIntoIter::Specific(iter) => iter.next(),
            DateSelectionIntoIter::RangeInclusive(range) => range.next(),
        }
    }
}

impl<T: DateOps> ExactSizeIterator for DateSelectionIntoIter<T> {
    fn len(&self) -> usize {
        match self {
            DateSelectionIntoIter::Specific(iter) => iter.len(),
            DateSelectionIntoIter::RangeInclusive(range) => range.len(),
        }
    }
}

impl<T: DateOps> IntoIterator for Days<T> {
    type Item = T;

    type IntoIter = DateSelectionIntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Days::Specific(vec) => DateSelectionIntoIter::Specific(vec.into_iter()),
            Days::RangeInclusive(range) => DateSelectionIntoIter::RangeInclusive(InclusiveDateRangeIter {
                current: range.start,
                to: range.last,
                completed: false,
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DaySelection {
    /// No specific days, range or week specified.
    None,
    /// A week, days optionally specified.
    Weekdays {
        week: RelativeWeek,
        days: Option<Days<Weekday>>,
    },
    /// Specific naive dates.
    Dates(Days<NaiveDate>),
}

impl DaySelection {
    pub fn from_args(args: DaySelectionArgs) -> Result<DaySelection> {
        let mut dag = args.dag.into_iter();
        if let Some(first) = dag.next() {
            return Ok(match first {
                DayArg::Weekday(first) => {
                    let mut days = Vec::with_capacity(1 + dag.len());
                    days.push(first);
                    for d in dag {
                        if let DayArg::Weekday(d) = d {
                            if days.contains(&d) {
                                return Err(anyhow!("Hver --dag må være unik ({} forekommer flere ganger)", d))
                            }
                            days.push(d);
                        } else {
                            return Err(anyhow!("--dag må være alle ukedager eller alle datoer"));
                        }
                    }
                    Self::Weekdays {
                        week: RelativeWeek::from_arg(args.uke),
                        days: Some(Days::Specific(days)),
                    }
                }
                DayArg::Date(first) => {
                    let mut days = Vec::with_capacity(1 + dag.len());
                    days.push(first);
                    for d in dag {
                        if let DayArg::Date(d) = d {
                            if days.contains(&d) {
                                return Err(anyhow!("Hver --dag må være unik ({} forekommer flere ganger)", d.format("%Y-%m-%d")))
                            }
                            days.push(d);
                        } else {
                            return Err(anyhow!("--dag må være alle ukedager eller alle datoer"));
                        }
                    }
                    if args.uke.is_some() {
                        return Err(anyhow!("--uke kan kun brukes sammen med ukedager"));
                    }
                    Self::Dates(Days::Specific(days))
                }
            });
        }

        match (args.fra, args.til) {
            (Some(fra), Some(til)) => match (fra, til) {
                (DayArg::Weekday(fra), DayArg::Weekday(til)) => {
                    if fra > til {
                        return Err(anyhow!("--fra kan ikke være etter --til"));
                    }
                    Ok(Self::Weekdays {
                        week: RelativeWeek::from_arg(args.uke),
                        days: Some(Days::RangeInclusive(RangeInclusive { start: fra, last: til })),
                    })
                }
                (DayArg::Date(fra), DayArg::Date(til)) => {
                    if fra > til {
                        return Err(anyhow!("--fra kan ikke være etter --til"));
                    }
                    if args.uke.is_some() {
                        return Err(anyhow!("--uke kan kun brukes sammen med ukedager"));
                    }
                    Ok(Self::Dates(Days::RangeInclusive(RangeInclusive {
                        start: fra,
                        last: til,
                    })))
                }
                _ => Err(anyhow!(
                    "--fra og --til må enten begge være ukedager eller begge være datoer"
                )),
            },
            (None, None) => Ok(if let Some(uke) = args.uke {
                Self::Weekdays {
                    week: RelativeWeek::from_arg(Some(uke)),
                    days: None,
                }
            } else {
                Self::None
            }),
            _ => Err(anyhow!("--fra og --til må settes sammen")),
        }
    }
}

pub struct Hours<T: Borrow<chrono::Duration>>(pub T);

impl<T: Borrow<chrono::Duration>> Display for Hours<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}t",
            self.0.borrow().num_hours(),
            (*self.0.borrow() - chrono::Duration::hours(self.0.borrow().num_hours()))
                .num_minutes()
                .abs()
                / 6
        )
    }
}

pub struct HoursDiff<T: Borrow<chrono::Duration>>(pub T);

impl<T: Borrow<chrono::Duration>> Display for HoursDiff<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.borrow().num_minutes() == 0 {
            write!(f, "=")
        } else {
            write!(
                f,
                "{}{}.{}t",
                if self.0.borrow().num_minutes() > 0 { "+" } else { "-" },
                self.0.borrow().num_hours().abs(),
                (*self.0.borrow() - chrono::Duration::hours(self.0.borrow().num_hours()))
                    .num_minutes()
                    .abs()
                    / 6
            )
        }
    }
}
