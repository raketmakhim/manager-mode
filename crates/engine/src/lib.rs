//! Game logic. No UI code here.
//!
//! - `attrs`: the player attributes (`Attr`, `Attrs`) and which ones outfield players and keepers have.
//! - `competition`: leagues and cups as lists of stages, schedules and the weekly match limit.
//! - `history`: reading real player history out of history.db.
//! - `lineup`: formations, and picking the best eleven for one.
//! - `model`: the live game state (`World`, `Club`, `Player`...).
//! - `outcome`: the result of a played match (full time, extra time, shootout).
//! - `overall`: how overall is built from attributes, by position (weights; `overall_change`).
//! - `rating`: player match ratings behind one function, `rate` (a swappable `RatingModel`).
//! - `rng`: the seeded random number generator every simulation takes.
//! - `sim`: the match models behind one function, `simulate`: cheap (team strength to a score) and shot-based (shots, saves, goals, cards, corners, possession). Inside the shot-based model, who shoots, scores and assists (`ScoringModel`), who fouls and is carded (`DisciplineModel`), who is substituted when (`SubstitutionModel`) and how tired players get (`FatigueModel`) are swappable models of their own. `development` is the same for experience (`ExperienceModel`): every match gives XP, and a full XP bar levels the user's players up, away from their real careers. A `LiveMatch` plays the shot-based model a minute at a time so the user can change a side in between.
//! - `source`: building a `World` from history.db.
//! - `stats`: what each player did, added up from the events of the matches played.
//! - `transfer`: players changing clubs in the summer (`Transfer`, `Window`); the real moves come from the data (`source::plan_transfers`).
mod attrs;
mod competition;
mod development;
mod history;
mod lineup;
mod model;
mod outcome;
mod overall;
mod rating;
mod rng;
mod sim;
mod source;
mod stats;
mod transfer;
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
pub use development::{Development, ExperienceModel, XP_PER_LEVEL};
pub use rating::RatingModel;
pub use overall::{overall_change, weights};
pub use rng::Rng;
pub use sim::{simulate, simulate_shot_based, team_strength, DisciplineModel, FatigueModel, LiveMatch, SubstitutionError, TacticsError, SubstitutionModel, ShotBasedParts, DEFAULT_SHOT_BASED_PARTS, Event, MatchDetail, MatchInput, MatchModel, MatchPlayer, MatchReport, MatchTeam, ScoringModel};
pub use rusqlite::Connection;
pub use source::{advance_ratings, editions, load_edition, plan_transfers};
pub use transfer::{Transfer, TransferKind, Window};
pub use stats::{live_match_stats, PlayerStats};
