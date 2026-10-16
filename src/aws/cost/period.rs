use std::{
    fmt,
    time::{SystemTime, UNIX_EPOCH},
};

const SECONDS_PER_DAY: u64 = 24 * 60 * 60;

/// A UTC calendar date. Cost Explorer reads and returns dates in UTC.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Date {
    year: i64,
    month: u32,
    day: u32,
}

impl Date {
    pub(super) fn today() -> Self {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs());
        Self::from_unix_days((seconds / SECONDS_PER_DAY) as i64)
    }

    #[cfg(test)]
    pub(super) fn new(year: i64, month: u32, day: u32) -> Self {
        Self { year, month, day }
    }

    pub(super) fn day(self) -> u32 {
        self.day
    }

    pub(super) fn add_days(self, days: i64) -> Self {
        Self::from_unix_days(self.unix_days() + days)
    }

    pub(super) fn month_start(self) -> Self {
        Self { day: 1, ..self }
    }

    pub(super) fn previous_month_start(self) -> Self {
        self.month_start().add_days(-1).month_start()
    }

    pub(super) fn next_month_start(self) -> Self {
        // Day 28 + 4 days is always in the next month.
        Self { day: 28, ..self }.add_days(4).month_start()
    }

    // Howard Hinnant's days-from-civil algorithm for the proleptic Gregorian calendar.
    fn unix_days(self) -> i64 {
        let year = if self.month <= 2 {
            self.year - 1
        } else {
            self.year
        };
        let era = year.div_euclid(400);
        let year_of_era = year - era * 400;
        let month = i64::from(self.month);
        let day_of_year =
            (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(self.day) - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        era * 146_097 + day_of_era - 719_468
    }

    fn from_unix_days(days: i64) -> Self {
        let days = days + 719_468;
        let era = days.div_euclid(146_097);
        let day_of_era = days - era * 146_097;
        let year_of_era =
            (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let shifted_month = (5 * day_of_year + 2) / 153;
        let day = (day_of_year - (153 * shifted_month + 2) / 5 + 1) as u32;
        let month = if shifted_month < 10 {
            shifted_month + 3
        } else {
            shifted_month - 9
        } as u32;
        let year = year_of_era + era * 400 + i64::from(month <= 2);
        Self { year, month, day }
    }
}

impl fmt::Display for Date {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:04}-{:02}-{:02}",
            self.year, self.month, self.day
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn month_boundaries_cross_years_and_short_months() {
        let new_year = Date::new(2026, 1, 1);
        assert_eq!(new_year.previous_month_start().to_string(), "2025-12-01");
        assert_eq!(new_year.add_days(-1).to_string(), "2025-12-31");

        let month_end = Date::new(2026, 10, 31);
        assert_eq!(month_end.add_days(1).to_string(), "2026-11-01");
        assert_eq!(month_end.next_month_start().to_string(), "2026-11-01");
        assert_eq!(
            Date::new(2026, 12, 15).next_month_start().to_string(),
            "2027-01-01"
        );

        let leap = Date::new(2028, 2, 28);
        assert_eq!(leap.add_days(1).to_string(), "2028-02-29");
        assert_eq!(
            Date::new(2028, 3, 31).previous_month_start().to_string(),
            "2028-02-01"
        );
        assert_eq!(Date::from_unix_days(0).to_string(), "1970-01-01");
    }
}
