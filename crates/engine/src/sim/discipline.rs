//! Fouls and cards: who commits a foul and whether it is booked. A discipline model is a pure function from the
//! lineup, who is already on a yellow card and a seeded RNG to a `Foul`; the match model calls it through
//! `DisciplineModel`. How many fouls a side commits is `FOULS_PER_SIDE`, the same for every model.
//! Adding a model: a `DisciplineModel` variant and an arm in `foul_weight`; the contract tests below run for it
//! through `DisciplineModel::ALL`.
//!
//! - `Even`: every outfield player is equally likely to commit a foul.
//! - `ByAggression`: the aggressive ones commit most of them, so careful players are rarely booked.
use super::pick::pick;
use super::MatchPlayer;
use crate::model::Position;
use crate::rng::Rng;

/// Which model decides who fouls and who is carded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisciplineModel {
    /// Every outfield player fouls equally often.
    Even,
    /// A player's share of his side's fouls follows his aggression attribute.
    ByAggression,
}

impl DisciplineModel {
    /// Every model, so a test can run them all.
    pub const ALL: &'static [DisciplineModel] = &[DisciplineModel::Even, DisciplineModel::ByAggression];
}

/// The model every match uses.
pub const DEFAULT_DISCIPLINE_MODEL: DisciplineModel = DisciplineModel::ByAggression;

// Tuned with `dev-tools/simtool` against the real big-five figures (fouls 25.3, yellow 4.10, red 0.20 per match) and
// the card records of Messi and Ronaldo: see `research/game-features/simulation-balance.md`.

/// Fouls a side commits in 90 minutes.
pub(super) const FOULS_PER_SIDE: f64 = 12.65;
/// Chance a foul is booked, and chance it is a straight red card.
const YELLOW_PER_FOUL: f64 = 0.18;
const RED_PER_FOUL: f64 = 0.004;
/// Each point of aggression multiplies a player's share of his side's fouls by this (as an exponent): a 75 commits
/// about 2.5 times the fouls of a 50. Real Messi is booked about 0.09 times a match and Ronaldo 0.125, against about
/// 0.2 for an average outfield player.
const FOULS_PER_AGGRESSION_POINT: f64 = 0.037;
/// No player commits more than this share of his side's fouls.
const MAX_FOUL_SHARE: f64 = 0.2;
/// A player already on a yellow card is booked this share as often: he plays more carefully. Without it second
/// yellows make red cards about two and a half times as common as in real football.
const BOOKED_CAUTION: f64 = 0.25;

/// What the referee does about a foul.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Card {
    None,
    Yellow,
    /// A straight red, or the second yellow of a player already booked.
    Red,
}

/// One foul.
#[derive(Clone, Copy, Debug)]
pub(super) struct Foul {
    /// The lineup slot of the player who committed it.
    pub slot: usize,
    pub card: Card,
}

/// A foul by one of `players`; `booked[slot]` is true for a player already on a yellow card.
pub(super) fn foul(model: DisciplineModel, players: &[MatchPlayer; 11], booked: &[bool; 11], rng: &mut Rng) -> Foul {
    let slot = pick(players, |p| foul_weight(model, p), MAX_FOUL_SHARE, rng);
    let caution = if booked[slot] { BOOKED_CAUTION } else { 1.0 };
    let card = if rng.unit() < YELLOW_PER_FOUL * caution {
        if booked[slot] { Card::Red } else { Card::Yellow }
    } else if rng.unit() < RED_PER_FOUL {
        Card::Red
    } else {
        Card::None
    };
    Foul { slot, card }
}

/// How likely a player is to commit a foul. The goalkeeper and an empty slot never do.
fn foul_weight(model: DisciplineModel, p: &MatchPlayer) -> f64 {
    if p.is_empty() || p.position == Position::Gk {
        return 0.0;
    }
    match model {
        DisciplineModel::Even => 1.0,
        DisciplineModel::ByAggression => (FOULS_PER_AGGRESSION_POINT * p.aggression as f64).exp(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::MatchTeam;

    // Rules every model must keep; a new model is added to `DisciplineModel::ALL` and these run for it too.

    #[test]
    fn every_model_keeps_the_goalkeeper_and_empty_slots_out_of_fouls() {
        for &model in DisciplineModel::ALL {
            let mut team = MatchTeam::flat(70);
            team.players[10] = MatchPlayer::empty(Position::St);
            let mut rng = Rng::new(4);
            for _ in 0..2000 {
                let slot = foul(model, &team.players, &[false; 11], &mut rng).slot;
                assert!(slot != 0 && slot != 10, "{model:?}: slot {slot} fouled");
            }
        }
    }

    #[test]
    fn every_model_books_a_booked_player_less_often_and_turns_his_yellow_into_a_red() {
        for &model in DisciplineModel::ALL {
            let team = MatchTeam::flat(70);
            let mut rng = Rng::new(5);
            let count = |booked: [bool; 11], rng: &mut Rng| {
                let fouls: Vec<Foul> = (0..20_000).map(|_| foul(model, &team.players, &booked, rng)).collect();
                let in_trouble = |c: Card| fouls.iter().filter(|f| f.card == c).count();
                (in_trouble(Card::Yellow), in_trouble(Card::Red))
            };
            let (fresh_yellows, fresh_reds) = count([false; 11], &mut rng);
            let (cautious_yellows, cautious_reds) = count([true; 11], &mut rng);
            assert!(fresh_yellows > 0 && fresh_reds < fresh_yellows, "{model:?}");
            assert_eq!(cautious_yellows, 0, "{model:?}: a booked player's next booking is a red");
            assert!(cautious_reds > fresh_reds, "{model:?}");
        }
    }

    // Rules of the aggression model.

    #[test]
    fn aggressive_players_foul_more_in_the_aggression_model_and_no_one_takes_more_than_the_share_limit() {
        let mut team = MatchTeam::flat(70);
        team.players[5].aggression = 90;
        team.players[6].aggression = 30;
        let mut rng = Rng::new(6);
        let mut fouls = [0u32; 11];
        for _ in 0..5000 {
            fouls[foul(DisciplineModel::ByAggression, &team.players, &[false; 11], &mut rng).slot] += 1;
        }
        assert!(fouls[5] > 3 * fouls[6], "{} fouls against {}", fouls[5], fouls[6]);
        assert!(fouls[5] as f64 / 5000.0 <= MAX_FOUL_SHARE + 0.03);
        assert_eq!(foul_weight(DisciplineModel::Even, &team.players[5]), foul_weight(DisciplineModel::Even, &team.players[6]));
    }
}
