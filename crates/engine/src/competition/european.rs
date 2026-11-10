//! The Champions League: how many places each country gets, who qualifies, and how it is played (a group stage, then
//! two-legged knockout rounds and a one-match final). Research and the sources of the ranking:
//! `research/game-features/european-places-by-season.md`.
use super::pairing::round_robin;
use super::standings::standings;
use super::{Competition, Entry, Format, Rules, Stage, DEFAULT_MODEL, DEFAULT_SUBSTITUTIONS};
use crate::model::{ClubId, Day, Fixture, World};
use crate::rng::Rng;
use crate::sim::team_strength;
use std::cmp::Reverse;
use std::collections::BTreeMap;

/// Clubs in a group.
const GROUP_SIZE: usize = 4;
/// Most groups; the real group stage has eight.
const MAX_GROUPS: usize = 8;
/// Matchdays in a group stage: every club meets the other three home and away. Later rounds are knockout rounds.
pub const CHAMPIONS_LEAGUE_GROUP_ROUNDS: u8 = 6;
/// Season week of the first group matchday, then one every two weeks. Even weeks, because the domestic cups use odd ones.
const GROUP_FIRST_WEEK: u8 = 6;
const WEEKS_BETWEEN_MATCHDAYS: u8 = 2;
/// Weeks between the last match of one stage and the first of the next, and between the two legs of a tie.
const WEEKS_BETWEEN_ROUNDS: u8 = 4;
const WEEKS_BETWEEN_LEGS: u8 = 2;
/// How many times a draw tries to keep group-mates apart before it accepts a repeat.
const DRAW_TRIES: usize = 50;
/// Russia was banned from European competition from this season on.
const RUSSIA_BANNED_FROM: u16 = 2022;

/// The first (best) 15 countries of UEFA's ranking for each season, by the year it starts in. Countries below the 15th
/// are not listed and get one Champions League place. `research/game-features/european-places-by-season.md` section 2.
const RANKINGS: &[(u16, [&str; 15])] = &[
    (2014, ["Spain", "England", "Germany", "Italy", "Portugal", "France", "Ukraine", "Russia", "Netherlands", "Turkey", "Belgium", "Greece", "Switzerland", "Cyprus", "Denmark"]),
    (2015, ["Spain", "England", "Germany", "Italy", "Portugal", "France", "Russia", "Netherlands", "Ukraine", "Belgium", "Turkey", "Greece", "Switzerland", "Austria", "Czech Republic"]),
    (2016, ["Spain", "England", "Germany", "Italy", "Portugal", "France", "Russia", "Ukraine", "Netherlands", "Belgium", "Switzerland", "Turkey", "Greece", "Czech Republic", "Romania"]),
    (2017, ["Spain", "Germany", "England", "Italy", "Portugal", "France", "Russia", "Ukraine", "Belgium", "Netherlands", "Turkey", "Switzerland", "Czech Republic", "Greece", "Romania"]),
    (2018, ["Spain", "Germany", "England", "Italy", "France", "Russia", "Portugal", "Ukraine", "Belgium", "Turkey", "Czech Republic", "Switzerland", "Netherlands", "Greece", "Austria"]),
    (2019, ["Spain", "England", "Italy", "Germany", "France", "Russia", "Portugal", "Ukraine", "Belgium", "Turkey", "Austria", "Switzerland", "Czech Republic", "Netherlands", "Greece"]),
    (2020, ["Spain", "England", "Italy", "Germany", "France", "Russia", "Portugal", "Belgium", "Ukraine", "Turkey", "Netherlands", "Austria", "Czech Republic", "Greece", "Croatia"]),
    (2021, ["Spain", "England", "Germany", "Italy", "France", "Portugal", "Russia", "Belgium", "Ukraine", "Netherlands", "Turkey", "Austria", "Denmark", "Scotland", "Czech Republic"]),
    (2022, ["England", "Spain", "Italy", "Germany", "France", "Portugal", "Netherlands", "Russia", "Belgium", "Austria", "Scotland", "Ukraine", "Turkey", "Denmark", "Cyprus"]),
    (2023, ["England", "Spain", "Italy", "Germany", "France", "Portugal", "Netherlands", "Austria", "Scotland", "Russia", "Serbia", "Ukraine", "Belgium", "Switzerland", "Greece"]),
];

