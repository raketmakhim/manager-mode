//! Season schedules, and the weekly limit on how many matches a club may play.
use crate::model::{ClubId, CompetitionId, CompetitionKind, Fixture, World, MAX_MATCHES_PER_WEEK};
use std::collections::BTreeMap;

/// Builds a league's double round-robin with the "circle method": one club stays put, the others rotate round it.
/// - First half: N-1 rounds (N even), every pair meets once. Second half: the same rounds with home and away swapped.
/// - Odd N: a dummy club is added and whoever draws it rests that round, so there are 2N rounds.
/// - Home and away alternate so a club never has more than 3 of the same in a row (2 for odd N).
/// - Round r is played in week r (one round a week), so a league uses only one of a club's 3 weekly matches.
///
/// No randomness: the same club list always gives the same schedule.
/// Fewer than 2 clubs: no fixtures. Round numbers start at 1 and restart for each competition.
pub fn round_robin(competition: CompetitionId, clubs: &[ClubId]) -> Vec<Fixture> {
    let n = clubs.len();
    assert!(n <= 128, "round numbers are u8, so at most 128 clubs");
    if n < 2 {
        return vec![];
    }
    let size = n + n % 2; // odd: index n is the dummy club
    let m = size - 1; // rounds per half; index m is the club that stays put
    let mut first = Vec::new(); // (round, home index, away index)
    for r in 0..m {
        for i in 0..m {
            let j = (r + m - i) % m; // club i meets the club j with i + j = r (mod m)
            let (home, away) = if i == j {
                if r % 2 == 0 { (i, m) } else { (m, i) } // the fixed club alternates
            } else if i < j {
                if (i + j) % 2 == 0 { (i, j) } else { (j, i) }
            } else {
                continue; // the pair was already made from the other side
            };
            if home < n && away < n {
                first.push((r, home, away)); // drops matches against the dummy
            }
        }
    }
    let second = first.iter().map(|&(r, home, away)| (r + m, away, home));
    let all = first.iter().copied().chain(second);
    all.map(|(r, home, away)| Fixture {
        competition,
        round: (r + 1) as u8,
        week: (r + 1) as u8,
        home: clubs[home],
        away: clubs[away],
        result: None,
    })
    .collect()
}

impl World {
    /// Replaces `fixtures` with a full season for every league. Cups are not built yet: each of their
    /// rounds can only be drawn after the previous one is played, so they will add fixtures during the season.
    pub fn generate_fixtures(&mut self) {
        self.fixtures = self
            .competitions
            .values()
            .filter(|c| c.kind == CompetitionKind::League)
            .flat_map(|c| round_robin(c.id, &c.clubs))
            .collect();
    }

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
    use crate::source::load_edition;
    use rusqlite::Connection;
    use std::collections::HashSet;

