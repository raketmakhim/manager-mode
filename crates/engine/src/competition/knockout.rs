//! Single-leg knockout cups: draw a round, play it, draw the next from the winners.
use super::{Competition, Entry, Format, Rules, Stage, DEFAULT_MODEL, DEFAULT_SUBSTITUTIONS};
use crate::model::{ClubId, CompetitionId, Day, Fixture, World};
use crate::rng::Rng;
use std::collections::{BTreeMap, HashSet};

/// The season week of a cup's first round: after the league has started, before the first cup clash with it.
const CUP_FIRST_WEEK: u8 = 3;
/// Weeks between one cup round and the next, so the rounds are spread through the season.
const CUP_WEEKS_BETWEEN_ROUNDS: u8 = 4;
/// Domestic cups are played midweek.
const CUP_DAY: Day = Day::Wednesday;

impl Rules {
    /// One single-leg knockout of every club. The default for any cup we have not researched.
    pub fn default_cup() -> Rules {
        Rules { stages: vec![Stage { format: Format::Knockout, entry: Entry::AllClubs }], relegated: 0, model: DEFAULT_MODEL, substitutions: DEFAULT_SUBSTITUTIONS }
    }
}

/// Draws one round. Returns the matches as (home, away). If the clubs are not a power of two, the extra
/// clubs play a preliminary round and everyone else gets a bye, so the following rounds are even.
/// Fewer than 2 clubs: no matches.
pub fn draw_round(clubs: &[ClubId], rng: &mut Rng) -> Vec<(ClubId, ClubId)> {
    if clubs.len() < 2 {
        return vec![];
    }
    let mut order = clubs.to_vec();
    rng.shuffle(&mut order);
    let playing = 2 * (clubs.len() - clubs.len().next_power_of_two() / 2); // 8 clubs: 8, 5 clubs: 2
    order[..playing].chunks(2).map(|pair| (pair[0], pair[1])).collect()
}

/// The clubs of a cup that have not lost yet.
fn clubs_left(clubs: &[ClubId], played: &[&Fixture]) -> Vec<ClubId> {
    let losers: HashSet<ClubId> = played
        .iter()
        .filter(|f| f.result.is_some())
        .map(|f| f.loser().unwrap_or_else(|| panic!("cup tie {} v {} (round {}) ended level: the simulation must decide cup ties", f.home, f.away, f.round)))
        .collect();
    clubs.iter().copied().filter(|c| !losers.contains(c)).collect()
}

impl World {
    /// A cup for every country, open to all its clubs, with the default rules. Cup ids come after every league id.
    /// A game-setup step, not part of loading real data. Safe to call twice: a country that has a cup keeps it.
    pub fn add_default_cups(&mut self) {
        let mut by_country: BTreeMap<String, Vec<ClubId>> = BTreeMap::new();
        for league in self.competitions.values().filter(|c| c.is_league()) {
            by_country.entry(league.country.clone()).or_default().extend(&league.clubs);
        }
        let first_id = self.competitions.keys().max().map_or(1, |id| id + 1);
        let has_cup = |country: &str| self.competitions.values().any(|c| c.is_cup() && c.country == country);
        let countries: Vec<_> = by_country.into_iter().filter(|(country, clubs)| clubs.len() >= 2 && !has_cup(country)).collect();
        for (id, (country, clubs)) in (first_id..).zip(countries) {
            let name = format!("{country} Cup");
            self.competitions.insert(id, Competition { id, name, country, rules: Rules::default_cup(), level: None, clubs });
        }
    }