/// Champions League places by ranking: (last rank in the band, places), for seasons up to 2017/18 and from 2018/19.
const BANDS_UNTIL_2017: [(usize, u8); 3] = [(3, 4), (6, 3), (15, 2)];
const BANDS_FROM_2018: [(usize, u8); 3] = [(4, 4), (6, 3), (15, 2)];
/// Places for a country outside the top 15.
const PLACES_OUTSIDE_TOP_15: u8 = 1;

/// How many Champions League places a country gets in the season that starts in `year`.
// shortcut: the extra place for the holders of the Champions League and Europa League is left out, and a year the
// research does not cover (before 2014, after 2023) uses the nearest ranking. Add them when European winners are kept.
pub fn champions_league_places(country: &str, year: u16) -> u8 {
    if country == "Russia" && year >= RUSSIA_BANNED_FROM {
        return 0;
    }
    let (_, ranking) = RANKINGS.iter().rev().find(|(from, _)| *from <= year).unwrap_or(&RANKINGS[0]);
    let bands = if year < 2018 { BANDS_UNTIL_2017 } else { BANDS_FROM_2018 };
    match ranking.iter().position(|c| *c == country) {
        Some(index) => bands.iter().find(|&&(last_rank, _)| index < last_rank).map_or(PLACES_OUTSIDE_TOP_15, |&(_, places)| places),
        None => PLACES_OUTSIDE_TOP_15,
    }
}

impl Rules {
    /// A group stage, then two-legged knockout rounds of the best two in each group.
    pub fn champions_league() -> Rules {
        Rules {
            stages: vec![Stage { format: Format::Groups, entry: Entry::AllClubs }, Stage { format: Format::TwoLeggedKnockout, entry: Entry::GroupQualifiers }],
            relegated: 0,
            model: DEFAULT_MODEL,
            substitutions: DEFAULT_SUBSTITUTIONS,
            first_week: GROUP_FIRST_WEEK,
            day: Day::Tuesday,
        }
    }
}

/// Groups for `entrants` clubs: as many as fit, a power of two so the knockout rounds come out even, at most `MAX_GROUPS`.
fn group_count(entrants: usize) -> usize {
    let fit = (entrants / GROUP_SIZE).min(MAX_GROUPS);
    if fit == 0 { 0 } else { 1 << fit.ilog2() }
}

/// The club that goes through a two-legged tie: more goals over both legs, and if they are level the winner of the
/// second leg (which went to extra time and penalties if it was level too).
// shortcut: no away goals rule (UEFA dropped it in 2021) and a second leg that is won in 90 minutes decides a level
// tie, where the real rule plays extra time. Upgrade by giving `MatchInput` the first leg's score.
fn tie_winner(first: &Fixture, second: &Fixture) -> ClubId {
    let ((first_home, first_away), (second_home, second_away)) = (first.result.expect("played").goals(), second.result.expect("played").goals());
    let (club_goals, other_goals) = (first_home + second_away, first_away + second_home);
    match club_goals.cmp(&other_goals) {
        std::cmp::Ordering::Greater => first.home,
        std::cmp::Ordering::Less => first.away,
        std::cmp::Ordering::Equal => second.winner().expect("a second leg has a winner"),
    }
}

