//! When matches are played, and the limit on how many a club may play in one week.
use crate::model::{ClubId, Day, World, MAX_MATCHES_PER_WEEK};
use std::collections::BTreeMap;

/// The date a match is played, like "Wed 13 Aug 2014": the week's Saturday plus the day's offset.
/// Week 1 is the first Saturday on or after 9 August of `season_start_year`.
// shortcut: every match is on a Saturday and the season never pauses (no winter break, no international breaks);
// upgrade when fixtures carry real dates.
pub fn match_date(season_start_year: u16, week: u8, day: Day) -> String {
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let first_day = days_from_civil(season_start_year as i64, 8, 9);
    let to_saturday = (5 - weekday(first_day)).rem_euclid(7); // Monday = 0
    let saturday = first_day + to_saturday + 7 * (week as i64 - 1);
    let (year, month, day_of_month) = civil_from_days(saturday + day.days_after_saturday());
    format!("{} {day_of_month} {} {year}", day.short_name(), MONTHS[month as usize - 1])
}

/// Day number counted from 1970-01-01 (Howard Hinnant's civil-date algorithm).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The inverse of `days_from_civil`: (year, month 1-12, day 1-31).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let m = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * m + 2) / 5 + 1;
    let month = if m < 10 { m + 3 } else { m - 9 };
    (year_of_era + era * 400 + (month <= 2) as i64, month, day)
}

/// 0 = Monday ... 6 = Sunday. 1970-01-01 was a Thursday.
fn weekday(days: i64) -> i64 {
    (days + 3).rem_euclid(7)
}

impl World {
    /// How many matches the club plays in that week (1 = first week of the season).
    // shortcut: scans every fixture on each call; index by (club, week) if scheduling gets slow
    pub fn matches_in_week(&self, club: ClubId, week: u8) -> usize {
        self.fixtures.iter().filter(|f| f.week == week && f.involves(club)).count()
    }

    /// The earliest week with a match still to play, or `None` when everything is played.
    pub fn next_unplayed_week(&self) -> Option<u8> {
        self.fixtures.iter().filter(|f| f.result.is_none()).map(|f| f.week).min()
    }

    /// First week from `from_week` on where every one of these clubs can still play one more match
    /// without passing `MAX_MATCHES_PER_WEEK`. `None` if no week up to 255 has room. A cup uses this to place its rounds.
    pub fn first_free_week(&self, clubs: &[ClubId], from_week: u8) -> Option<u8> {
        (from_week..=u8::MAX).find(|&w| clubs.iter().all(|&c| self.matches_in_week(c, w) < MAX_MATCHES_PER_WEEK))
    }

    /// Every club and week that has too many matches, as readable text. Empty means the calendar is fine.
    pub fn calendar_problems(&self) -> Vec<String> {
        let mut counts: BTreeMap<(ClubId, u8), usize> = BTreeMap::new();
        for f in &self.fixtures {
            for club in [f.home, f.away] {
                *counts.entry((club, f.week)).or_default() += 1;
            }
        }
        counts
            .into_iter()
            .filter(|&(_, n)| n > MAX_MATCHES_PER_WEEK)
            .map(|((club, week), n)| {
                let name = self.clubs.get(&club).map_or("?", |c| c.name.as_str());
                format!("{name} plays {n} matches in week {week} (max {MAX_MATCHES_PER_WEEK})")
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::competition::round_robin;
    use crate::model::{Day, Fixture};

    /// A fixture for club 1 at home in `week`, of some made-up cup.
    fn cup_match(week: u8) -> Fixture {
        Fixture { competition: 900, round: 1, week, day: Day::Wednesday, home: 1, away: 2, result: None }
    }

    #[test]
    fn weeks_are_saturdays_a_week_apart_across_the_new_year() {
        assert_eq!(match_date(2014, 1, Day::Saturday), "Sat 9 Aug 2014"); // 9 August 2014 was a Saturday
        assert_eq!(match_date(2014, 1, Day::Wednesday), "Wed 13 Aug 2014");
        assert_eq!(match_date(2014, 2, Day::Saturday), "Sat 16 Aug 2014");
        assert_eq!(match_date(2014, 25, Day::Saturday), "Sat 24 Jan 2015");
        assert_eq!(match_date(2023, 1, Day::Saturday), "Sat 12 Aug 2023"); // 9 August 2023 was a Wednesday
        assert_eq!(match_date(2015, 30, Day::Saturday), "Sat 5 Mar 2016"); // 2016 is a leap year: Feb has 29 days
    }

    #[test]
    fn a_league_alone_never_breaks_the_weekly_limit() {
        let mut w = World::new(0, 0);
        w.fixtures = round_robin(7, &(1..=20).collect::<Vec<_>>());
        assert!(w.calendar_problems().is_empty());
        assert_eq!(w.matches_in_week(1, 5), 1);
    }

    #[test]
    fn four_matches_in_a_week_is_a_problem_and_three_is_fine() {
        let mut w = World::new(0, 0);
        w.fixtures = vec![cup_match(3), cup_match(3), cup_match(3)];
        assert!(w.calendar_problems().is_empty(), "3 in a week is allowed");
        w.fixtures.push(cup_match(3));
        let problems = w.calendar_problems();
        assert_eq!(problems.len(), 2, "clubs 1 and 2 both have 4: {problems:?}");
        assert!(problems[0].contains("4 matches in week 3"));
    }

    #[test]
    fn first_free_week_skips_weeks_that_are_full() {
        let mut w = World::new(0, 0);
        assert_eq!(w.first_free_week(&[1, 2], 3), Some(3)); // empty calendar: right away
        w.fixtures = vec![cup_match(3), cup_match(3)];
        assert_eq!(w.first_free_week(&[1, 2], 3), Some(3)); // 2 played, room for the 3rd
        w.fixtures.push(cup_match(3));
        assert_eq!(w.first_free_week(&[1, 2], 3), Some(4)); // full, so next week
        assert_eq!(w.first_free_week(&[5, 6], 3), Some(3)); // other clubs are unaffected
        // One busy club is enough to block the week for the whole round.
        assert_eq!(w.first_free_week(&[2, 5], 3), Some(4));
    }
}
