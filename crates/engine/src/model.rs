//! The live game state. Everything points at everything else by id, so the whole `World` is plain data
//! (easy to save, and no borrowing puzzles). Unlike `Snapshot`, which is one read-only row of history,
//! a `Player` here is the player right now and the simulation changes it.
use crate::attrs::Attrs;
use crate::competition::Competition;
use crate::outcome::Outcome;
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

/// Defines `Position` plus its short code and `parse` in one place, so the three cannot drift apart.
/// Each line is: variant name, code as written in the data. Grouped goalkeeper, defence, midfield, attack; inside
/// each group left to right (centre positions deepest first).
macro_rules! positions {
    ($($name:ident $code:literal),* $(,)?) => {
        /// Where a player can play, e.g. `Position::St`.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Position { $($name),* }

        impl Position {
            /// Every position, in the order they are listed below.
            pub const ALL: &'static [Position] = &[$(Position::$name),*];
            /// Short code, e.g. "ST".
            pub fn code(self) -> &'static str { match self { $(Position::$name => $code),* } }
            /// "ST" -> `Some(Position::St)`; anything else is `None`.
            pub fn parse(s: &str) -> Option<Position> { match s { $($code => Some(Position::$name),)* _ => None } }
        }
    };
}

positions! {
    Gk  "GK",

    Lwb "LWB",
    Lb  "LB",
    Cb  "CB",
    Rb  "RB",
    Rwb "RWB",

    Lm  "LM",
    Cdm "CDM",
    Cm  "CM",
    Cam "CAM",
    Rm  "RM",

    Lw  "LW",
    Cf  "CF",
    St  "ST",
    Rw  "RW",
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
    /// The starting eleven. Empty until lineup selection exists.
    pub lineup: Vec<PlayerId>,
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
}