/// The clubs that won their ties in the last knockout round, or `None` when that round was the final (or there is
/// no knockout round yet). `last` is the highest round played.
fn tie_winners(played: &[&Fixture], last: u8) -> Option<Vec<ClubId>> {
    let mut ties: BTreeMap<(ClubId, ClubId), Vec<&Fixture>> = BTreeMap::new();
    for f in played.iter().filter(|f| f.round + 1 >= last) {
        ties.entry((f.home.min(f.away), f.home.max(f.away))).or_default().push(f);
    }
    if ties.is_empty() || ties.values().any(|legs| legs.len() != 2) {
        return None;
    }
    Some(ties.into_values().map(|mut legs| {
        legs.sort_by_key(|f| f.round);
        tie_winner(legs[0], legs[1])
    }).collect())
}

/// Pairs each group winner with the runner-up of another group, as (runner-up, winner): the runner-up plays the first
/// leg at home. Tries to avoid group-mates meeting again; after `DRAW_TRIES` it takes what it has.
fn draw_knockout_start(winners: &[ClubId], runners_up: &[ClubId], rng: &mut Rng) -> Vec<(ClubId, ClubId)> {
    let mut order: Vec<usize> = (0..runners_up.len()).collect();
    for _ in 0..DRAW_TRIES {
        rng.shuffle(&mut order);
        if order.iter().enumerate().all(|(group, &other)| group != other) {
            break;
        }
    }
    order.iter().enumerate().map(|(group, &other)| (runners_up[other], winners[group])).collect()
}

impl Competition {
    /// True for the Champions League: a group stage and then two-legged knockout rounds.
    pub fn is_champions_league(&self) -> bool {
        matches!(self.rules.stages.as_slice(), [Stage { format: Format::Groups, .. }, Stage { format: Format::TwoLeggedKnockout, .. }])
    }

    /// The groups of a Champions League: its `clubs` hold them in order, `GROUP_SIZE` each.
    pub fn groups(&self) -> Vec<Vec<ClubId>> {
        self.clubs.chunks(GROUP_SIZE).map(|group| group.to_vec()).collect()
    }
}

impl World {
    /// Who qualifies for the season that starts next year, from the league tables of the season just played: the top
    /// places of each country's first division. Call it before promotion and relegation change the divisions.
    pub fn champions_league_entrants(&self) -> Vec<ClubId> {
        self.qualify(self.year + 1, |league| self.table(league.id).into_iter().map(|row| row.club).collect())
    }

    /// Who qualifies for the season that is starting when there are no tables yet (a new career): the strongest clubs of
    /// each country's first division.
    pub fn champions_league_entrants_by_strength(&self) -> Vec<ClubId> {
        self.qualify(self.year, |league| self.by_strength(&league.clubs))
    }

    /// The clubs best first by the strength of their best eleven.
    fn by_strength(&self, clubs: &[ClubId]) -> Vec<ClubId> {
        let mut ranked = clubs.to_vec();
        ranked.sort_by_key(|&club| (Reverse((team_strength(&self.lineup_ratings(club)) * 1000.0) as i64), club));
        ranked
    }

    /// For each country with places, the first `champions_league_places` clubs of its first division in the order given.
    fn qualify(&self, year: u16, order: impl Fn(&Competition) -> Vec<ClubId>) -> Vec<ClubId> {
        let mut top_flights: BTreeMap<&str, &Competition> = BTreeMap::new();
        for league in self.competitions.values().filter(|c| c.is_league() && c.level == Some(1)) {
            top_flights.entry(&league.country).or_insert(league);
        }
        top_flights.values().flat_map(|league| order(league).into_iter().take(champions_league_places(&league.country, year) as usize)).collect()
    }

    /// Puts `club` into the entrants in place of the weakest of them. Does nothing if it is already there.
    pub fn place_in_champions_league(&self, entrants: &mut Vec<ClubId>, club: ClubId) {
        if entrants.contains(&club) {
            return;
        }
        if let Some(weakest) = self.by_strength(entrants).pop() {
            entrants.retain(|&c| c != weakest);
        }
        entrants.push(club);
    }

