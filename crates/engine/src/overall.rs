//! How a player's overall rating is built from his attributes: a weighted average whose weights depend on his main
//! position. The weights are a fit to the real data (FC 24), see `research/game-features/overall-rating-formula.md`.
//! The engine only uses them for a change: when an attribute moves, the overall moves by weight x change.
use crate::attrs::Attr::{self, *};
use crate::model::Position;

/// The attributes that count towards a position's overall, with their weights (a share of 1: the weights of a
/// position add up to about 1). Attributes not listed count for nothing.
#[rustfmt::skip]
pub fn weights(position: Position) -> &'static [(Attr, f32)] {
    match position {
        Position::Gk => &[
            (GkDiving, 0.214),
            (GkHandling, 0.211),
            (GkReflexes, 0.210),
            (GkPositioning, 0.209),
            (Reactions, 0.107),
            (GkKicking, 0.051),
        ],

        Position::Lwb => &[
            (Crossing, 0.129),
            (Interceptions, 0.124),
            (SlidingTackle, 0.097),
            (StandingTackle, 0.096),
            (Stamina, 0.095),
            (ShortPassing, 0.091),
            (BallControl, 0.079),
            (Reactions, 0.079),
            (Marking, 0.064),
            (SprintSpeed, 0.060),
            (Dribbling, 0.041),
            (Acceleration, 0.040),
        ],

        Position::Lb => &[
            (SlidingTackle, 0.137),
            (Interceptions, 0.119),
            (StandingTackle, 0.117),
            (Crossing, 0.092),
            (Reactions, 0.084),
            (Marking, 0.080),
            (Stamina, 0.079),
            (BallControl, 0.074),
            (ShortPassing, 0.067),
            (SprintSpeed, 0.064),
            (Acceleration, 0.053),
            (Heading, 0.035),
        ],

        Position::Cb => &[
            (StandingTackle, 0.181),
            (Marking, 0.138),
            (Interceptions, 0.130),
            (SlidingTackle, 0.103),
            (Strength, 0.082),
            (Heading, 0.078),
            (Aggression, 0.067),
            (Jumping, 0.057),
            (Reactions, 0.056),
            (ShortPassing, 0.048),
            (BallControl, 0.044),
        ],

        Position::Rb => &[
            (SlidingTackle, 0.131),
            (StandingTackle, 0.125),
            (Interceptions, 0.110),
            (Crossing, 0.094),
            (Reactions, 0.090),
            (Marking, 0.080),
            (Stamina, 0.077),
            (BallControl, 0.072),
            (SprintSpeed, 0.068),
            (ShortPassing, 0.067),
            (Acceleration, 0.049),
            (Heading, 0.038),
        ],

        Position::Rwb => &[
            (Crossing, 0.123),
            (Interceptions, 0.113),
            (SlidingTackle, 0.103),
            (ShortPassing, 0.096),
            (Stamina, 0.095),
            (StandingTackle, 0.094),
            (Reactions, 0.090),
            (BallControl, 0.079),
            (Marking, 0.071),
            (SprintSpeed, 0.058),
            (Acceleration, 0.043),
            (Dribbling, 0.036),
        ],

        Position::Lm => &[
            (Dribbling, 0.150),
            (BallControl, 0.137),
            (ShortPassing, 0.107),
            (Crossing, 0.098),
            (Positioning, 0.086),
            (Reactions, 0.070),
            (Acceleration, 0.065),
            (Vision, 0.065),
            (SprintSpeed, 0.060),
            (Finishing, 0.060),
            (LongPassing, 0.049),
            (Stamina, 0.048),
        ],

        Position::Cdm => &[
            (ShortPassing, 0.145),
            (Interceptions, 0.134),
            (StandingTackle, 0.129),
            (BallControl, 0.103),
            (LongPassing, 0.095),
            (Marking, 0.087),
            (Reactions, 0.075),
            (Stamina, 0.059),
            (Aggression, 0.047),
            (SlidingTackle, 0.047),
            (Strength, 0.039),
            (Vision, 0.037),
        ],

        Position::Cm => &[
            (ShortPassing, 0.173),
            (BallControl, 0.145),
            (LongPassing, 0.126),
            (Vision, 0.123),
            (Reactions, 0.090),
            (Dribbling, 0.068),
            (Positioning, 0.060),
            (Stamina, 0.059),
            (StandingTackle, 0.051),
            (Interceptions, 0.049),
            (LongShots, 0.037),
            (Finishing, 0.021),
        ],

        Position::Cam => &[
            (ShortPassing, 0.164),
            (BallControl, 0.145),
            (Vision, 0.140),
            (Dribbling, 0.132),
            (Positioning, 0.094),
            (Reactions, 0.073),
            (Finishing, 0.072),
            (LongShots, 0.047),
            (Acceleration, 0.039),
            (LongPassing, 0.037),
            (SprintSpeed, 0.031),
            (Agility, 0.029),
        ],

        Position::Rm => &[
            (BallControl, 0.144),
            (Dribbling, 0.141),
            (ShortPassing, 0.109),
            (Crossing, 0.099),
            (Positioning, 0.087),
            (Reactions, 0.073),
            (Acceleration, 0.071),
            (Vision, 0.062),
            (Finishing, 0.061),
            (SprintSpeed, 0.056),
            (Stamina, 0.049),
            (LongPassing, 0.046),
        ],

        Position::Lw => &[
            (BallControl, 0.155),
            (Dribbling, 0.148),
            (Finishing, 0.116),
            (Positioning, 0.091),
            (Crossing, 0.089),
            (ShortPassing, 0.085),
            (Reactions, 0.070),
            (Acceleration, 0.066),
            (Vision, 0.066),
            (SprintSpeed, 0.061),
            (LongShots, 0.033),
            (Agility, 0.027),
        ],

        Position::Cf => &[
            (BallControl, 0.174),
            (Dribbling, 0.129),
            (Positioning, 0.125),
            (Finishing, 0.120),
            (ShortPassing, 0.097),
            (Reactions, 0.089),
            (Vision, 0.082),
            (Acceleration, 0.055),
            (ShotPower, 0.052),
            (SprintSpeed, 0.046),
            (LongShots, 0.025),
            (Heading, 0.025),
        ],

        Position::St => &[
            (Finishing, 0.193),
            (Positioning, 0.137),
            (BallControl, 0.106),
            (ShotPower, 0.102),
            (Heading, 0.098),
            (Reactions, 0.085),
            (Dribbling, 0.066),
            (ShortPassing, 0.055),
            (SprintSpeed, 0.051),
            (Strength, 0.047),
            (Acceleration, 0.036),
            (LongShots, 0.030),
        ],

        Position::Rw => &[
            (Dribbling, 0.156),
            (BallControl, 0.142),
            (Positioning, 0.103),
            (Finishing, 0.099),
            (ShortPassing, 0.096),
            (Crossing, 0.088),
            (Acceleration, 0.073),
            (Reactions, 0.072),
            (SprintSpeed, 0.055),
            (Vision, 0.052),
            (LongShots, 0.037),
            (Agility, 0.030),
        ],
    }
}

