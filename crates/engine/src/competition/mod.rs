//! Competitions: a league or a cup is a list of stages, and every format shares the same pieces.
//!
//! - `pairing`: who plays whom in a league, as a pure function (`round_robin`).
//! - `knockout`: cups, drawn round by round (`draw_round`, `World::advance_cups`).
//! - `calendar`: when matches are played, and the limit on matches per week.
//!
//! Design and reasons: `research/engineering/competition-design.md`.
mod calendar;
mod knockout;
mod pairing;

pub use calendar::week_date;
pub use knockout::draw_round;
pub use pairing::round_robin;

use crate::model::{ClubId, CompetitionId, World};
use crate::rng::Rng;

/// A league or a cup: a set of clubs that play fixtures against each other under some rules.
pub struct Competition {
    /// A league uses its `league_id` from history.db; cups will use ids above all of those.
    pub id: CompetitionId,
    pub name: String,
    /// Country, e.g. "England". Several countries have a league with the same name.
    pub country: String,
    /// The rules for the season this competition is played in.
    pub rules: Rules,
    pub clubs: Vec<ClubId>,
}

impl Competition {
    /// True for a plain league: one stage, a round-robin of every club, so the whole schedule is known up front.
    pub fn is_league(&self) -> bool {
        matches!(self.rules.stages.as_slice(), [Stage { format: Format::RoundRobin, entry: Entry::AllClubs }])
    }

    /// True for a plain cup: one stage, a single-leg knockout of every club.
    pub fn is_cup(&self) -> bool {
        matches!(self.rules.stages.as_slice(), [Stage { format: Format::Knockout, entry: Entry::AllClubs }])
    }
}

/// How a competition is played, as a list of stages that run one after another.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rules {
    /// Played in order: a later stage takes its clubs from earlier ones through its `Entry`.
    pub stages: Vec<Stage>,
}

impl Rules {
    /// Everyone plays everyone home and away. The default for any league we have not researched.
    pub fn double_round_robin() -> Rules {
        Rules { stages: vec![Stage { format: Format::RoundRobin, entry: Entry::AllClubs }] }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stage {
    /// How the stage is played.
    pub format: Format,
    /// Which clubs play it.
    pub entry: Entry,
}

/// The shape of a stage. League phase and groups are added when a competition needs them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Single-leg ties; the loser is out. Extra time and penalties settle a draw.
    Knockout,

    /// Every club plays every other home and away.
    RoundRobin,
}

/// Which clubs play a stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    /// Every club of the competition.
    AllClubs,
}

impl World {
    /// Game setup for a freshly loaded world: adds the default cups, makes every league's fixtures and draws
    /// the first round of every cup. The one call a new career makes.
    pub fn start_season(&mut self, rng: &mut Rng) {
        self.add_default_cups();
        self.generate_fixtures();
        self.advance_cups(rng);
    }

    /// Replaces `fixtures` with a full season for every league. Cups are skipped: a cup's rounds can only be
    /// drawn after the previous one is played, so `advance_cups` adds them during the season.
    pub fn generate_fixtures(&mut self) {
        self.fixtures = self
            .competitions
            .values()
            .filter(|c| c.is_league())
            .flat_map(|c| round_robin(c.id, &c.clubs))
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::pairing::assert_valid;
    use super::*;
    use crate::source::load_edition;
    use rusqlite::Connection;

    #[test]
    fn the_default_rules_make_a_league() {
        let c = Competition { id: 1, name: "L".into(), country: "X".into(), rules: Rules::double_round_robin(), clubs: vec![] };
        assert!(c.is_league());
        let cup_like = Rules { stages: vec![] };
        assert!(!Competition { rules: cup_like, ..c }.is_league());
    }

    #[test]
    fn a_competition_that_is_not_a_league_gets_no_fixtures_up_front() {
        let mut w = World::new(0, 0);
        let rules = Rules { stages: vec![] };
        w.competitions.insert(1, Competition { id: 1, name: "C".into(), country: "X".into(), rules, clubs: vec![1, 2, 3, 4] });
        w.generate_fixtures();
        assert!(w.fixtures.is_empty());
    }

    #[test]
    fn starting_a_season_schedules_leagues_and_the_first_cup_round_within_the_weekly_limit() {
        let mut w = World::new(0, 0);
        let clubs: Vec<ClubId> = (1..=20).collect();
        w.competitions.insert(1, Competition { id: 1, name: "L".into(), country: "X".into(), rules: Rules::double_round_robin(), clubs });
        w.start_season(&mut Rng::new(1));
        let cup = w.competitions.values().find(|c| c.is_cup()).expect("a default cup");
        assert_eq!(w.fixtures.iter().filter(|f| f.competition == 1).count(), 20 * 19);
        assert_eq!(w.fixtures.iter().filter(|f| f.competition == cup.id).count(), 4); // 20 clubs: 4 preliminary ties, 12 byes
        assert!(w.calendar_problems().is_empty());
    }

    #[test]
    fn every_real_fifa_15_league_gets_a_valid_schedule_and_calendar() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../history.db");
        if !std::path::Path::new(path).exists() { return; } // needs `cargo run -p import` first
        let mut w = load_edition(&Connection::open(path).unwrap(), 15).unwrap();
        w.generate_fixtures();
        let mut total = 0;
        for c in w.competitions.values().filter(|c| c.is_league() && c.clubs.len() >= 2) {
            assert_valid(c.id, &c.clubs, w.fixtures.iter().filter(|f| f.competition == c.id));
            total += c.clubs.len() * (c.clubs.len() - 1);
        }
        assert_eq!(w.fixtures.len(), total); // one-club leagues add nothing
        assert!(w.calendar_problems().is_empty());
    }
}