    /// Starts the Champions League: the strongest `entrants` that fit into groups are seeded into pots by strength, drawn
    /// into groups and given their six matchdays. No competition is made if fewer than two groups fit.
    // shortcut: no qualifying rounds, and the groups ignore which country a club is from. Upgrade when the
    // qualifying rounds and the real seeding rules are added.
    pub fn start_champions_league(&mut self, entrants: &[ClubId], rng: &mut Rng) {
        let groups = group_count(entrants.len());
        if groups < 2 {
            return;
        }
        let ranked = self.by_strength(entrants);
        let mut pots: Vec<Vec<ClubId>> = ranked[..groups * GROUP_SIZE].chunks(groups).map(|pot| pot.to_vec()).collect();
        pots.iter_mut().for_each(|pot| rng.shuffle(pot));
        let clubs: Vec<ClubId> = (0..groups).flat_map(|g| pots.iter().map(move |pot| pot[g])).collect();
        let id = self.competitions.keys().max().map_or(1, |id| id + 1);
        for (g, group) in clubs.chunks(GROUP_SIZE).enumerate() {
            let day = if g % 2 == 0 { Day::Tuesday } else { Day::Wednesday };
            for mut f in round_robin(id, group) {
                f.week = GROUP_FIRST_WEEK + WEEKS_BETWEEN_MATCHDAYS * (f.round - 1);
                f.day = day;
                self.fixtures.push(f);
            }
        }
        let competition = Competition { id, name: "Champions League".into(), country: "Europe".into(), rules: Rules::champions_league(), level: None, clubs };
        self.competitions.insert(id, competition);
    }

    /// Draws the next knockout round of the Champions League once the stage before it is fully played: the best two of
    /// each group first, then the winners of the ties, and a single final for the last two. Call it after results.
    pub fn advance_champions_league(&mut self, rng: &mut Rng) {
        let Some(competition) = self.competitions.values().find(|c| c.is_champions_league()) else { return };
        let (id, groups) = (competition.id, competition.groups());
        let played: Vec<&Fixture> = self.fixtures.iter().filter(|f| f.competition == id).collect();
        if played.iter().any(|f| f.result.is_none()) {
            return;
        }
        let Some(last) = played.iter().map(|f| f.round).max() else { return };
        let (pairs, two_legged) = if last == CHAMPIONS_LEAGUE_GROUP_ROUNDS {
            let tables: Vec<_> = groups.iter().map(|group| standings(group, &played)).collect();
            let (winners, runners_up): (Vec<_>, Vec<_>) = tables.iter().map(|t| (t[0].club, t[1].club)).unzip();
            (draw_knockout_start(&winners, &runners_up, rng), true)
        } else {
            let Some(mut winners) = tie_winners(&played, last) else { return };
            rng.shuffle(&mut winners);
            let two_legged = winners.len() > 2;
            (winners.chunks(2).map(|pair| (pair[0], pair[1])).collect(), two_legged)
        };
        let latest_week = played.iter().map(|f| f.week).max().unwrap_or(GROUP_FIRST_WEEK);
        let playing: Vec<ClubId> = pairs.iter().flat_map(|&(home, away)| [home, away]).collect();
        let Some(first_week) = self.first_free_week(&playing, latest_week.saturating_add(WEEKS_BETWEEN_ROUNDS)) else { return };
        let fixture = |round: u8, week: u8, home: ClubId, away: ClubId, must_have_winner: bool| Fixture { competition: id, round, week, day: Day::Wednesday, home, away, result: None, must_have_winner };
        let mut new = vec![];
        for &(home, away) in &pairs {
            new.push(fixture(last + 1, first_week, home, away, !two_legged));
            if two_legged {
                let second_week = self.first_free_week(&playing, first_week + WEEKS_BETWEEN_LEGS).unwrap_or(first_week + WEEKS_BETWEEN_LEGS);
                new.push(fixture(last + 2, second_week, away, home, true));
            }
        }
        self.fixtures.extend(new);
    }

