//! The overall-based rating model: the player's level sets where his rating starts, and what happened in the match
//! moves it from there. A 90-rated player has a higher rating than a 60-rated one for the same match, and a star needs
//! a quiet game to fall to the middle. The events part is the same as in `formula`, only smaller, so the two models
//! stay comparable. The numbers are our own design.
use super::formula::events_score;
use super::RatingInput;

/// The rating of a player at the reference level in a match where nothing happens.
const BASE: f32 = 6.3;
/// The rating (0-99) that counts as average: about the mean starter of every league in the world, so the average
/// rating of a whole world season is about 6.5.
const REFERENCE_OVERALL: f32 = 67.0;
/// Rating points per point of overall: a 91 starts 0.7 above a 67, a 43 starts 0.7 below.
const PER_OVERALL_POINT: f32 = 0.03;
/// How much of the match's events counts, against `formula`'s full count.
const EVENTS_SHARE: f32 = 0.8;

pub(super) fn rate(input: &RatingInput) -> f32 {
    BASE + PER_OVERALL_POINT * (input.overall as f32 - REFERENCE_OVERALL) + EVENTS_SHARE * events_score(input)
}
