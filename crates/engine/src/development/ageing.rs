//! The ageing model `ByAge`: how much a player's overall changes over one summer. The mean is a table by age (young
//! players grow, from about 31 they slowly decline, faster from 33 and a lot after 35), a young player grows more the
//! further he is below his potential and little past it, and a goalkeeper ages two years later than an outfield player.
//! Every number is our own design, fitted by eye to the measured averages of `research/game-features/experience.md`
//! (section 2) but declining faster than the data does, because that data leaves out the players who dropped out of it.
use super::Candidate;

/// The mean change in overall over a summer for a player who is this age in the new season, from 16: (age, change).
/// Younger than the first row counts as the first; older than the last counts as the last.
const MEAN_CHANGE: [(u8, f32); 23] = [
    (16, 3.0),
    (17, 3.0),
    (18, 2.8),
    (19, 2.5),
    (20, 2.2),
    (21, 1.8),
    (22, 1.4),
    (23, 1.1),
    (24, 0.8),
    (25, 0.5),
    (26, 0.3),
    (27, 0.2),
    (28, 0.0),
    (29, 0.0),
    (30, 0.0),
    (31, -0.5),
    (32, -0.8),
    (33, -1.3),
    (34, -1.7),
    (35, -2.4),
    (36, -3.0),
    (37, -3.5),
    (38, -4.0),
];
/// A goalkeeper changes as an outfield player this many years younger.
const KEEPER_LATER_BY: u8 = 2;
/// Growth is this share of the table's with no gap to potential, plus `GROWTH_PER_GAP_POINT` of it for each point of gap
/// (so 1.0 with a gap of 10), up to the most.
const GROWTH_AT_NO_GAP: f32 = 0.6;
const GROWTH_PER_GAP_POINT: f32 = 0.04;
const MOST_GROWTH: f32 = 1.4;
/// Past his potential growth falls with exp(points over / this), as a level of experience does.
const OVER_POTENTIAL_SCALE: f32 = 6.0;

/// The mean change (in overall points, can be negative) of one summer for this player.
pub(super) fn mean_change(who: &Candidate) -> f32 {
    let age = if who.keeper { who.age.saturating_sub(KEEPER_LATER_BY) } else { who.age };
    let row = MEAN_CHANGE.iter().rev().find(|&&(from, _)| age >= from).unwrap_or(&MEAN_CHANGE[0]);
    let table = row.1;
    if table <= 0.0 {
        return table; // decline does not depend on potential
    }
    let gap = who.potential as f32 - who.overall as f32;
    let share = if gap >= 0.0 { (GROWTH_AT_NO_GAP + GROWTH_PER_GAP_POINT * gap).min(MOST_GROWTH) } else { GROWTH_AT_NO_GAP * (gap / OVER_POTENTIAL_SCALE).exp() };
    table * share
}
