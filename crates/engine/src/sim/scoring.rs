//! Who shoots, whether the shot is a goal and who sets it up: the scoring part of the shot-based match model. A
//! scoring model is a set of pure functions from the lineup (and a seeded RNG) to a choice; the match model calls
//! them through `ScoringModel`. How many shots a side gets is not decided here (that is possession and strength).
//! Adding a model: a `ScoringModel` variant and an arm in each function; the compiler lists the places, and the
//! contract tests below run for it through `ScoringModel::ALL`.
//!
//! - `Flat`: every player of a slot is the same; only the position matters.
//! - `StarWeighted`: finishing decides who shoots and how often it goes in, creativity who assists, the keeper's
//!   rating how often he is beaten.
use super::pick::pick;
use super::MatchPlayer;
use crate::model::Position;
use crate::rng::Rng;

/// Which model decides who shoots, scores and assists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScoringModel {
    /// Position only: every striker is as likely to shoot, score and assist as every other striker.
    Flat,
    /// Position, raised by the player's finishing (shots, conversion) and creativity (assists), and the keeper's rating.
    StarWeighted,
}

impl ScoringModel {
    /// Every model, so a test can run them all.
    pub const ALL: &'static [ScoringModel] = &[ScoringModel::Flat, ScoringModel::StarWeighted];
}

/// The model every match uses.
pub const DEFAULT_SCORING_MODEL: ScoringModel = ScoringModel::StarWeighted;

// Tuned with `dev-tools/simtool` against the real big-five tables and the best scorers and assisters of
// `research/game-features/simulation-balance.md`. Rerun the simtool after changing any of them.

/// Chance a shot on target is a goal, for a shooter with the reference finishing against a keeper of the reference rating.
const GOAL_PER_SHOT_ON_TARGET: f64 = 0.303;
/// The finishing and goalkeeper rating that count as average.
const REFERENCE_RATING: f64 = 70.0;
/// Each point of the shooter's finishing above the reference changes that chance by this share (exponent), within
/// `FINISHING_FACTOR_RANGE`. It is his own finishing, not the gap to the keeper: a striker in a weak league does not
/// score like an elite one just because the keepers there are weak.
const FINISHING_PER_POINT: f64 = 0.03;
const FINISHING_FACTOR_RANGE: (f64, f64) = (0.6, 2.0);
/// The same for each point the goalkeeper's rating is below the reference.
const KEEPER_PER_POINT: f64 = 0.012;
const KEEPER_FACTOR_RANGE: (f64, f64) = (0.8, 1.25);
/// Each point of finishing multiplies a player's share of his side's shots by this (as an exponent): a 92 takes
/// about 3 times the shots of a 70 in the same slot. Real stars take far more shots than a flat weight gives.
const SHOTS_PER_FINISHING_POINT: f64 = 0.05;
/// No player takes more than this share of his side's shots, however good he is and however weak the rest are (the
/// best real strikers take about a quarter). Without a limit a lone good finisher in a weak side scored
/// more than a goal a match.
const MAX_SHOT_SHARE: f64 = 0.28;
/// The same limit for the share of the assists.
const MAX_ASSIST_SHARE: f64 = 0.45;
/// Each point of creativity multiplies a player's share of his side's assists by this (as an exponent).
const ASSISTS_PER_CREATIVITY_POINT: f64 = 0.05;
/// Chance a goal is set up by a teammate.
const GOAL_ASSISTED: f64 = 0.70;

/// The slot of the player who takes a shot.
pub(super) fn shooter(model: ScoringModel, players: &[MatchPlayer; 11], rng: &mut Rng) -> usize {
    pick(players, |p| shot_weight(model, p), MAX_SHOT_SHARE, rng)
}

/// Chance (0 to 1) that a shot on target by `shooter` beats `keeper`.
pub(super) fn goal_chance(model: ScoringModel, shooter: &MatchPlayer, keeper: &MatchPlayer) -> f64 {
    match model {
        ScoringModel::Flat => GOAL_PER_SHOT_ON_TARGET,
        ScoringModel::StarWeighted => {
            let finishing = (FINISHING_PER_POINT * (shooter.finishing as f64 - REFERENCE_RATING)).exp().clamp(FINISHING_FACTOR_RANGE.0, FINISHING_FACTOR_RANGE.1);
            let keeping = (KEEPER_PER_POINT * (REFERENCE_RATING - keeper.rating as f64)).exp().clamp(KEEPER_FACTOR_RANGE.0, KEEPER_FACTOR_RANGE.1);
            GOAL_PER_SHOT_ON_TARGET * finishing * keeping
        }
    }
}

/// The slot of the player who set up a goal scored by `scorer`, or `None` if nobody did. Never the scorer.
pub(super) fn assister(model: ScoringModel, players: &[MatchPlayer; 11], scorer: &MatchPlayer, rng: &mut Rng) -> Option<usize> {
    if rng.unit() >= GOAL_ASSISTED {
        return None;
    }
    let weight = |p: &MatchPlayer| if p.id == scorer.id { 0.0 } else { assist_weight(model, p) };
    // With nobody else able to assist, `pick` would fall back to slot 0, the keeper.
    players.iter().any(|p| weight(p) > 0.0).then(|| pick(players, weight, MAX_ASSIST_SHARE, rng))
}

