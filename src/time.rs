use chrono::Datelike;

#[derive(PartialEq)]
pub struct Weekday {
    pub full_name: &'static str,
    pub short_name: &'static str,
}

#[derive(PartialEq)]
pub enum Weekdays {
    Monday(Weekday),
    Tuesday(Weekday),
    Wednesday(Weekday),
    Thursday(Weekday),
    Friday(Weekday),
    Saturday(Weekday),
    Sunday(Weekday),
}

impl Weekday {
    fn new(full_name: &'static str, short_name: &'static str) -> Self {
        Weekday {
            full_name,
            short_name,
        }
    }
}

impl Weekdays {
    pub fn all() -> [Self; 7] {
        [
            Weekdays::monday(),
            Weekdays::tueday(),
            Weekdays::wednesday(),
            Weekdays::thurday(),
            Weekdays::friday(),
            Weekdays::saturday(),
            Weekdays::sunday(),
        ]
    }

    fn monday() -> Self {
        Weekdays::Monday(Weekday::new("mandag", "man"))
    }

    fn tueday() -> Self {
        Weekdays::Tuesday(Weekday::new("tirsdag", "tir"))
    }

    fn wednesday() -> Self {
        Weekdays::Wednesday(Weekday::new("onsdag", "ons"))
    }

    fn thurday() -> Self {
        Weekdays::Thursday(Weekday::new("torsdag", "tor"))
    }

    fn friday() -> Self {
        Weekdays::Friday(Weekday::new("fredag", "fre"))
    }

    fn saturday() -> Self {
        Weekdays::Saturday(Weekday::new("lørdag", "lør"))
    }

    fn sunday() -> Self {
        Weekdays::Sunday(Weekday::new("søndag", "søn"))
    }

    pub fn get_weekday(&self) -> &Weekday {
        match self {
            Self::Monday(w) => w,
            Self::Tuesday(w) => w,
            Self::Wednesday(w) => w,
            Self::Thursday(w) => w,
            Self::Friday(w) => w,
            Self::Saturday(w) => w,
            Self::Sunday(w) => w,
        }
    }

    pub fn as_chrono_weekday(&self) -> chrono::Weekday {
        match self {
            Self::Monday(_) => chrono::Weekday::Mon,
            Self::Tuesday(_) => chrono::Weekday::Tue,
            Self::Wednesday(_) => chrono::Weekday::Wed,
            Self::Thursday(_) => chrono::Weekday::Thu,
            Self::Friday(_) => chrono::Weekday::Fri,
            Self::Saturday(_) => chrono::Weekday::Sat,
            Self::Sunday(_) => chrono::Weekday::Sun,
        }
    }
}

impl<D: Datelike> From<&D> for Weekdays {
    fn from(dl: &D) -> Self {
        match dl.weekday() {
            chrono::Weekday::Mon => Weekdays::monday(),
            chrono::Weekday::Tue => Weekdays::tueday(),
            chrono::Weekday::Wed => Weekdays::wednesday(),
            chrono::Weekday::Thu => Weekdays::thurday(),
            chrono::Weekday::Fri => Weekdays::friday(),
            chrono::Weekday::Sat => Weekdays::saturday(),
            chrono::Weekday::Sun => Weekdays::sunday(),
        }
    }
}
