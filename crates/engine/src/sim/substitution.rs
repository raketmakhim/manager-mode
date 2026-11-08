//! Substitutions: when a side changes players, who goes off and who comes on. A substitution model is a pair of pure
//! functions: `plan` gives the minutes of a side's changes before the match, and `choose` picks, when one is due, the
//! player to take off and the bench player to bring on; the match model calls them through `SubstitutionModel`.
//! Adding a model: a `SubstitutionModel` variant and an arm in each function; the contract tests below run for it
//! through `SubstitutionModel::ALL`.
//!
//! - `Never`: nobody is replaced (the model before substitutions existed).
//! - `Planned`: a few changes after the break, mostly forwards and midfielders off, a bench player of the same line on.
use super::pick::pick;
use super::{MatchPlayer, MatchTeam};
use crate::model::Line;
use crate::rng::Rng;

/// Which model decides the substitutions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubstitutionModel {
    /// No substitutions.
    Never,
    /// Changes planned for the second half, like-for-like by line.
    Planned,
}

impl SubstitutionModel {
    /// Every model, so a test can run them all.
    pub const ALL: &'static [SubstitutionModel] = &[SubstitutionModel::Never, SubstitutionModel::Planned];
}

/// The model every match uses.
pub const DEFAULT_SUBSTITUTION_MODEL: SubstitutionModel = SubstitutionModel::Planned;

// Real figures (research/game-features/simulation-balance.md): about 3.9 changes a side a match under the five-change
// rule and about 2.9 before it, the first at about the 60th minute, three in four after the 60th, and about 13% of
// the goals scored by substitutes. The shape of the spread is our own.

/// The first changes up to this many are used almost every match; the later ones about half the time (5 allowed: 3.9
/// used, 3 allowed: 2.9).
const ALWAYS_USED: u8 = 3;
const ALWAYS_USED_CHANCE: f64 = 0.97;
const EXTRA_USED_CHANCE: f64 = 0.5;
/// A change is made between these minutes of the match, most often around the middle of them.
const EARLIEST_MINUTE: f64 = 50.0;
const SPREAD_MINUTES: f64 = 40.0;

/// The minute of each change one side makes in a 90-minute match, given it may make `allowed`.
pub(super) fn plan(model: SubstitutionModel, allowed: u8, rng: &mut Rng) -> Vec<u8> {
    match model {
        SubstitutionModel::Never => vec![],
        SubstitutionModel::Planned => {
            let mut minutes = vec![];
            for n in 0..allowed {
                if rng.unit() < if n < ALWAYS_USED { ALWAYS_USED_CHANCE } else { EXTRA_USED_CHANCE } {
                    minutes.push((EARLIEST_MINUTE + SPREAD_MINUTES * (rng.unit() + rng.unit()) / 2.0) as u8);
                }
            }
            minutes
        }
    }
}

/// One change: the lineup slot that is emptied and the bench place that fills it.
#[derive(Clone, Copy, Debug)]
pub(super) struct Change {
    pub slot: usize,
    pub bench: usize,
}

/// The change to make now for `team` as it stands, or `None` if there is nobody to take off or bring on.
pub(super) fn choose(model: SubstitutionModel, team: &MatchTeam, rng: &mut Rng) -> Option<Change> {
    match model {
        SubstitutionModel::Never => None,
        SubstitutionModel::Planned => {
            let off_weight = |p: &MatchPlayer| match p.position.line() {
                _ if p.is_empty() => 0.0,
                Line::Goalkeeper => 0.0,
                Line::Defence => 0.7,
                Line::Midfield => 1.2,
                Line::Attack => 1.6,
            };
            if team.players.iter().all(|p| off_weight(p) == 0.0) {
                return None;
            }
            let slot = pick(&team.players, off_weight, 1.0, rng);
            let line = team.players[slot].position.line();
            // The best unused outfield player of the same line, otherwise the best unused outfield player.
            let outfield = |(_, p): &(usize, &MatchPlayer)| !p.is_empty() && p.position.line() != Line::Goalkeeper;
            let best = |same_line: bool| {
                team.bench.iter().enumerate().filter(outfield).filter(|(_, p)| !same_line || p.position.line() == line).max_by_key(|(_, p)| p.rating).map(|(i, _)| i)
            };
            best(true).or_else(|| best(false)).map(|bench| Change { slot, bench })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Rules every model must keep; a new model is added to `SubstitutionModel::ALL` and these run for it too.

    #[test]
    fn every_model_plans_no_more_than_the_allowed_changes_inside_the_match() {
        for &model in SubstitutionModel::ALL {
            let mut rng = Rng::new(1);
            for allowed in [0, 3, 5] {
                for _ in 0..500 {
                    let minutes = plan(model, allowed, &mut rng);
                    assert!(minutes.len() <= allowed as usize && minutes.iter().all(|m| (1..=90).contains(m)), "{model:?}: {minutes:?}");
                }
            }
        }
    }

    #[test]
    fn every_model_takes_off_an_outfield_player_and_brings_on_one_from_the_bench() {
        for &model in SubstitutionModel::ALL {
            let team = MatchTeam::flat(70);
            let mut rng = Rng::new(2);
            for _ in 0..500 {
                if let Some(change) = choose(model, &team, &mut rng) {
                    assert!(change.slot != 0, "{model:?}: the keeper came off");
                    assert!(!team.bench[change.bench].is_empty() && team.bench[change.bench].position.line() != Line::Goalkeeper, "{model:?}");
                }
            }
        }
    }

    #[test]
    fn every_model_changes_nothing_with_an_empty_bench() {
        for &model in SubstitutionModel::ALL {
            let mut team = MatchTeam::flat(70);
            team.bench.iter_mut().for_each(|p| p.id = 0);
            assert!(choose(model, &team, &mut Rng::new(3)).is_none(), "{model:?}");
        }
    }

    // Rules of the planned model.

    #[test]
    fn the_planned_model_makes_about_as_many_changes_as_real_teams_and_mostly_late() {
        let mut rng = Rng::new(4);
        let plans: Vec<Vec<u8>> = (0..4000).map(|_| plan(SubstitutionModel::Planned, 5, &mut rng)).collect();
        let mean = plans.iter().map(|p| p.len()).sum::<usize>() as f64 / plans.len() as f64;
        assert!((3.6..4.2).contains(&mean), "{mean} changes a side");
        let all: Vec<u8> = plans.concat();
        let after_60 = all.iter().filter(|&&m| m > 60).count() as f64 / all.len() as f64;
        assert!((0.65..0.9).contains(&after_60), "{after_60} of the changes after the 60th minute");
    }

    #[test]
    fn the_planned_model_replaces_a_player_with_a_bench_player_of_his_line_when_there_is_one() {
        let team = MatchTeam::flat(70);
        let mut rng = Rng::new(5);
        for _ in 0..500 {
            let change = choose(SubstitutionModel::Planned, &team, &mut rng).unwrap();
            // The flat bench has cover for defence, midfield and attack.
            assert_eq!(team.bench[change.bench].position.line(), team.players[change.slot].position.line());
        }
    }
}