/// How many overall points a player of this position gains or loses when his attributes change by `changes`
/// (points of attribute, negative for a drop). Fractional: +2 finishing is about +0.4 for a striker.
pub fn overall_change(position: Position, changes: &[(Attr, i8)]) -> f32 {
    let weight = |attr: Attr| weights(position).iter().find(|(a, _)| *a == attr).map_or(0.0, |&(_, w)| w);
    changes.iter().map(|&(attr, change)| weight(attr) * change as f32).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::load_edition;
    use rusqlite::Connection;

    #[test]
    fn every_position_has_weights_that_are_positive_and_add_up_to_about_one() {
        for &position in Position::ALL {
            let w = weights(position);
            assert!(w.iter().all(|&(_, weight)| weight >= 0.02), "{position:?} has a tiny or negative weight");
            let sum: f32 = w.iter().map(|&(_, weight)| weight).sum();
            assert!((0.95..=1.05).contains(&sum), "{position:?} weights add up to {sum}");
        }
    }

    #[test]
    fn an_attribute_matters_for_the_positions_that_use_it() {
        let finishing = [(Attr::Finishing, 2)];
        assert!(overall_change(Position::St, &finishing) > 0.3);
        assert_eq!(overall_change(Position::Cb, &finishing), 0.0);
        assert!(overall_change(Position::Cb, &[(Attr::StandingTackle, 2)]) > 0.3);
        assert!(overall_change(Position::St, &[(Attr::Finishing, -2)]) < -0.3, "a drop lowers the overall");
        assert_eq!(overall_change(Position::St, &[]), 0.0);
    }

    /// The real test: take two real editions, predict each player's new overall from how his attributes changed, and
    /// compare it with the real new overall.
    #[test]
    fn the_formula_predicts_the_real_change_in_overall_between_editions() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../history.db");
        if !std::path::Path::new(path).exists() { return; } // needs `cargo run -p import` first
        let conn = Connection::open(path).unwrap();
        let (before, after) = (load_edition(&conn, 23).unwrap(), load_edition(&conn, 24).unwrap());
        let (mut compared, mut close) = (0, 0);
        for (id, was) in &before.players {
            let Some(now) = after.players.get(id) else { continue };
            let (Some(&position), Some(&same)) = (was.positions.first(), now.positions.first()) else { continue };
            if position != same {
                continue;
            }
            let changes: Vec<(Attr, i8)> = Attr::ALL
                .iter()
                .filter_map(|&a| Some((a, now.attrs[a]? as i8 - was.attrs[a]? as i8)))
                .collect();
            if changes.len() < Attr::ALL.len() {
                continue; // an attribute is missing in one edition
            }
            let predicted = was.overall as f32 + overall_change(position, &changes);
            compared += 1;
            close += ((predicted.round() - now.overall as f32).abs() <= 1.0) as usize;
        }
        assert!(compared > 5000, "only {compared} players compared");
        let share = close as f32 / compared as f32;
        assert!(share > 0.98, "only {:.1}% of {compared} predictions are within 1 point", share * 100.0);
    }
}
