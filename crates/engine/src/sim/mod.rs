//! Match models: how a match is played out. Every model is a function from the two lineups and a seeded RNG to a
//! `MatchReport`, and `simulate` picks one by `MatchModel`. A competition's `Rules` say which model its matches use.
//!
//! - `cheap`: team strength to a score (Poisson goals with a Dixon-Coles correction). No events.
//!
//! The report always has the score. A model that plays the match out also returns its events as `detail`; stats and
//! match ratings are worked out from those events, so any model that emits the events they need works with them.
//! Adding a model: a new file, a `MatchModel` variant and an arm in `simulate`; the compiler lists the rest.
mod cheap;

pub use cheap::team_strength;

use crate::model::PlayerId;
use crate::outcome::{Outcome, Side};
use crate::rng::Rng;

/// Which model plays a match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchModel {
    /// Team strength to a score: fast, no events. For every competition that does not need detail.
    Cheap,
}

impl MatchModel {
    /// Every model, so a test can run them all.
    pub const ALL: &'static [MatchModel] = &[MatchModel::Cheap];
}

/// What a model needs to know about one side.
#[derive(Clone, Copy, Debug)]
pub struct MatchTeam {
    /// The rating (0-99) of the player in each lineup slot, in `Formation::slots` order; 0 for an empty slot.
    pub ratings: [u8; 11],
}

/// Everything a model needs to play one match.
#[derive(Clone, Copy, Debug)]
pub struct MatchInput {
    pub home: MatchTeam,
    pub away: MatchTeam,
    /// True for a cup tie: a level score goes on to extra time and then a shootout.
    pub must_have_winner: bool,
}

/// Something that happened in a match.
// Models with more detail add variants; code that reads events matches the ones it knows and ignores the rest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Event {
    Goal {
        /// 1 to 90 (and beyond for stoppage time and extra time).
        minute: u8,
        side: Side,
        scorer: PlayerId,
        /// `None` for a goal nobody set up.
        assister: Option<PlayerId>,
    },
}

/// What a model that plays the match out returns on top of the score.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MatchDetail {
    /// In time order.
    pub events: Vec<Event>,
}

/// The result of a simulated match.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchReport {
    /// The score, extra time and shootout. Every model gives this.
    pub outcome: Outcome,
    /// `None` for a model that only produces the score.
    pub detail: Option<MatchDetail>,
}

/// Plays one match with the given model.
pub fn simulate(model: MatchModel, input: &MatchInput, rng: &mut Rng) -> MatchReport {
    match model {
        MatchModel::Cheap => cheap::simulate(input, rng),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(home: u8, away: u8, must_have_winner: bool) -> MatchInput {
        MatchInput { home: MatchTeam { ratings: [home; 11] }, away: MatchTeam { ratings: [away; 11] }, must_have_winner }
    }

    // Rules every model must keep; a new model is added to `MatchModel::ALL` and these run for it too.

    #[test]
    fn every_model_repeats_for_the_same_seed() {
        for &model in MatchModel::ALL {
            let play = |seed| simulate(model, &input(75, 68, true), &mut Rng::new(seed));
            assert_eq!(play(4), play(4), "{model:?}");
        }
    }

    #[test]
    fn every_model_gives_a_cup_tie_a_winner_and_keeps_a_league_match_to_90_minutes() {
        for &model in MatchModel::ALL {
            let mut rng = Rng::new(3);
            for _ in 0..1000 {
                assert!(simulate(model, &input(70, 70, true), &mut rng).outcome.winner().is_some(), "{model:?}");
                let league = simulate(model, &input(70, 70, false), &mut rng).outcome;
                assert!(league.extra_time.is_none() && league.shootout.is_none(), "{model:?}");
            }
        }
    }

    #[test]
    fn every_model_lets_the_stronger_side_win_more_often() {
        for &model in MatchModel::ALL {
            let mut rng = Rng::new(7);
            let (mut strong, mut weak) = (0, 0);
            for _ in 0..2000 {
                match simulate(model, &input(80, 70, false), &mut rng).outcome.winner() {
                    Some(Side::Home) => strong += 1,
                    Some(Side::Away) => weak += 1,
                    None => {}
                }
            }
            assert!(strong > 2 * weak, "{model:?}: {strong} against {weak}");
        }
    }

    #[test]
    fn a_model_with_detail_has_goal_events_that_add_up_to_the_score() {
        for &model in MatchModel::ALL {
            let mut rng = Rng::new(5);
            for _ in 0..500 {
                let report = simulate(model, &input(72, 70, true), &mut rng);
                let Some(detail) = report.detail else { continue }; // a score-only model has nothing to check
                let count = |side| detail.events.iter().filter(|e| matches!(e, Event::Goal { side: s, .. } if *s == side)).count();
                let (home, away) = report.outcome.goals();
                assert_eq!((count(Side::Home), count(Side::Away)), (home as usize, away as usize), "{model:?}");
            }
        }
    }
}
