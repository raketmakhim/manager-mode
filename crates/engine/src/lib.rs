//! Game logic. No UI code here.
//!
//! - `attrs`: the player attributes (`Attr`, `Attrs`) and which ones outfield players and keepers have.
//! - `competition`: leagues and cups as lists of stages, schedules and the weekly match limit.
//! - `history`: reading real player history out of history.db.
//! - `lineup`: formations, and picking the best eleven for one.
//! - `model`: the live game state (`World`, `Club`, `Player`...).
//! - `outcome`: the result of a played match (full time, extra time, shootout).
//! - `overall`: how overall is built from attributes, by position (weights; `overall_change`).
//! - `rng`: the seeded random number generator every simulation takes.
//! - `sim`: the match models (cheap: team strength to a score) behind one function, `simulate`.
//! - `source`: building a `World` from history.db.
mod attrs;
mod competition;
mod history;
mod lineup;
mod model;
mod outcome;
mod overall;
mod rng;
mod sim;
mod source;
#[cfg(test)]
mod testing;

pub use attrs::{Attr, Attrs, KEEPER, OUTFIELD};
pub use competition::{
    draw_round, match_date, round_robin, standings, ClubMove, Competition, Entry, Format, Rules, Stage, TableRow,
    TOP_FIVE_LEAGUES,
};
pub use history::{career, edition_label, player_name, search, season_label, season_name, Hit, Snapshot};
pub use lineup::{
    familiarity, pick_bench, pick_lineup, rating_at, Bench, Familiarity, Formation, Lineup, Selection, Spot, BENCH_SIZE,
};
pub use model::{
    Club, ClubId, CompetitionId, Day, Fixture, Line, Player, PlayerId, Position, World, FULL_FITNESS, MAX_MATCHES_PER_WEEK, NO_RESULT,
    NEUTRAL_FORM,
};
pub use outcome::{Outcome, Side};
pub use overall::{overall_change, weights};
pub use rng::Rng;
pub use sim::{simulate, team_strength, Event, MatchDetail, MatchInput, MatchModel, MatchReport, MatchTeam};
pub use rusqlite::Connection;
pub use source::{advance_ratings, editions, load_edition};
