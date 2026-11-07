//! The live game state. Everything points at everything else by id, so the whole `World` is plain data
//! (easy to save, and no borrowing puzzles). Unlike `Snapshot`, which is one read-only row of history,
//! a `Player` here is the player right now and the simulation changes it.
use crate::attrs::Attrs;
use crate::competition::Competition;
use crate::lineup::{Formation, Selection};
use crate::outcome::{Outcome, Side};
use std::collections::BTreeMap;

/// Same number as `player_id` in history.db, so we can look up a player's real future.
pub type PlayerId = u32;
pub type ClubId = u32;
/// A league or a cup. League ids are the `league_id` from history.db; cups will use ids above all of those.
pub type CompetitionId = u32;

/// Starting value for game-only fields the data does not have.
pub const NEUTRAL_FORM: u8 = 50;
pub const FULL_FITNESS: u8 = 100;

/// The most matches a club may play in one week (a 7-day block of the season).
pub const MAX_MATCHES_PER_WEEK: usize = 3;

/// Which part of the pitch a position is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Line {
    Goalkeeper,
    Defence,
    Midfield,
    Attack,
}

/// Defines `Position` plus its short code, its line and `parse` in one place, so they cannot drift apart.
/// Each line is: variant name, code as written in the data, line. Grouped goalkeeper, defence, midfield, attack;
/// inside each group left to right (centre positions deepest first).
macro_rules! positions {
    ($($name:ident $code:literal $line:ident),* $(,)?) => {
        /// Where a player can play, e.g. `Position::St`.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Position { $($name),* }

        impl Position {
            /// Every position, in the order they are listed below.
            pub const ALL: &'static [Position] = &[$(Position::$name),*];
            /// Short code, e.g. "ST".
            pub fn code(self) -> &'static str { match self { $(Position::$name => $code),* } }
            /// The part of the pitch he plays on.
            pub fn line(self) -> Line { match self { $(Position::$name => Line::$line),* } }
            /// "ST" -> `Some(Position::St)`; anything else is `None`.
            pub fn parse(s: &str) -> Option<Position> { match s { $($code => Some(Position::$name),)* _ => None } }
        }
    };
}

positions! {
    Gk  "GK"  Goalkeeper,

    Lwb "LWB" Defence,
    Lb  "LB"  Defence,
    Cb  "CB"  Defence,
    Rb  "RB"  Defence,
    Rwb "RWB" Defence,

    Lm  "LM"  Midfield,
    Cdm "CDM" Midfield,
    Cm  "CM"  Midfield,
    Cam "CAM" Midfield,
    Rm  "RM"  Midfield,

    Lw  "LW"  Attack,
    Cf  "CF"  Attack,
    St  "ST"  Attack,
    Rw  "RW"  Attack,
}

pub struct Player {
    pub id: PlayerId,
    pub name: String,
    /// "1985-10-24".
    pub birth_date: String,
    /// Main position first.
    pub positions: Vec<Position>,
    /// Current ratings. These are what the simulation changes over time.
    pub attrs: Attrs,
    /// 0-99.
    pub overall: u8,
    /// 0-99.
    pub potential: u8,
    /// `None` = free agent.
    pub club: Option<ClubId>,
    /// Euros per week. 0 if unknown.
    pub wage: u32,
    /// Last year of the contract; `None` for a free agent.
    pub contract_until: Option<u16>,
    /// Transfer value in euros. 0 if unknown.
    pub value: u32,
    /// 0-100. Game-only, not in the data.
    pub form: u8,
    /// 0-100. Game-only, not in the data.
    pub fitness: u8,
}

pub struct Club {
    pub id: ClubId,
    pub name: String,
    /// The club's domestic league.
    pub league: CompetitionId,
    /// Everyone at the club, best first at load time.
    pub squad: Vec<PlayerId>,
    /// Transfer budget in euros; `None` when the data does not have it (all of FC 24).
    pub budget: Option<i64>,
    /// How the club lines up.
    pub formation: Formation,
    /// The starting eleven and the bench the user has set. Empty means nothing was set, and `World::selection_of`
    /// picks one. Read it through `selection_of`, not from here.
    pub selection: Selection,
}

/// The day of the week a match is played. A season week starts on the Saturday, so the order here is the order
/// within a week: league matches on Saturday, then the midweek cup and European nights.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Day {
    Saturday,
    Tuesday,
    Wednesday,
    Thursday,
}

