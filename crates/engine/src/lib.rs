//! Game logic. No UI code here.
//!
//! - `attrs`: the player attributes (`Attr`, `Attrs`) and which ones outfield players and keepers have.
//! - `history`: reading real player history out of history.db.
//! - `model`: the live game state (`World`, `Club`, `Player`...).
//! - `source`: building a `World` from history.db.
//! - `fixtures`: season schedules (round-robin).
mod attrs;
mod fixtures;
mod history;
mod model;
mod source;

pub use attrs::{Attr, Attrs, KEEPER, OUTFIELD};
pub use fixtures::round_robin;
pub use history::{career, edition_label, player_name, search, season_label, Hit, Snapshot};
pub use model::{
    Club, ClubId, Competition, CompetitionId, CompetitionKind, Fixture, Player, PlayerId, Position, World, FULL_FITNESS,
    MAX_MATCHES_PER_WEEK, NEUTRAL_FORM,
};
pub use rusqlite::Connection;
pub use source::{editions, load_edition};
