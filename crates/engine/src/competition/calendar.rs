//! When matches are played, and the limit on how many a club may play in one week.
use crate::model::{ClubId, World, MAX_MATCHES_PER_WEEK};
use std::collections::BTreeMap;

impl World {
    /// How many matches the club plays in that week (1 = first week of the season).
    // shortcut: scans every fixture on each call; index by (club, week) if scheduling gets slow
    pub fn matches_in_week(&self, club: ClubId, week: u8) -> usize {
        self.fixtures.iter().filter(|f| f.week == week && (f.home == club || f.away == club)).count()
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
    use crate::model::Fixture;

    /// A fixture for club 1 at home in `week`, of some made-up cup.
    fn cup_match(week: u8) -> Fixture {
        Fixture { competition: 900, round: 1, week, home: 1, away: 2, result: None }
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
