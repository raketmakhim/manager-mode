//! Player match ratings: how well one player played in one match, from 3.0 to 10.0. A rating model is a pure function
//! from what a player did and how his team fared (`RatingInput`) to a number, and `rate` picks one by `RatingModel`.
//! Adding a model: a new file, a `RatingModel` variant and an arm in `rate`; the compiler lists the rest, and the
//! contract tests below run for it through `RatingModel::ALL`.
//!
//! - `formula`: a base rating plus the player's goals, assists, saves, clean sheets and cards, by position.
//! - `overall_based`: the same events, smaller, on top of a starting rating that follows the player's overall.
//!
//! A model sees one player at a time and only his summary of the match, so it works for any match model that
//! produces the events the summary is built from (`stats::match_stats`).
mod formula;
mod overall_based;

use crate::model::Position;
use crate::stats::PlayerStats;

/// Which model rates the players.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RatingModel {
    /// A base rating and event bonuses and penalties, scaled by position and minutes played.
    Formula,
    /// Starts from the player's overall and adds a smaller share of the events: a star rates higher for the same game.
    OverallBased,
}

impl RatingModel {
    /// Every model, so a test can run them all.
    pub const ALL: &'static [RatingModel] = &[RatingModel::Formula, RatingModel::OverallBased];
}

/// The model that rates the players of every match.
pub const DEFAULT_RATING_MODEL: RatingModel = RatingModel::OverallBased;

/// A player who is on the pitch for fewer minutes than this gets no rating (as on Sofascore).
pub const MIN_RATED_MINUTES: u16 = 10;
/// Every rating is between these.
pub const RATING_RANGE: (f32, f32) = (3.0, 10.0);

/// How the player's team did in the match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TeamResult {
    Win,
    Draw,
    Loss,
}

/// What a rating model knows about one player's match.
#[derive(Clone, Copy, Debug)]
pub struct RatingInput<'a> {
    /// The slot he played.
    pub position: Position,
    /// His rating in that slot (0-99), which is his overall at his own position.
    pub overall: u8,
    /// Minutes on the pitch (90, 120 with extra time, fewer if sent off).
    pub minutes: u16,
    /// What he did: goals, assists, shots, saves, conceded, clean sheet, cards.
    pub stats: &'a PlayerStats,
    pub result: TeamResult,
}

/// The rating (3.0 to 10.0) of one player in one match, or `None` if he played too few minutes to be rated.
pub fn rate(model: RatingModel, input: &RatingInput) -> Option<f32> {
    if input.minutes < MIN_RATED_MINUTES {
        return None;
    }
    let rating = match model {
        RatingModel::Formula => formula::rate(input),
        RatingModel::OverallBased => overall_based::rate(input),
    };
    Some(rating.clamp(RATING_RANGE.0, RATING_RANGE.1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use Position::*;

    fn rated(model: RatingModel, position: Position, stats: PlayerStats, result: TeamResult) -> f32 {
        let input = RatingInput { position, overall: 70, minutes: 90, stats: &stats, result };
        rate(model, &input).expect("90 minutes is enough to be rated")
    }

    fn with(f: impl Fn(&mut PlayerStats)) -> PlayerStats {
        let mut s = PlayerStats { appearances: 1, minutes: 90, ..PlayerStats::default() };
        f(&mut s);
        s
    }

    // Rules every model must keep; a new model is added to `RatingModel::ALL` and these run for it too.

    #[test]
    fn every_model_stays_in_range_and_rates_nobody_under_ten_minutes() {
        for &model in RatingModel::ALL {
            let wild = with(|s| (s.goals, s.assists, s.reds, s.conceded) = (9, 9, 3, 9));
            let awful = with(|s| (s.reds, s.yellows, s.conceded) = (3, 5, 20));
            for position in Position::ALL {
                for stats in [&wild, &awful] {
                    let r = rated(model, *position, *stats, TeamResult::Loss);
                    assert!((RATING_RANGE.0..=RATING_RANGE.1).contains(&r), "{model:?} {position:?}: {r}");
                }
            }
            let input = RatingInput { position: St, overall: 70, minutes: MIN_RATED_MINUTES - 1, stats: &wild, result: TeamResult::Draw };
            assert_eq!(rate(model, &input), None, "{model:?}");
        }
    }

    #[test]
    fn every_model_pays_for_goals_and_assists_and_charges_for_cards() {
        for &model in RatingModel::ALL {
            for position in [Gk, Cb, Cm, Lw, St] {
                let plain = rated(model, position, with(|_| {}), TeamResult::Draw);
                assert!(rated(model, position, with(|s| s.goals = 1), TeamResult::Draw) > plain, "{model:?} {position:?} goal");
                assert!(rated(model, position, with(|s| s.assists = 1), TeamResult::Draw) > plain, "{model:?} {position:?} assist");
                assert!(rated(model, position, with(|s| s.yellows = 1), TeamResult::Draw) < plain, "{model:?} {position:?} yellow");
                assert!(rated(model, position, with(|s| { s.reds = 1; s.minutes = 60 }), TeamResult::Draw) < plain, "{model:?} {position:?} red");
            }
        }
    }

    #[test]
    fn every_model_rates_a_win_above_a_draw_above_a_defeat_for_the_same_play() {
        for &model in RatingModel::ALL {
            for position in Position::ALL {
                let r = |result| rated(model, *position, with(|_| {}), result);
                assert!(r(TeamResult::Win) > r(TeamResult::Draw) && r(TeamResult::Draw) > r(TeamResult::Loss), "{model:?} {position:?}");
            }
        }
    }

    #[test]
    fn a_better_player_rates_higher_for_the_same_match_with_the_overall_based_model_and_the_same_with_the_formula() {
        let stats = with(|s| s.goals = 1);
        let at = |model, overall| {
            let input = RatingInput { position: St, overall, minutes: 90, stats: &stats, result: TeamResult::Win };
            rate(model, &input).unwrap()
        };
        assert!(at(RatingModel::OverallBased, 90) > at(RatingModel::OverallBased, 60) + 0.8);
        assert_eq!(at(RatingModel::Formula, 90), at(RatingModel::Formula, 60), "the formula model ignores overall");
    }
}