    /// The rules that make a league schedule valid, whatever its exact order.
    fn assert_valid<'a>(competition: CompetitionId, clubs: &[ClubId], fixtures: impl Iterator<Item = &'a Fixture>) {
        let n = clubs.len();
        let fixtures: Vec<&Fixture> = fixtures.collect();
        assert_eq!(fixtures.len(), n * (n - 1), "{n} clubs: wrong number of matches");

        // Every ordered pair (home, away) exactly once, and nobody plays themselves.
        let pairs: HashSet<(ClubId, ClubId)> = fixtures.iter().map(|f| (f.home, f.away)).collect();
        assert_eq!(pairs.len(), n * (n - 1), "{n} clubs: a pairing is repeated");
        assert!(fixtures.iter().all(|f| f.home != f.away));
        assert!(fixtures.iter().all(|f| clubs.contains(&f.home) && clubs.contains(&f.away)));
        assert!(fixtures.iter().all(|f| f.result.is_none()));
        assert!(fixtures.iter().all(|f| f.competition == competition && f.week == f.round));

        // Rounds are 1..=2(N-1) for even N and 1..=2N for odd N, and each holds the right number of matches.
        let rounds = if n % 2 == 0 { 2 * (n - 1) } else { 2 * n };
        let mut per_round: BTreeMap<u8, Vec<&Fixture>> = BTreeMap::new();
        for f in &fixtures {
            per_round.entry(f.round).or_default().push(f);
        }
        assert_eq!(per_round.keys().copied().collect::<Vec<_>>(), (1..=rounds as u8).collect::<Vec<_>>());
        for (round, fs) in &per_round {
            assert_eq!(fs.len(), n / 2, "{n} clubs, round {round}");
            // A club plays at most once per round.
            let playing: HashSet<ClubId> = fs.iter().flat_map(|f| [f.home, f.away]).collect();
            assert_eq!(playing.len(), 2 * fs.len(), "{n} clubs: a club plays twice in round {round}");
        }

        // Each club: N-1 home games, N-1 away games, and never a long streak of the same.
        for &club in clubs {
            let mut games: Vec<(u8, bool)> = fixtures
                .iter()
                .filter_map(|f| (f.home == club).then_some((f.round, true)).or((f.away == club).then_some((f.round, false))))
                .collect();
            games.sort();
            assert_eq!(games.iter().filter(|g| g.1).count(), n - 1);
            assert_eq!(games.iter().filter(|g| !g.1).count(), n - 1);
            let longest = games.chunk_by(|a, b| a.1 == b.1).map(|run| run.len()).max().unwrap();
            assert!(longest <= 3, "{n} clubs: club {club} has {longest} home or away games in a row");
        }
    }

    fn empty_world() -> World {
        World {
            edition: 0,
            year: 0,
            competitions: BTreeMap::new(),
            clubs: BTreeMap::new(),
            players: BTreeMap::new(),
            fixtures: vec![],
        }
    }

    /// A fixture for club 1 at home in `week`, of some made-up cup.
    fn cup_match(week: u8) -> Fixture {
        Fixture { competition: 900, round: 1, week, home: 1, away: 2, result: None }
    }

    #[test]
    fn every_league_size_gives_a_valid_schedule() {
        for n in 2..=30 {
            let clubs: Vec<ClubId> = (100..100 + n as u32).collect();
            assert_valid(7, &clubs, round_robin(7, &clubs).iter());
        }
    }

    #[test]
    fn a_league_with_fewer_than_two_clubs_has_no_fixtures() {
        assert!(round_robin(7, &[]).is_empty());
        assert!(round_robin(7, &[7]).is_empty());
    }

    #[test]
    fn same_clubs_give_the_same_schedule() {
        let clubs: Vec<ClubId> = (1..=20).collect();
        let a: Vec<_> = round_robin(7, &clubs).iter().map(|f| (f.round, f.home, f.away)).collect();
        let b: Vec<_> = round_robin(7, &clubs).iter().map(|f| (f.round, f.home, f.away)).collect();
        assert_eq!(a, b);
    }

    #[test]
    fn a_league_alone_never_breaks_the_weekly_limit() {
        let mut w = empty_world();
        w.fixtures = round_robin(7, &(1..=20).collect::<Vec<_>>());
        assert!(w.calendar_problems().is_empty());
        assert_eq!(w.matches_in_week(1, 5), 1);
    }

    #[test]
    fn four_matches_in_a_week_is_a_problem_and_three_is_fine() {
        let mut w = empty_world();
        w.fixtures = vec![cup_match(3), cup_match(3), cup_match(3)];
        assert!(w.calendar_problems().is_empty(), "3 in a week is allowed");
        w.fixtures.push(cup_match(3));
        let problems = w.calendar_problems();
        assert_eq!(problems.len(), 2, "clubs 1 and 2 both have 4: {problems:?}");
        assert!(problems[0].contains("4 matches in week 3"));
    }

    #[test]
    fn first_free_week_skips_weeks_that_are_full() {
        let mut w = empty_world();
        assert_eq!(w.first_free_week(&[1, 2], 3), Some(3)); // empty calendar: right away
        w.fixtures = vec![cup_match(3), cup_match(3)];
        assert_eq!(w.first_free_week(&[1, 2], 3), Some(3)); // 2 played, room for the 3rd
        w.fixtures.push(cup_match(3));
        assert_eq!(w.first_free_week(&[1, 2], 3), Some(4)); // full, so next week
        assert_eq!(w.first_free_week(&[5, 6], 3), Some(3)); // other clubs are unaffected
        // One busy club is enough to block the week for the whole round.
        assert_eq!(w.first_free_week(&[2, 5], 3), Some(4));
    }

    #[test]
    fn every_real_fifa_15_league_gets_a_valid_schedule_and_calendar() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../history.db");
        if !std::path::Path::new(path).exists() { return; } // needs `cargo run -p import` first
        let mut w = load_edition(&Connection::open(path).unwrap(), 15).unwrap();
        w.generate_fixtures();
        let mut total = 0;
        for c in w.competitions.values().filter(|c| c.clubs.len() >= 2) {
            assert_valid(c.id, &c.clubs, w.fixtures.iter().filter(|f| f.competition == c.id));
            total += c.clubs.len() * (c.clubs.len() - 1);
        }
        assert_eq!(w.fixtures.len(), total); // one-club leagues add nothing
        assert!(w.calendar_problems().is_empty());
    }
}
