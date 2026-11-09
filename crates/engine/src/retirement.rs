//! Retirement: at the start of every season each player may decide that this is his last; he plays it out and retires
//! when it ends, so the user can see who is going and plan for it (`Player::career`). A retirement model is one pure
//! function, `chance`, chosen by `RetirementModel`.
//! Adding a model: a `RetirementModel` variant and an arm in `chance`; the contract tests below run for it through
//! `RetirementModel::ALL`.
//!
//! - `none`: nobody retires.
//! - `by_age`: the chance rises with age and is smaller for a player with a high overall (stars play on).
//!
//! The data cannot tell us who really retired: a quarter of the 25 to 29 year olds are missing from the next edition
//! too, because the game drops players, so the curve is our own (`research/game-features/retirement.md`).
use crate::model::{CareerStatus, ClubId, PlayerId, World};
use crate::rng::Rng;

/// Which model decides who retires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetirementModel {
    /// Nobody retires.
    None,
    /// The chance rises with age and falls with overall.
    ByAge,
}

impl RetirementModel {
    /// Every model, so a test can run them all.
    pub const ALL: &'static [RetirementModel] = &[RetirementModel::None, RetirementModel::ByAge];
}

/// The model every career uses.
pub const DEFAULT_RETIREMENT_MODEL: RetirementModel = RetirementModel::ByAge;

/// Nobody under this age retires, and the chance grows with the square of the years past it: 1% at 32, 16% at 35, 49%
/// at 38 for an average player.
const NO_RETIREMENT_UNTIL: f32 = 31.0;
const YEARS_TO_CERTAIN: f32 = 10.0;
/// Everyone retires at this age.
const LAST_AGE: u8 = 42;
/// The chance is multiplied by `(100 - overall) / OVERALL_DIVISOR` between these: a 90 retires at 0.4 of the average
/// chance, a 60 at 1.1 times.
const OVERALL_DIVISOR: f32 = 35.0;
const OVERALL_FACTOR_RANGE: (f32, f32) = (0.4, 1.6);

/// The chance (0 to 1) that a player of this age and overall decides to retire after this season.
pub fn chance(model: RetirementModel, age: u8, overall: u8) -> f32 {
    match model {
        RetirementModel::None => 0.0,
        RetirementModel::ByAge => {
            if age >= LAST_AGE {
                return 1.0;
            }
            let years = (age as f32 - NO_RETIREMENT_UNTIL).max(0.0);
            let by_age = (years / YEARS_TO_CERTAIN).powi(2);
            let by_overall = ((100.0 - overall as f32) / OVERALL_DIVISOR).clamp(OVERALL_FACTOR_RANGE.0, OVERALL_FACTOR_RANGE.1);
            (by_age * by_overall).min(1.0)
        }
    }
}

impl World {
    /// Start of a season: every player who is playing decides, by chance, whether this is his last season. Returns the
    /// club and id of those who decided, in id order.
    pub fn plan_retirements(&mut self, rng: &mut Rng) -> Vec<(Option<ClubId>, PlayerId)> {
        self.plan_retirements_with(DEFAULT_RETIREMENT_MODEL, rng)
    }

    /// `plan_retirements` with a given model.
    pub fn plan_retirements_with(&mut self, model: RetirementModel, rng: &mut Rng) -> Vec<(Option<ClubId>, PlayerId)> {
        let mut ids: Vec<PlayerId> = self.players.keys().copied().collect();
        ids.sort_unstable(); // the same seed must give the same retirements, whatever order the map iterates in
        let year = self.year;
        let mut planned = vec![];
        for id in ids {
            let p = self.players.get_mut(&id).expect("id came from the map");
            if p.career != CareerStatus::Playing {
                continue;
            }
            let chance = chance(model, p.age(year), p.overall);
            if chance > 0.0 && rng.unit() < chance as f64 {
                p.career = CareerStatus::Retiring;
                planned.push((p.club, id));
            }
        }
        planned
    }