    /// Draws the next round of every domestic cup and of the Champions League.
    pub fn advance_competitions(&mut self, rng: &mut Rng) {
        self.advance_cups(rng);
        self.advance_champions_league(rng);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::outcome::Outcome;
    use crate::testing::add_club_with_eleven;

    /// A world with `clubs` clubs, club 1 the strongest, none in a league.
    fn world_with_clubs(clubs: u32) -> World {
        let mut w = World::new(0, 2014);
        for id in 1..=clubs {
            add_club_with_eleven(&mut w, id, 1000, 99 - id as u8);
        }
        w
    }

    fn played(round: u8, home: ClubId, away: ClubId, goals: (u8, u8)) -> Fixture {
        Fixture { competition: 1, round, week: round, day: Day::Wednesday, home, away, result: Some(Outcome::full_time(goals.0, goals.1)), must_have_winner: false }
    }

    /// Starts a Champions League of the first `clubs` clubs and plays it to the end, the home side winning every match
    /// 1-0. Returns the world and how many fixtures each call of `advance_champions_league` added.
    fn play_out(clubs: u32, seed: u64) -> (World, Vec<usize>) {
        let mut w = world_with_clubs(clubs);
        let mut rng = Rng::new(seed);
        w.start_champions_league(&(1..=clubs).collect::<Vec<_>>(), &mut rng);
        let mut added = vec![];
        loop {
            w.fixtures.iter_mut().filter(|f| f.result.is_none()).for_each(|f| f.result = Some(Outcome::full_time(1, 0)));
            let before = w.fixtures.len();
            w.advance_champions_league(&mut rng);
            if w.fixtures.len() == before {
                return (w, added);
            }
            added.push(w.fixtures.len() - before);
        }
    }

    #[test]
    fn a_country_gets_places_by_its_rank_that_season() {
        assert_eq!(champions_league_places("Spain", 2014), 4);
        assert_eq!(champions_league_places("Italy", 2014), 3, "rank 4 had three places until 2017/18");
        assert_eq!(champions_league_places("Italy", 2018), 4, "and four from 2018/19");
        assert_eq!(champions_league_places("Portugal", 2018), 2, "it fell to rank 7");
        assert_eq!(champions_league_places("Scotland", 2022), 2);
        assert_eq!(champions_league_places("Malta", 2022), 1, "outside the top 15");
    }

    #[test]
    fn russia_has_no_places_while_it_is_banned() {
        assert_eq!((champions_league_places("Russia", 2021), champions_league_places("Russia", 2022), champions_league_places("Russia", 2023)), (2, 0, 0));
    }

    #[test]
    fn a_year_outside_the_research_uses_the_nearest_ranking() {
        assert_eq!(champions_league_places("England", 2030), champions_league_places("England", 2023));
        assert_eq!(champions_league_places("Spain", 2005), champions_league_places("Spain", 2014));
    }

    #[test]
    fn the_groups_are_a_power_of_two_and_at_most_eight() {
        let groups: Vec<usize> = [0, 7, 8, 15, 16, 23, 24, 31, 32, 60].iter().map(|&n| group_count(n)).collect();
        assert_eq!(groups, [0, 1, 2, 2, 4, 4, 4, 4, 8, 8]);
    }

    #[test]
    fn the_strongest_clubs_of_a_country_qualify_by_its_places() {
        let mut w = World::new(0, 2014);
        for id in 1..=20 {
            add_club_with_eleven(&mut w, id, 1, 99 - id as u8);
        }
        let league = Competition { id: 1, name: "L".into(), country: "Spain".into(), rules: Rules::double_round_robin(), level: Some(1), clubs: (1..=20).collect() };
        w.competitions.insert(1, league);
        assert_eq!(w.champions_league_entrants_by_strength(), vec![1, 2, 3, 4], "Spain had four places");
    }

    #[test]
    fn the_table_decides_who_qualifies_for_the_next_season() {
        let mut w = World::new(0, 2014);
        for id in 1..=6 {
            add_club_with_eleven(&mut w, id, 1, 80);
        }
        let league = Competition { id: 1, name: "L".into(), country: "Italy".into(), rules: Rules::double_round_robin(), level: Some(1), clubs: (1..=6).collect() };
        w.competitions.insert(1, league);
        w.fixtures = vec![played(1, 5, 1, (3, 0)), played(2, 6, 2, (2, 0)), played(3, 5, 2, (1, 0)), played(4, 6, 3, (1, 0))];
        // Italy has three places in 2015. Clubs 5 and 6 won twice; the rest have no points, so goal difference orders them.
        assert_eq!(w.champions_league_entrants(), vec![5, 6, 4]);
    }

    #[test]
    fn a_user_club_that_would_not_qualify_takes_the_weakest_place() {
        let w = world_with_clubs(40);
        let mut entrants: Vec<ClubId> = (1..=32).collect();
        w.place_in_champions_league(&mut entrants, 40);
        assert_eq!(entrants.len(), 32);
        assert!(entrants.contains(&40) && !entrants.contains(&32), "the weakest, club 32, makes room");
        w.place_in_champions_league(&mut entrants, 1);
        assert_eq!(entrants.len(), 32, "a club already in changes nothing");
    }

    #[test]
    fn a_group_stage_has_groups_of_four_that_meet_home_and_away_without_breaking_the_calendar() {
        let mut w = world_with_clubs(32);
        w.start_champions_league(&(1..=32).collect::<Vec<_>>(), &mut Rng::new(1));
        let cl = w.competitions.values().find(|c| c.is_champions_league()).unwrap();
        assert_eq!(cl.groups().len(), 8);
        assert_eq!(w.fixtures.len(), 8 * 12);
        for group in cl.groups() {
            let in_group = w.fixtures.iter().filter(|f| group.contains(&f.home)).count();
            assert_eq!((group.len(), in_group), (4, 12), "every group plays its own twelve matches");
        }
        assert!(w.fixtures.iter().all(|f| !f.must_have_winner), "a group match can be drawn");
        assert!(w.calendar_problems().is_empty(), "{:?}", w.calendar_problems());
    }

    #[test]
    fn the_pots_put_one_club_of_each_strength_band_in_every_group() {
        let mut w = world_with_clubs(32);
        w.start_champions_league(&(1..=32).collect::<Vec<_>>(), &mut Rng::new(3));
        let cl = w.competitions.values().find(|c| c.is_champions_league()).unwrap();
        for group in cl.groups() {
            let mut pots: Vec<u32> = group.iter().map(|&c| (c - 1) / 8).collect();
            pots.sort_unstable();
            assert_eq!(pots, [0, 1, 2, 3], "club ids {group:?}");
        }
    }

    #[test]
    fn too_few_clubs_make_no_champions_league() {
        let mut w = world_with_clubs(7);
        w.start_champions_league(&(1..=7).collect::<Vec<_>>(), &mut Rng::new(1));
        assert!(w.competitions.is_empty() || w.competitions.values().all(|c| !c.is_champions_league()));
        assert!(w.fixtures.is_empty());
    }

    #[test]
    fn the_knockout_rounds_follow_the_groups_down_to_one_final() {
        let (w, added) = play_out(32, 5);
        // 16 round of 16 matches, 8 quarter-finals, 4 semi-finals, and one final.
        assert_eq!(added, [16, 8, 4, 1]);
        let last_round = w.fixtures.iter().map(|f| f.round).max().unwrap();
        let final_matches: Vec<_> = w.fixtures.iter().filter(|f| f.round == last_round).collect();
        assert_eq!(final_matches.len(), 1);
        assert!(final_matches[0].must_have_winner, "a final always has a winner");
        assert!(w.calendar_problems().is_empty(), "{:?}", w.calendar_problems());
    }

    #[test]
    fn each_tie_is_over_two_legs_and_only_the_second_decides_a_level_score() {
        let (w, _) = play_out(32, 6);
        for round in 7..=12u8 {
            let legs: Vec<_> = w.fixtures.iter().filter(|f| f.round == round).collect();
            let is_first_leg = round % 2 == 1;
            assert!(legs.iter().all(|f| f.must_have_winner != is_first_leg), "round {round}");
        }
        let first = w.fixtures.iter().find(|f| f.round == 7).unwrap();
        let second = w.fixtures.iter().find(|f| f.round == 8 && f.home == first.away && f.away == first.home).expect("the return leg");
        assert!(first.week < second.week, "the second leg is played later");
    }

    #[test]
    fn a_knockout_tie_is_won_on_goals_over_both_legs_then_by_the_second_leg() {
        let a_wins_on_aggregate = tie_winner(&played(1, 1, 2, (2, 0)), &played(2, 2, 1, (1, 0)));
        let b_wins_on_aggregate = tie_winner(&played(1, 1, 2, (0, 1)), &played(2, 2, 1, (1, 1)));
        let mut second_leg = played(2, 2, 1, (1, 1));
        second_leg.result = Some(Outcome { full_time: (1, 1), extra_time: Some((0, 0)), shootout: Some((4, 5)) });
        let level_goes_to_the_second_leg_winner = tie_winner(&played(1, 1, 2, (0, 0)), &second_leg);
        assert_eq!((a_wins_on_aggregate, b_wins_on_aggregate, level_goes_to_the_second_leg_winner), (1, 2, 1));
    }

    #[test]
    fn no_group_winner_meets_the_runner_up_of_its_own_group_when_it_can_be_avoided() {
        for seed in 1..=30 {
            let winners: Vec<ClubId> = (1..=8).collect();
            let runners_up: Vec<ClubId> = (11..=18).collect();
            let pairs = draw_knockout_start(&winners, &runners_up, &mut Rng::new(seed));
            assert!(pairs.iter().all(|&(runner_up, winner)| runner_up - 10 != winner), "seed {seed}: {pairs:?}");
            let mut firsts: Vec<ClubId> = pairs.iter().map(|p| p.0).collect();
            firsts.sort_unstable();
            assert_eq!(firsts, runners_up, "every runner-up is drawn once");
        }
    }

    #[test]
    fn the_same_seed_gives_the_same_champions_league() {
        let (a, b) = (play_out(32, 9).0, play_out(32, 9).0);
        let teams = |w: &World| w.fixtures.iter().map(|f| (f.round, f.week, f.home, f.away)).collect::<Vec<_>>();
        assert_eq!(teams(&a), teams(&b));
    }

    #[test]
    fn a_real_season_with_the_champions_league_finishes_and_the_next_one_qualifies_from_its_tables() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../history.db");
        if !std::path::Path::new(path).exists() {
            return; // needs `cargo run -p import` first
        }
        let mut w = crate::source::load_edition(&rusqlite::Connection::open(path).unwrap(), 15).unwrap();
        let mut rng = Rng::new(1);
        w.start_season(&mut rng);
        let entrants = w.champions_league_entrants_by_strength();
        w.start_champions_league(&entrants, &mut rng);
        let cl = w.competitions.values().find(|c| c.is_champions_league()).expect("a Champions League").id;
        assert_eq!(w.competitions[&cl].clubs.len(), 32);
        assert!(w.calendar_problems().is_empty(), "{:?}", w.calendar_problems());
        w.play_season_to_end(&mut rng);
        assert!(w.season_finished() && w.calendar_problems().is_empty(), "{:?}", w.calendar_problems());
        let rounds = w.fixtures.iter().filter(|f| f.competition == cl).map(|f| f.round).max();
        assert_eq!(rounds, Some(13), "six group matchdays, three two-legged rounds and a final");
        w.start_next_season(&mut rng).expect("the season is over");
        assert_eq!(w.competitions.values().filter(|c| c.is_champions_league()).count(), 1);
        assert!(w.calendar_problems().is_empty());
    }
}
