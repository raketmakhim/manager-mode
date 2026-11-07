//! Game logic. No UI code here.
//!
//! - `attrs`: the player attributes (`Attr`, `Attrs`) and which ones outfield players and keepers have.
//! - `competition`: leagues and cups as lists of stages, schedules and the weekly match limit.
//! - `history`: reading real player history out of history.db.
//! - `model`: the live game state (`World`, `Club`, `Player`...).
//! - `outcome`: the result of a played match (full time, extra time, shootout).
//! - `rng`: the seeded random number generator every simulation takes.
//! - `sim`: the cheap match simulation (team strength, goals, extra time, shootout).
//! - `source`: building a `World` from history.db.
mod attrs;
mod competition;
mod history;
mod model;
mod outcome;
mod rng;
mod sim;
mod source;

pub use attrs::{Attr, Attrs, KEEPER, OUTFIELD};
pub use competition::{draw_round, match_date, round_robin, standings, ClubMove, Competition, Entry, Format, Rules, Stage, TableRow};
pub use history::{career, edition_label, player_name, search, season_label, season_name, Hit, Snapshot};
pub use model::{
    Club, ClubId, CompetitionId, Day, Fixture, Player, PlayerId, Position, World, FULL_FITNESS, MAX_MATCHES_PER_WEEK,
    NEUTRAL_FORM,
};
pub use outcome::{Outcome, Side};
pub use rng::Rng;
pub use sim::{simulate_match, team_strength};
pub use rusqlite::Connection;
pub use source::{editions, load_edition};