impl Day {
    /// Days after that week's Saturday.
    pub fn days_after_saturday(self) -> i64 {
        match self {
            Day::Saturday => 0,
            Day::Tuesday => 3,
            Day::Wednesday => 4,
            Day::Thursday => 5,
        }
    }

    /// "Sat", "Tue", "Wed" or "Thu".
    pub fn short_name(self) -> &'static str {
        match self {
            Day::Saturday => "Sat",
            Day::Tuesday => "Tue",
            Day::Wednesday => "Wed",
            Day::Thursday => "Thu",
        }
    }
}

/// What is shown for a match that has no result yet.
pub const NO_RESULT: &str = "-";

pub struct Fixture {
    pub competition: CompetitionId,
    /// Matchday within its competition: 1 = the first round of that league or cup.
    pub round: u8,
    /// When it is played: 1 = the first week of the season. Rounds of different competitions can share a week,
    /// but no club may play more than `MAX_MATCHES_PER_WEEK` in one.
    pub week: u8,
    /// Which day of that week. League matches are on Saturdays.
    pub day: Day,
    pub home: ClubId,
    pub away: ClubId,
    /// `None` until played.
    pub result: Option<Outcome>,
}

impl Fixture {
    /// True if the club plays in this match, at home or away.
    pub fn involves(&self, club: ClubId) -> bool {
        self.home == club || self.away == club
    }

    /// The club that won, or `None` for a draw or a match not played yet.
    pub fn winner(&self) -> Option<ClubId> {
        match self.result?.winner()? {
            Side::Home => Some(self.home),
            Side::Away => Some(self.away),
        }
    }

    /// The club that lost, or `None` for a draw or a match not played yet.
    pub fn loser(&self) -> Option<ClubId> {
        match self.result?.winner()? {
            Side::Home => Some(self.away),
            Side::Away => Some(self.home),
        }
    }

    /// "2-1", "2-1 aet", or `NO_RESULT` before it is played.
    pub fn result_text(&self) -> String {
        self.result.map_or(NO_RESULT.to_string(), |o| o.to_string())
    }
}

/// The whole game in one place. Ordered maps (not HashMap) so that looping over them gives the
/// same order every run, which keeps a seeded simulation repeatable.
pub struct World {
    /// FIFA version it was loaded from (15 = FIFA 15).
    pub edition: i64,
    /// Calendar year the season starts, e.g. 2014 for FIFA 15 (the 2014/15 season).
    pub year: u16,
    pub competitions: BTreeMap<CompetitionId, Competition>,
    pub clubs: BTreeMap<ClubId, Club>,
    pub players: BTreeMap<PlayerId, Player>,
    pub fixtures: Vec<Fixture>,
}

impl World {
    /// An empty world for the given edition and season start year.
    pub fn new(edition: i64, year: u16) -> World {
        World {
            edition,
            year,
            competitions: BTreeMap::new(),
            clubs: BTreeMap::new(),
            players: BTreeMap::new(),
            fixtures: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_position_code_parses_back_to_itself() {
        assert_eq!(Position::ALL.len(), 15);
        for &position in Position::ALL {
            assert_eq!(Position::parse(position.code()), Some(position));
        }
        assert_eq!(Position::parse("XX"), None);
    }

    fn fixture(result: Option<Outcome>) -> Fixture {
        Fixture { competition: 1, round: 1, week: 1, day: Day::Saturday, home: 10, away: 20, result }
    }

    #[test]
    fn a_fixture_knows_its_clubs_winner_loser_and_result_text() {
        let won_by_home = fixture(Some(Outcome::full_time(2, 1)));
        assert!(won_by_home.involves(10) && won_by_home.involves(20) && !won_by_home.involves(30));
        assert_eq!((won_by_home.winner(), won_by_home.loser()), (Some(10), Some(20)));
        assert_eq!(won_by_home.result_text(), "2-1");

        let draw = fixture(Some(Outcome::full_time(1, 1)));
        assert_eq!((draw.winner(), draw.loser()), (None, None));

        let unplayed = fixture(None);
        assert_eq!((unplayed.winner(), unplayed.loser()), (None, None));
        assert_eq!(unplayed.result_text(), "-");

        let shootout = fixture(Some(Outcome { full_time: (1, 1), extra_time: Some((0, 0)), shootout: Some((3, 4)) }));
        assert_eq!(shootout.winner(), Some(20), "a shootout decides it");
    }
}
