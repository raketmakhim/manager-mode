//! The live game state. Everything points at everything else by id, so the whole `World` is plain data
//! (easy to save, and no borrowing puzzles). Unlike `Snapshot`, which is one read-only row of history,
//! a `Player` here is the player right now and the simulation changes it.
use crate::attrs::Attrs;
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

/// Where a player can play. Grouped goalkeeper, defence, midfield, attack; inside each group left to right
/// (centre positions deepest first).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    Gk,

    Lwb,
    Lb,
    Cb,
    Rb,
    Rwb,

    Lm,
    Cdm,
    Cm,
    Cam,
    Rm,

    Lw,
    Cf,
    St,
    Rw,
}

impl Position {
    /// "ST" -> `Some(Position::St)`; anything else is `None`.
    pub fn parse(s: &str) -> Option<Position> {
        Some(match s {
            "GK" => Position::Gk,

            "LWB" => Position::Lwb,
            "LB" => Position::Lb,
            "CB" => Position::Cb,
            "RB" => Position::Rb,
            "RWB" => Position::Rwb,

            "LM" => Position::Lm,
            "CDM" => Position::Cdm,
            "CM" => Position::Cm,
            "CAM" => Position::Cam,
            "RM" => Position::Rm,

            "LW" => Position::Lw,
            "CF" => Position::Cf,
            "ST" => Position::St,
            "RW" => Position::Rw,
            _ => return None,
        })
    }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompetitionKind {
    /// Everyone plays everyone home and away; the whole schedule is known up front.
    League,
    /// Knockout. Not built yet. Each round can only be drawn once the last one is played.
    Cup,
}

/// A league or a cup: a set of clubs that play fixtures against each other.
pub struct Competition {
    pub id: CompetitionId,
    pub name: String,
    /// Country, e.g. "England". Several countries have a league with the same name.
    pub country: String,
    pub kind: CompetitionKind,
    pub clubs: Vec<ClubId>,
}

pub struct Fixture {
    pub competition: CompetitionId,
    /// Matchday within its competition: 1 = the first round of that league or cup.
    pub round: u8,
    /// When it is played: 1 = the first week of the season. Rounds of different competitions can share a week,
    /// but no club may play more than `MAX_MATCHES_PER_WEEK` in one.
    pub week: u8,
    pub home: ClubId,
    pub away: ClubId,
    /// (home goals, away goals); `None` until played.
    pub result: Option<(u8, u8)>,
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
