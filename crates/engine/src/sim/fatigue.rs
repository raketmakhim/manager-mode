//! Tiring during a match: how tired a player is after some minutes on the pitch, and what that does to his rating in
//! his slot. A fatigue model is a pure function from the player, the minutes he has played and his side's share of the
//! ball; the match model calls it through `FatigueModel`. Between matches (rest, fitness, injuries) is not here.
//! Adding a model: a `FatigueModel` variant and an arm in `tiredness`; the contract tests below run for it through
//! `FatigueModel::ALL`.
//!
//! - `None`: nobody tires.
//! - `ByStamina`: tiredness grows with the minutes played, faster for a player with low stamina, for a midfielder or
//!   wide player than for a centre-back, and a little faster for the side without the ball.
//!
//! Work rate is not used (see `research/game-features/fitness.md`): stamina alone for now.
use super::MatchPlayer;
use crate::model::Position;

/// Which model decides how tired the players get.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FatigueModel {
    /// Nobody tires.
    None,
    /// Tiredness from stamina, position, minutes played and the share of the ball.
    ByStamina,
}

impl FatigueModel {
    /// Every model, so a test can run them all.
    pub const ALL: &'static [FatigueModel] = &[FatigueModel::None, FatigueModel::ByStamina];
}

/// The model every match uses.
pub const DEFAULT_FATIGUE_MODEL: FatigueModel = FatigueModel::ByStamina;

// Real running falls by roughly 8-18% from the start to the end of a match once the stoppages are taken out, more in
// the last 15 minutes, and by position and possession as in `research/game-features/fitness.md`. Nothing maps that to
// a rating, so every number here is our own guess, to be tuned with the simtool: the match must score as many goals
// and the second half as many as before, because an average side tiring the same as the other changes nothing.

/// Tiredness (0 fresh to 1 spent) of a midfielder with the reference stamina who has played 90 minutes with half the ball.
const TIRED_AT_90: f64 = 0.5;
/// The stamina that counts as average.
const REFERENCE_STAMINA: f64 = 70.0;
/// Each stamina point above the reference cuts tiredness by this share (as an exponent): a 90 tires about 0.6 times
/// as fast as a 70, a 50 about 1.6 times.
const STAMINA_PER_POINT: f64 = 0.025;
/// Tiredness grows with the minutes played to this power: slowly at first and faster late, as real running does.
const BUILD_UP: f64 = 1.5;
/// Rating points a player loses when fully spent.
const RATING_LOSS_AT_SPENT: f64 = 10.0;
/// Each 10 points of ball share below half adds this share to the tiredness (and above half takes it off): the side
/// without the ball does more of the sprinting. Modest, since total distance is the same whoever has the ball.
const NO_BALL_EXTRA_PER_TEN_POINTS: f64 = 0.06;

/// How much of a 90-minute run a player of this slot does against a central midfielder: wide players and full-backs
/// sprint most, centre-backs run least, the goalkeeper does not tire.
fn position_factor(position: Position) -> f64 {
    use Position::*;
    match position {
        Gk => 0.0,
        Cb => 0.85,
        Cf | St => 1.0,
        Cdm | Cm | Cam | Lm | Rm | Lw | Rw | Lwb | Rwb | Lb | Rb => 1.1,
    }
}

/// How tired the player is, from 0 (fresh) to 1, after `minutes_on` minutes on the pitch, when his side has
/// `ball_share` (0 to 1) of the ball.
pub(super) fn tiredness(model: FatigueModel, player: &MatchPlayer, minutes_on: u8, ball_share: f64) -> f64 {
    match model {
        FatigueModel::None => 0.0,
        FatigueModel::ByStamina => {
            let stamina = (STAMINA_PER_POINT * (REFERENCE_STAMINA - player.stamina as f64)).exp();
            let ball = 1.0 + NO_BALL_EXTRA_PER_TEN_POINTS * (0.5 - ball_share) * 10.0;
            let run = (minutes_on as f64 / 90.0).powf(BUILD_UP);
            (TIRED_AT_90 * position_factor(player.position) * stamina * ball * run).clamp(0.0, 1.0)
        }
    }
}

/// The player's rating in his slot (0-99) once tired.
pub(super) fn tired_rating(model: FatigueModel, player: &MatchPlayer, minutes_on: u8, ball_share: f64) -> u8 {
    let loss = RATING_LOSS_AT_SPENT * tiredness(model, player, minutes_on, ball_share);
    (player.rating as f64 - loss).round().max(0.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::MatchTeam;

    fn player(position: Position, stamina: u8) -> MatchPlayer {
        MatchPlayer { position, stamina, ..MatchTeam::flat(70).players[5] }
    }

    // Rules every model must keep; a new model is added to `FatigueModel::ALL` and these run for it too.

    #[test]
    fn every_model_keeps_tiredness_between_zero_and_one_and_never_raises_a_rating() {
        for &model in FatigueModel::ALL {
            for position in Position::ALL {
                for stamina in [0, 40, 99] {
                    for minutes in [0, 45, 90, 120] {
                        let p = player(*position, stamina);
                        let t = tiredness(model, &p, minutes, 0.3);
                        assert!((0.0..=1.0).contains(&t), "{model:?} {position:?}: {t}");
                        assert!(tired_rating(model, &p, minutes, 0.3) <= p.rating, "{model:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn every_model_starts_a_player_fresh_and_never_makes_him_less_tired_as_the_match_goes_on() {
        for &model in FatigueModel::ALL {
            let p = player(Position::Cm, 60);
            assert_eq!(tiredness(model, &p, 0, 0.5), 0.0, "{model:?}");
            let by_minute: Vec<f64> = (0..=120).map(|m| tiredness(model, &p, m, 0.5)).collect();
            assert!(by_minute.windows(2).all(|w| w[0] <= w[1]), "{model:?}");
        }
    }

    #[test]
    fn every_model_leaves_the_goalkeeper_fresh() {
        for &model in FatigueModel::ALL {
            assert_eq!(tiredness(model, &player(Position::Gk, 10), 120, 0.2), 0.0, "{model:?}");
        }
    }

    // Rules of the stamina model.

    #[test]
    fn a_player_with_more_stamina_tires_more_slowly() {
        let at = |stamina| tiredness(FatigueModel::ByStamina, &player(Position::Cm, stamina), 90, 0.5);
        assert!(at(90) < 0.7 * at(70) && at(70) < 0.7 * at(50), "{} {} {}", at(90), at(70), at(50));
    }

    #[test]
    fn centre_backs_tire_less_than_wide_players_and_the_side_without_the_ball_tires_more() {
        let t = |position, ball| tiredness(FatigueModel::ByStamina, &player(position, 70), 90, ball);
        assert!(t(Position::Cb, 0.5) < t(Position::St, 0.5) && t(Position::St, 0.5) < t(Position::Lw, 0.5));
        assert!(t(Position::Cm, 0.35) > t(Position::Cm, 0.5) && t(Position::Cm, 0.5) > t(Position::Cm, 0.65));
    }

    #[test]
    fn tiredness_builds_up_faster_late_in_the_match() {
        let t = |m| tiredness(FatigueModel::ByStamina, &player(Position::Cm, 70), m, 0.5);
        assert!(t(90) - t(60) > t(30) - t(0), "the last 30 minutes tire more than the first 30");
    }
}
