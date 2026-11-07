//! Who plays whom. Pure functions: clubs in, fixtures out, no `World`.
use crate::model::{ClubId, CompetitionId, Fixture};

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
                if r.is_multiple_of(2) { (i, m) } else { (m, i) } // the fixed club alternates
            } else if i < j {
                if (i + j).is_multiple_of(2) { (i, j) } else { (j, i) }
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

/// The rules that make a league schedule valid, whatever its exact order. Shared by the tests of
/// every module that generates league fixtures.
#[cfg(test)]
pub(crate) fn assert_valid<'a>(competition: CompetitionId, clubs: &[ClubId], fixtures: impl Iterator<Item = &'a Fixture>) {
    use std::collections::{BTreeMap, HashSet};

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
    let rounds = if n.is_multiple_of(2) { 2 * (n - 1) } else { 2 * n };
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
