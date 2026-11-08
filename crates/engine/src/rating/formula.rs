//! The formula rating model: a base rating plus bonuses and penalties for what the player did, scaled by position
//! (and, for clean sheets and goals against, by the share of the match he played), plus a small term for the team's
//! result. The weights are our own design (see `research/game-features/match-ratings.md`: no provider publishes
//! theirs) and are tuned to the shape of real ratings in the test of `stats.rs` that plays a season and looks at
//! the spread.
use super::{RatingInput, TeamResult};
use crate::model::Line;

/// What an uneventful player of a drawn match gets.
const BASE: f32 = 6.3;
/// A goal. Worth more to a defender or midfielder, for whom it is rarer.
const GOAL: f32 = 1.2;
/// An assist.
const ASSIST: f32 = 0.9;
/// A shot on target that was not a goal: a small plus for the attempt.
const SHOT_ON_TARGET: f32 = 0.06;
/// A save by the goalkeeper.
const SAVE: f32 = 0.15;
/// A yellow card and a red card.
const YELLOW: f32 = -0.3;
const RED: f32 = -1.5;
/// The team's result, for everyone (a win lifts the whole side).
const WIN: f32 = 0.4;
const LOSS: f32 = -0.4;

/// A goal counts this many times its value for a player of this line.
fn goal_scale(line: Line) -> f32 {
    match line {
        Line::Goalkeeper | Line::Defence => 1.4,
        Line::Midfield => 1.15,
        Line::Attack => 0.9,
    }
}

/// An assist counts this many times its value.
fn assist_scale(line: Line) -> f32 {
    match line {
        Line::Defence => 1.2,
        _ => 1.0,
    }
}

/// A goal against costs this much, and a clean sheet gives this much, to a player of this line. Forwards are not
/// blamed or thanked for what the back line did.
fn conceded_penalty(line: Line) -> f32 {
    match line {
        Line::Goalkeeper => 0.25,
        Line::Defence => 0.1,
        Line::Midfield => 0.05,
        Line::Attack => 0.0,
    }
}

fn clean_sheet_bonus(line: Line) -> f32 {
    match line {
        Line::Goalkeeper => 0.8,
        Line::Defence => 1.1,
        Line::Midfield => 0.15,
        Line::Attack => 0.0,
    }
}

/// The base rating plus everything the player did and the team's result.
pub(super) fn rate(input: &RatingInput) -> f32 {
    BASE + events_score(input)
}

/// What the player's match was worth in rating points, above or below nothing happening: his events, what happened
/// behind him and the team's result. Shared with the models that start from something other than `BASE`.
pub(super) fn events_score(input: &RatingInput) -> f32 {
    let (s, line) = (input.stats, input.position.line());
    let missed_on_target = s.shots_on_target.saturating_sub(s.goals) as f32;
    // What he did counts in full, however long he played.
    let did = s.goals as f32 * GOAL * goal_scale(line)
        + s.assists as f32 * ASSIST * assist_scale(line)
        + missed_on_target * SHOT_ON_TARGET
        + s.saves as f32 * SAVE
        + s.yellows as f32 * YELLOW
        + s.reds as f32 * RED;
    // What happened behind him only counts for the part of the match he was there.
    let share = input.minutes.min(90) as f32 / 90.0;
    let at_the_back = s.clean_sheets as f32 * clean_sheet_bonus(line) - s.conceded as f32 * conceded_penalty(line);
    let result = match input.result {
        TeamResult::Win => WIN,
        TeamResult::Draw => 0.0,
        TeamResult::Loss => LOSS,
    };
    did + at_the_back * share + result
}
