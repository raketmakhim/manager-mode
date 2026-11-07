//! Game logic. No UI code here.
//!
//! - `attrs`: the player attributes (`Attr`, `Attrs`) and which ones outfield players and keepers have.
//! - `competition`: leagues and cups as lists of stages, schedules and the weekly match limit.
//! - `history`: reading real player history out of history.db.
//! - `model`: the live game state (`World`, `Club`, `Player`...).
//! - `outcome`: the result of a played match (full time, extra time, shootout).
//! - `source`: building a `World` from history.db.
mod attrs;
mod competition;
mod history;
mod model;
mod outcome;
mod source;

pub use attrs::{Attr, Attrs, KEEPER, OUTFIELD};
pub use competition::{round_robin, Competition, Entry, Format, Rules, Stage};
pub use history::{career, edition_label, player_name, search, season_label, Hit, Snapshot};
pub use model::{
    Club, ClubId, CompetitionId, Fixture, Player, PlayerId, Position, World, FULL_FITNESS, MAX_MATCHES_PER_WEEK,
    NEUTRAL_FORM,
};
pub use outcome::{Outcome, Side};
pub use rusqlite::Connection;
pub use source::{editions, load_edition};
