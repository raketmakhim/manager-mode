//! The match-based experience model: every match a player plays gives him 1 to 10 experience points (XP), more for
//! playing the whole match, for playing well and for being the best player, and less the higher his overall is. Each
//! time his XP bar fills (`XP_PER_LEVEL`) he levels up: his overall rises by a number of points that is smaller for an
//! older player and, past his potential, smaller again (a soft limit: a winning player keeps rising, slowly). Every
//! number is our own design, tuned with `dev-tools/simtool`; there is no source (`research/game-features/experience.md`).
use super::{Candidate, MatchPlay};
use crate::model::Line;

/// What a match gives before the overall factor (the most is 1 + 4 + 3 + 2 = 10): turning up, the minutes played (4 for
/// the whole match), form (3 at full form) and being the player of the match.
const APPEARANCE_XP: f32 = 1.0;
const MINUTES_XP: f32 = 4.0;
const FORM_XP: f32 = 3.0;
const PLAYER_OF_THE_MATCH_XP: f32 = 2.0;
/// The XP of a match is between these.
const XP_RANGE: (f32, f32) = (1.0, 10.0);
/// The factor on a match's XP for a given overall: 1 up to 54, falling to the floor at 90 and over (a 90 gets about
/// a third of what a 60 does).
const FACTOR_AT_ZERO: f32 = 99.0;
const FACTOR_DIVISOR: f32 = 45.0;
const FACTOR_RANGE: (f32, f32) = (0.3, 1.0);
/// One rating point above or below what a regular of his line normally gets is full form (or none).
const FORM_SCALE: f32 = 1.0;
/// Overall points one level gives a player of 21 or younger who is below his potential.
const LEVEL_POINTS: f32 = 1.0;
/// Past his potential, a level is worth exp(-points over / this) of that: 6 over is a third, 12 over a seventh.
const OVER_POTENTIAL_SCALE: f32 = 6.0;
/// Reaction to levels by age: full at this age or younger, falling to the floor by `OLD_AGE`.
const YOUNG_AGE: f32 = 21.0;
const OLD_AGE: f32 = 33.0;
const OLD_FLOOR: f32 = 0.25;

/// How far above an uneventful match a regular's average rating is (goals, assists, clean sheets and wins), measured
/// over a simulated season of FIFA 15 (5300 players with 2400 minutes or more): forwards and goalkeepers about a third
/// of a point, midfielders and defenders much less.
fn expected_rating_above_neutral(line: Line) -> f32 {
    match line {
        Line::Attack => 0.33,
        Line::Goalkeeper => 0.32,
        Line::Midfield => 0.16,
        Line::Defence => 0.12,
    }
}

/// Less XP for a higher overall.
fn overall_factor(overall: u8) -> f32 {
    ((FACTOR_AT_ZERO - overall as f32) / FACTOR_DIVISOR).clamp(FACTOR_RANGE.0, FACTOR_RANGE.1)
}

/// The XP of one match, 1 to 10 (0 if he did not play).
pub(super) fn match_xp(play: &MatchPlay) -> u8 {
    if play.minutes == 0 {
        return 0;
    }
    let share = (play.minutes as f32 / 90.0).min(1.0);
    // A player with too few minutes to be rated has middling form.
    let form = play.rating.map_or(0.5, |r| ((r - play.neutral_rating - expected_rating_above_neutral(play.line)) / FORM_SCALE + 0.5).clamp(0.0, 1.0));
    let raw = APPEARANCE_XP + MINUTES_XP * share + FORM_XP * form + PLAYER_OF_THE_MATCH_XP * play.player_of_the_match as u8 as f32;
    (raw * overall_factor(play.overall)).round().clamp(XP_RANGE.0, XP_RANGE.1) as u8
}

fn youth(age: u8) -> f32 {
    ((OLD_AGE - age as f32) / (OLD_AGE - YOUNG_AGE)).clamp(OLD_FLOOR, 1.0)
}

/// Overall points one level gives him.
pub(super) fn level_points(who: &Candidate) -> f32 {
    let over = (who.overall as f32 - who.potential as f32).max(0.0);
    LEVEL_POINTS * youth(who.age) * (-over / OVER_POTENTIAL_SCALE).exp()
}