/// How likely a player is to take a shot: the shots per 90 minutes of his slot, raised by how well he finishes.
// The three line averages (defenders 0.48, midfielders 1.45, attackers 2.81) are real Premier League 2014/15 figures
// (research/game-features/match-simulation-methods.md). The split inside the midfield and attack is our own guess,
// chosen so a 4-4-2's midfield still averages 1.45: wide and attacking midfielders shoot about twice as often as
// central ones.
fn shot_weight(model: ScoringModel, p: &MatchPlayer) -> f64 {
    use Position::*;
    let per_90 = match p.position {
        _ if p.is_empty() => 0.0,
        Gk => 0.0,
        Lwb | Lb | Cb | Rb | Rwb => 0.48,
        Cdm => 0.8,
        Cm => 1.0,
        Lm | Rm | Cam => 2.0,
        Lw | Rw => 2.6,
        Cf | St => 2.81,
    };
    match model {
        ScoringModel::Flat => per_90,
        ScoringModel::StarWeighted => per_90 * (SHOTS_PER_FINISHING_POINT * p.finishing as f64).exp(),
    }
}

/// How likely a player is to set up a goal: the assists per 90 minutes of his slot, raised by his creativity. The
/// line averages (defenders 0.04, midfielders 0.12, attackers 0.11) are real; the split inside them is our own guess.
fn assist_weight(model: ScoringModel, p: &MatchPlayer) -> f64 {
    use Position::*;
    let per_90 = match p.position {
        _ if p.is_empty() => 0.0,
        Gk => 0.0,
        Cb => 0.03,
        Lwb | Lb | Rb | Rwb => 0.06,
        Cdm => 0.04,
        Cm => 0.08,
        Lm | Rm | Cam => 0.20,
        Lw | Rw => 0.22,
        Cf | St => 0.18,
    };
    match model {
        ScoringModel::Flat => per_90,
        ScoringModel::StarWeighted => per_90 * (ASSISTS_PER_CREATIVITY_POINT * p.creativity as f64).exp(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::MatchTeam;

    // Rules every model must keep; a new model is added to `ScoringModel::ALL` and these run for it too.

    #[test]
    fn every_model_picks_a_filled_outfield_slot_to_shoot_and_never_the_scorer_to_assist() {
        for &model in ScoringModel::ALL {
            let mut team = MatchTeam::flat(70);
            team.players[10] = MatchPlayer::empty(Position::St);
            let mut rng = Rng::new(2);
            for _ in 0..2000 {
                let slot = shooter(model, &team.players, &mut rng);
                assert!(slot != 0 && slot != 10, "{model:?}: slot {slot} shot");
                if let Some(a) = assister(model, &team.players, &team.players[slot], &mut rng) {
                    assert!(a != slot && a != 0 && a != 10, "{model:?}: slot {a} assisted slot {slot}");
                }
            }
        }
    }

    #[test]
    fn every_model_gives_a_probability_and_a_goal_is_less_likely_against_a_better_keeper() {
        for &model in ScoringModel::ALL {
            let team = MatchTeam::flat(70);
            let keeper_of = |rating| MatchPlayer { rating, ..team.players[0] };
            let chance = |rating| goal_chance(model, &team.players[10], &keeper_of(rating));
            assert!((0.0..=1.0).contains(&chance(40)) && (0.0..=1.0).contains(&chance(99)), "{model:?}");
            assert!(chance(99) <= chance(40), "{model:?}");
        }
    }

    #[test]
    fn forwards_take_far_more_shots_than_defenders_in_every_model() {
        for &model in ScoringModel::ALL {
            let team = MatchTeam::flat(70);
            let mut rng = Rng::new(9);
            let (mut forwards, mut defenders) = (0, 0);
            for _ in 0..5000 {
                match team.players[shooter(model, &team.players, &mut rng)].position.line() {
                    crate::model::Line::Attack => forwards += 1,
                    crate::model::Line::Defence => defenders += 1,
                    _ => {}
                }
            }
            // Two forwards against four defenders.
            assert!(forwards / 2 > 3 * (defenders / 4), "{model:?}: {forwards} against {defenders}");
        }
    }

    #[test]
    fn a_goal_nobody_can_have_set_up_has_no_assister_rather_than_the_keeper() {
        let mut team = MatchTeam::flat(70);
        for p in team.players.iter_mut().skip(1) {
            p.id = 0; // empty slots
        }
        team.players[10].id = 10;
        let mut rng = Rng::new(8);
        assert!((0..500).all(|_| assister(ScoringModel::StarWeighted, &team.players, &team.players[10], &mut rng).is_none()));
    }

    // Rules of the star-weighted model.

    #[test]
    fn nobody_takes_more_than_the_largest_share_of_the_shots() {
        let mut team = MatchTeam::flat(60);
        team.players[10].finishing = 99; // one star striker in a weak side
        let mut rng = Rng::new(1);
        let star = (0..5000).filter(|_| shooter(ScoringModel::StarWeighted, &team.players, &mut rng) == 10).count() as f64 / 5000.0;
        assert!((MAX_SHOT_SHARE - 0.03..MAX_SHOT_SHARE + 0.03).contains(&star), "the star took {star} of the shots");
    }

    #[test]
    fn a_better_finisher_shoots_and_scores_more_only_in_the_star_weighted_model() {
        let mut team = MatchTeam::flat(70);
        team.players[10].finishing = 95;
        let keeper = team.players[0];
        let mut rng = Rng::new(3);
        let star_shots = |model, rng: &mut Rng| (0..5000).filter(|_| shooter(model, &team.players, rng) == 10).count();
        // The share limit holds the star to about 28% of the shots, against about 20% for an equal striker.
        assert!(star_shots(ScoringModel::StarWeighted, &mut rng) as f64 > 1.2 * star_shots(ScoringModel::Flat, &mut rng) as f64);
        let (star, plain) = (&team.players[10], &team.players[9]);
        assert!(goal_chance(ScoringModel::StarWeighted, star, &keeper) > goal_chance(ScoringModel::StarWeighted, plain, &keeper));
        assert_eq!(goal_chance(ScoringModel::Flat, star, &keeper), goal_chance(ScoringModel::Flat, plain, &keeper));
    }
}