    /// Draws the next round of every cup whose current round is fully played (the first round if none is drawn yet).
    /// Call it again after each batch of results. Every played cup match must have a winner.
    // shortcut: every cup plays round 1 in week 3 and then one round every 4 weeks, on a Wednesday, whatever the
    // country; real cups have fixed dates. Upgrade when cup dates become data in `Rules`.
    pub fn advance_cups(&mut self, rng: &mut Rng) {
        let cups: Vec<(CompetitionId, Vec<ClubId>)> =
            self.competitions.values().filter(|c| c.is_cup()).map(|c| (c.id, c.clubs.clone())).collect();
        for (id, clubs) in cups {
            let own: Vec<&Fixture> = self.fixtures.iter().filter(|f| f.competition == id).collect();
            let last_round = own.iter().map(|f| f.round).max().unwrap_or(0);
            if own.iter().any(|f| f.result.is_none()) {
                continue; // the round is still being played
            }
            let matches = draw_round(&clubs_left(&clubs, &own), rng);
            let playing: Vec<ClubId> = matches.iter().flat_map(|&(home, away)| [home, away]).collect();
            let round = last_round + 1;
            let first_choice = CUP_FIRST_WEEK.saturating_add((round - 1).saturating_mul(CUP_WEEKS_BETWEEN_ROUNDS));
            let Some(week) = self.first_free_week(&playing, first_choice) else { continue };
            self.fixtures.extend(matches.into_iter().map(|(home, away)| Fixture {
                competition: id,
                round,
                week,
                day: CUP_DAY,
                home,
                away,
                result: None,
            }));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::outcome::Outcome;

    fn world_with_cup(n: u32) -> World {
        let mut w = World::new(0, 0);
        let clubs = (1..=n).collect();
        w.competitions.insert(1, Competition { id: 1, name: "Cup".into(), country: "X".into(), rules: Rules::default_cup(), level: None, clubs });
        w
    }

    /// Plays every drawn match (the home side wins) and draws the next round, until the cup is over.
    /// Returns the number of matches in each round.
    fn play_out(w: &mut World, rng: &mut Rng) -> Vec<usize> {
        let mut rounds = vec![];
        loop {
            let before = w.fixtures.len();
            w.advance_cups(rng);
            if w.fixtures.len() == before {
                return rounds;
            }
            rounds.push(w.fixtures.len() - before);
            w.fixtures.iter_mut().for_each(|f| f.result = Some(Outcome::full_time(1, 0)));
        }
    }

    #[test]
    fn a_draw_pairs_each_club_at_most_once_and_leaves_an_even_field() {
        for n in 2..=70u32 {
            let clubs: Vec<ClubId> = (1..=n).collect();
            let matches = draw_round(&clubs, &mut Rng::new(n as u64));
            let playing: HashSet<ClubId> = matches.iter().flat_map(|&(h, a)| [h, a]).collect();
            assert_eq!(playing.len(), 2 * matches.len(), "{n} clubs: a club is drawn twice");
            let left = n as usize - matches.len(); // winners plus clubs on a bye
            assert!(left.is_power_of_two(), "{n} clubs leave {left}");
        }
    }

    #[test]
    fn a_cup_ends_with_one_winner_after_n_minus_1_matches() {
        for n in 2..=70 {
            let mut w = world_with_cup(n);
            let rounds = play_out(&mut w, &mut Rng::new(3));
            assert_eq!(w.fixtures.len(), n as usize - 1, "{n} clubs");
            assert_eq!(rounds.len(), (n as f64).log2().ceil() as usize, "{n} clubs: rounds");
            assert!(w.calendar_problems().is_empty());
        }
    }

    #[test]
    fn cup_rounds_are_spread_through_the_season_on_midweek_days() {
        let mut w = world_with_cup(32);
        play_out(&mut w, &mut Rng::new(3));
        let mut weeks: Vec<(u8, u8)> = w.fixtures.iter().map(|f| (f.round, f.week)).collect();
        weeks.dedup();
        assert_eq!(weeks, vec![(1, 3), (2, 7), (3, 11), (4, 15), (5, 19)]);
        assert!(w.fixtures.iter().all(|f| f.day == Day::Wednesday));
    }

    #[test]
    fn the_next_round_waits_until_the_current_one_is_played() {
        let mut w = world_with_cup(8);
        let mut rng = Rng::new(1);
        w.advance_cups(&mut rng);
        w.advance_cups(&mut rng);
        assert_eq!(w.fixtures.len(), 4, "round 2 must not be drawn before round 1 is played");
    }

    #[test]
    fn the_same_seed_gives_the_same_cup() {
        let run = |seed| {
            let mut w = world_with_cup(20);
            play_out(&mut w, &mut Rng::new(seed));
            w.fixtures.iter().map(|f| (f.round, f.home, f.away)).collect::<Vec<_>>()
        };
        assert_eq!(run(5), run(5));
        assert_ne!(run(5), run(6));
    }

    #[test]
    fn a_default_cup_is_added_for_each_country() {
        let mut w = World::new(0, 0);
        let league = |id, country: &str, clubs: Vec<ClubId>| Competition {
            id,
            name: "L".into(),
            country: country.into(),
            rules: Rules::double_round_robin(),
            level: None,
            clubs,
        };
        w.competitions.insert(4, league(4, "England", vec![1, 2]));
        w.competitions.insert(9, league(9, "England", vec![3, 4]));
        w.competitions.insert(5, league(5, "Malta", vec![5])); // one club: no cup
        w.add_default_cups();
        let cups: Vec<_> = w.competitions.values().filter(|c| c.is_cup()).collect();
        assert_eq!(cups.len(), 1);
        assert_eq!((cups[0].id, cups[0].name.as_str(), cups[0].clubs.len()), (10, "England Cup", 4));
        w.add_default_cups();
        assert_eq!(w.competitions.values().filter(|c| c.is_cup()).count(), 1, "a second call must add nothing");
    }

    #[test]
    #[should_panic(expected = "ended level")]
    fn a_level_cup_tie_stops_the_simulation_with_a_clear_message() {
        let mut w = world_with_cup(4);
        let mut rng = Rng::new(1);
        w.advance_cups(&mut rng);
        w.fixtures.iter_mut().for_each(|f| f.result = Some(Outcome::full_time(1, 1)));
        w.advance_cups(&mut rng);
    }
}