    /// End of a season: everyone who planned to retire leaves his club for good. Returns their ids in id order.
    pub fn retire_planned(&mut self) -> Vec<PlayerId> {
        let mut retired: Vec<PlayerId> = self.players.values().filter(|p| p.career == CareerStatus::Retiring).map(|p| p.id).collect();
        retired.sort_unstable();
        for id in &retired {
            let p = self.players.get_mut(id).expect("id came from the map");
            p.career = CareerStatus::Retired;
            if let Some(club) = p.club.take().and_then(|c| self.clubs.get_mut(&c)) {
                club.squad.retain(|s| s != id);
            }
        }
        retired
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::add_club_with_eleven;

    #[test]
    fn every_model_gives_a_chance_that_never_falls_with_age() {
        for &model in RetirementModel::ALL {
            for overall in [50, 70, 90] {
                let chances: Vec<f32> = (16..=45).map(|age| chance(model, age, overall)).collect();
                assert!(chances.iter().all(|c| (0.0..=1.0).contains(c)), "{model:?}");
                assert!(chances.windows(2).all(|pair| pair[0] <= pair[1]), "{model:?} at overall {overall}");
            }
        }
    }

    #[test]
    fn by_age_spares_the_young_keeps_stars_longer_and_ends_every_career() {
        let m = RetirementModel::ByAge;
        assert_eq!(chance(m, 28, 60), 0.0);
        assert!(chance(m, 36, 90) < chance(m, 36, 60), "a star plays on");
        assert_eq!(chance(m, LAST_AGE, 95), 1.0);
        assert_eq!(chance(RetirementModel::None, 50, 50), 0.0);
    }

    /// Club 1 of eleven players born in 1979 (35 in 2014), overall 60.
    fn old_club() -> World {
        let mut w = World::new(15, 2014);
        add_club_with_eleven(&mut w, 1, 7, 60);
        for p in w.players.values_mut() {
            p.birth_date = "1979-03-01".into();
        }
        w
    }

    #[test]
    fn a_player_decides_at_the_start_and_leaves_when_the_season_ends() {
        let mut w = old_club();
        let planned = w.plan_retirements_with(RetirementModel::ByAge, &mut Rng::new(3));
        assert!(!planned.is_empty() && planned.len() < 11, "at 35, some of eleven decide: {}", planned.len());
        assert!(planned.iter().all(|(_, id)| w.players[id].career == CareerStatus::Retiring && w.players[id].club == Some(1)), "still at the club until the season ends");
        assert_eq!(w.clubs[&1].squad.len(), 11);

        let retired = w.retire_planned();
        assert_eq!(retired.len(), planned.len());
        for id in &retired {
            assert_eq!((w.players[id].career, w.players[id].club), (CareerStatus::Retired, None));
            assert!(!w.clubs[&1].squad.contains(id));
        }
        assert_eq!(w.clubs[&1].squad.len(), 11 - retired.len());
        assert!(w.retire_planned().is_empty(), "nobody retires twice");
        assert!(w.plan_retirements_with(RetirementModel::ByAge, &mut Rng::new(4)).iter().all(|(_, id)| !retired.contains(id)), "a retired player does not decide again");
    }

    #[test]
    fn the_same_seed_gives_the_same_retirements() {
        let (mut a, mut b) = (old_club(), old_club());
        assert_eq!(a.plan_retirements(&mut Rng::new(9)), b.plan_retirements(&mut Rng::new(9)));
    }

    #[test]
    fn a_player_age_comes_from_his_birth_year() {
        let mut p = crate::testing::player(1, 70, &[crate::model::Position::St]);
        p.birth_date = "1995-01-01".into();
        assert_eq!(p.age(2014), 19);
        p.birth_date = "garbage".into();
        assert_eq!(p.age(2014), 25);
    }
}
