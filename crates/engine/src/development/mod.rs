//! How the user's players change away from their real careers (everyone else follows the real data, `source`).
//!
//! Experience: every match gives each player XP (`match_xp`), which fills a bar (`Player::xp`); a full bar (`XP_PER_LEVEL`)
//! is a level (`level_points` overall points, applied by `World::level_up_club`) and the XP over the line stays.
//! The summer: `World::age_club` changes every player of the user's club once when a season starts, by an `AgeingModel`.
//! Adding a model: a new file, an enum variant and an arm in the dispatch function; the contract tests run for it
//! through `ALL`. Design and real figures: `research/game-features/experience.md`.
//!
//! - `none`: no XP, no levels.
//! - `match_based`: XP from minutes, form and player of the match, less for a high overall, more for a young player.
mod ageing;
mod match_based;

use crate::attrs::Attr;
use crate::lineup::overall_position;
use crate::model::{ClubId, Line, PlayerId, Position, World};
use crate::overall::{overall_change, weights};
use crate::rng::Rng;

/// Which model decides how players change over the summer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgeingModel {
    /// Nobody changes: the user's players keep their ratings from one season to the next.
    None,
    /// Young players grow, old players decline, by a table of age and the gap to potential.
    ByAge,
}

impl AgeingModel {
    /// Every model, so a test can run them all.
    pub const ALL: &'static [AgeingModel] = &[AgeingModel::None, AgeingModel::ByAge];
}

/// The model every career uses.
pub const DEFAULT_AGEING_MODEL: AgeingModel = AgeingModel::ByAge;

/// The most a summer's random change moves a player from the model's mean, in overall points (the change is the sum of two
/// even draws, so it clusters near the mean: a standard deviation of about 1.2).
const SUMMER_NOISE: f32 = 2.9;

/// Experience points that fill the bar and make a level.
pub const XP_PER_LEVEL: u16 = 100;

/// Which model decides the experience and what a level is worth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExperienceModel {
    /// No experience: no XP, no levels.
    None,
    /// XP from minutes, form and player of the match, less for a high overall.
    MatchBased,
}

impl ExperienceModel {
    /// Every model, so a test can run them all.
    pub const ALL: &'static [ExperienceModel] = &[ExperienceModel::None, ExperienceModel::MatchBased];
}

/// The model every career uses.
pub const DEFAULT_EXPERIENCE_MODEL: ExperienceModel = ExperienceModel::MatchBased;

/// What an experience model knows about one player's match.
#[derive(Clone, Copy, Debug)]
pub struct MatchPlay {
    /// Minutes on the pitch (0 if he did not play).
    pub minutes: u16,
    /// His match rating, or `None` if he played too few minutes to be rated.
    pub rating: Option<f32>,
    /// What an uneventful match is worth to a player of his rating and position (`rating::neutral_rating`), so the
    /// model can tell a good match from a high overall.
    pub neutral_rating: f32,
    /// His rating in the slot he played, 0-99.
    pub overall: u8,
    /// His age in years, which sets the bonus a young player gets.
    pub age: u8,
    /// The part of the pitch he played on, which sets what a good rating is.
    pub line: Line,
    pub player_of_the_match: bool,
}

/// The experience points one match gives a player (1 to 10, more for a young player); 0 if he did not play.
pub fn match_xp(model: ExperienceModel, play: &MatchPlay) -> u8 {
    match model {
        ExperienceModel::None => 0,
        ExperienceModel::MatchBased => match_based::match_xp(play),
    }
}

/// What an experience model knows about a player who is about to level up.
#[derive(Clone, Copy, Debug)]
pub struct Candidate {
    /// 0-99.
    pub overall: u8,
    /// 0-99.
    pub potential: u8,
    /// Years.
    pub age: u8,
    /// Goalkeepers age later.
    pub keeper: bool,
}

/// Overall points one level gives the player. Can be fractional.
pub fn level_points(model: ExperienceModel, who: &Candidate) -> f32 {
    match model {
        ExperienceModel::None => 0.0,
        ExperienceModel::MatchBased => match_based::level_points(who),
    }
}

/// The mean overall points (can be negative) a summer gives the player; 0 for no ageing.
pub fn summer_points(model: AgeingModel, who: &Candidate) -> f32 {
    match model {
        AgeingModel::None => 0.0,
        AgeingModel::ByAge => ageing::mean_change(who),
    }
}

/// What levelling up did to one player.
#[derive(Clone, Copy, Debug)]
pub struct Development {
    pub player: PlayerId,
    /// How many times his bar filled.
    pub levels: u8,
    pub overall_before: u8,
    pub overall_after: u8,
}

impl World {
    /// The summer: every player of the club changes by what his age and potential say, plus a random amount, applied to
    /// his attributes and overall like a level (`apply`). Young players go up, from about 31 they decline. Call it
    /// once when a season starts, after the transfers; the rest of the world follows the real data
    /// (`source::advance_ratings`). Returns who changed, `levels` 0.
    // shortcut: every attribute of his position moves by the same amount; real players lose pace before passing.
    // Weight the attributes by age when that matters (`age_curves.py` has the per-attribute figures).
    // A bigger real rise (`Player::real_change`) replaces the change, see below.
    pub fn age_club(&mut self, club: ClubId, rng: &mut Rng) -> Vec<Development> {
        self.age_club_with(DEFAULT_AGEING_MODEL, club, rng)
    }

    /// `age_club` with a given model.
    pub fn age_club_with(&mut self, model: AgeingModel, club: ClubId, rng: &mut Rng) -> Vec<Development> {
        let mut changed = vec![];
        let year = self.year;
        for id in self.clubs[&club].squad.clone() {
            let p = self.players.get_mut(&id).unwrap();
            let (position, before) = (overall_position(p).unwrap_or(Position::Cm), p.overall);
            let mean = summer_points(model, &Candidate { overall: p.overall, potential: p.potential, age: p.age(year), keeper: position == Position::Gk });
            let drawn = if mean != 0.0 { mean + (rng.unit() + rng.unit() - 1.0) as f32 * SUMMER_NOISE } else { 0.0 };
            // Only a player the table says is declining can go down: luck alone must not take points off a player in
            // his prime (it felt like an attack). The cost is a small rise in the average growth of those players.
            let own = if mean >= 0.0 { drawn.max(0.0) } else { drawn };
            // A player who really improved more than that keeps the real rise: a real star should not be held back.
            // A real decline, or a smaller rise, changes nothing.
            let points = if p.real_change > 0 && p.real_change as f32 > own { p.real_change as f32 } else { own };
            p.real_change = 0;
            if points != 0.0 {
                apply(p, position, points, rng);
            }
            if p.overall != before {
                changed.push(Development { player: id, levels: 0, overall_before: before, overall_after: p.overall });
            }
        }
        if !self.clubs[&club].formation_set {
            self.choose_formation(club);
        }
        changed
    }

    /// Levels up every player of the club whose XP bar is full: each level moves the attributes that count for his
    /// position by the level's points (rounded at random so the average is exact) and his overall by what that is
    /// worth, and takes `XP_PER_LEVEL` off the bar, so XP over the line carries on. The automatic formation is chosen
    /// again, as the squad's strength changed. Call it after matches. Returns who levelled up.
    // shortcut: every attribute of his position moves by the same amount; real players grow in different attributes at
    // different ages (physical ones fade first, `age_curves.py`). Weight the attributes by age when that matters.
    pub fn level_up_club(&mut self, club: ClubId, rng: &mut Rng) -> Vec<Development> {
        self.level_up_club_with(DEFAULT_EXPERIENCE_MODEL, club, rng)
    }

    /// `level_up_club` with a given model.
    pub fn level_up_club_with(&mut self, model: ExperienceModel, club: ClubId, rng: &mut Rng) -> Vec<Development> {
        let mut changed = vec![];
        let year = self.year;
        for id in self.clubs[&club].squad.clone() {
            let p = self.players.get_mut(&id).unwrap();
            let (position, before, mut levels) = (overall_position(p).unwrap_or(Position::Cm), p.overall, 0u8);
            while p.xp >= XP_PER_LEVEL {
                p.xp -= XP_PER_LEVEL;
                let points = level_points(model, &Candidate { overall: p.overall, potential: p.potential, age: p.age(year), keeper: position == Position::Gk });
                apply(p, position, points, rng);
                levels = levels.saturating_add(1);
            }
            if levels > 0 {
                changed.push(Development { player: id, levels, overall_before: before, overall_after: p.overall });
            }
        }
        if !changed.is_empty() && !self.clubs[&club].formation_set {
            self.choose_formation(club);
        }
        changed
    }
}

/// Moves the attributes that count for `position` by `points` each, and the overall by what that is worth. `position` is
/// the one his overall belongs to (`overall_position`), so the overall and that position's formula move together and
/// his rating at every other position, worked out from the two, stays right. An
/// attribute that is missing or already at its limit does not move, and the overall does not count it.
fn apply(p: &mut crate::model::Player, position: Position, points: f32, rng: &mut Rng) {
    let changes: Vec<(Attr, i8)> = weights(position)
        .iter()
        .map(|&(attr, _)| {
            let wanted = random_round(points, rng) as i8;
            (attr, p.attrs.shift(attr, wanted))
        })
        .collect();
    // Rounded at random too, or a level worth a tenth of a point would never show in the overall.
    p.overall = (p.overall as i16 + random_round(overall_change(position, &changes), rng)).clamp(1, 99) as u8;
}

/// A number made whole at random, so that over many players the average is the number itself: 1.3 is 2 three times in ten
/// and 1 otherwise.
fn random_round(x: f32, rng: &mut Rng) -> i16 {
    let whole = x.floor();
    whole as i16 + (rng.unit() < (x - whole) as f64) as i16
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lineup::rating_at;
    use crate::testing::{add_club_with_eleven, player_with_attrs};

    fn candidate(overall: u8, potential: u8, age: u8) -> Candidate {
        Candidate { overall, potential, age, keeper: false }
    }

    fn play(minutes: u16, rating: Option<f32>, overall: u8, motm: bool) -> MatchPlay {
        MatchPlay { minutes, rating, neutral_rating: 6.3 + 0.03 * (overall as f32 - 67.0), overall, age: 27, line: Line::Midfield, player_of_the_match: motm }
    }

    // Rules every model must keep; a new model is added to `ExperienceModel::ALL` and these run for it too.

    #[test]
    fn every_model_gives_between_none_and_ten_xp_a_match_and_a_level_of_a_couple_of_points_at_most() {
        for &model in ExperienceModel::ALL {
            for minutes in [0, 5, 45, 90, 120] {
                for rating in [None, Some(3.0), Some(6.6), Some(10.0)] {
                    for overall in [1, 45, 70, 99] {
                        let xp = match_xp(model, &play(minutes, rating, overall, minutes > 60));
                        assert!(xp <= 20, "{model:?}: {xp}");
                        assert!(minutes > 0 || xp == 0, "{model:?}: he did not play but got {xp}");
                    }
                }
            }
            for overall in [45, 70, 92] {
                for potential in [50, 80, 99] {
                    for age in [16, 22, 30, 38] {
                        let points = level_points(model, &candidate(overall, potential, age));
                        assert!((0.0..=2.0).contains(&points), "{model:?} {overall} {potential} {age}: {points}");
                    }
                }
            }
        }
    }

    // Rules of the match-based model.

    #[test]
    fn a_match_gives_one_to_ten_xp_more_for_the_whole_match_for_form_and_for_player_of_the_match() {
        let xp = |minutes, rating, motm| match_xp(ExperienceModel::MatchBased, &play(minutes, rating, 50, motm));
        assert_eq!(xp(0, None, false), 0);
        assert!(xp(10, Some(6.3), false) >= 1);
        assert!(xp(90, Some(6.5), false) > xp(20, Some(6.5), false), "the whole match is worth more");
        assert!(xp(90, Some(8.5), false) > xp(90, Some(5.5), false), "form is worth more");
        assert!(xp(90, Some(7.5), true) > xp(90, Some(7.5), false), "player of the match is worth more");
        assert_eq!(xp(90, Some(10.0), true), 10, "the best a low overall gets");
    }

    #[test]
    fn a_young_player_gets_more_xp_for_the_same_match_in_tiers_and_nobody_gets_less_than_one() {
        let xp = |age| match_xp(ExperienceModel::MatchBased, &MatchPlay { age, ..play(90, Some(6.8), 60, false) });
        let by_age: Vec<u8> = (16..=34).map(xp).collect();
        assert!(by_age.windows(2).all(|w| w[0] >= w[1]), "never more XP for being older: {by_age:?}");
        assert_eq!(xp(27), xp(34), "27 and over get the base");
        assert!(xp(18) >= 2 * xp(27) - 1 && xp(18) > xp(22) && xp(22) > xp(25) && xp(25) > xp(27), "{} {} {} {}", xp(18), xp(22), xp(25), xp(27));
        assert_eq!(xp(20), xp(16), "the same up to 20");
        let cameo = |age| match_xp(ExperienceModel::MatchBased, &MatchPlay { age, ..play(5, None, 95, false) });
        assert!(cameo(34) >= 1 && cameo(17) >= 2, "{} {}", cameo(34), cameo(17));
        assert_eq!(match_xp(ExperienceModel::None, &MatchPlay { age: 17, ..play(90, Some(8.0), 50, true) }), 0);
    }

    #[test]
    fn the_same_match_gives_a_high_overall_less_xp_than_a_low_one() {
        let xp = |overall| match_xp(ExperienceModel::MatchBased, &play(90, Some(7.5), overall, false));
        assert!(xp(55) > xp(75) && xp(75) > xp(95), "{} {} {}", xp(55), xp(75), xp(95));
        assert!(xp(55) >= 2 * xp(95), "{} against {}", xp(55), xp(95));
        assert!(xp(95) >= 1, "even a star gets something for playing");
    }

    // Rules every ageing model must keep.

    #[test]
    fn every_ageing_model_gives_a_modest_finite_change() {
        for &model in AgeingModel::ALL {
            for age in 15..=45 {
                for potential in [40, 70, 95] {
                    for keeper in [false, true] {
                        let c = Candidate { overall: 70, potential, age, keeper };
                        let points = summer_points(model, &c);
                        assert!(points.is_finite() && (-6.0..=5.0).contains(&points), "{model:?} {age}: {points}");
                    }
                }
            }
        }
        assert_eq!(summer_points(AgeingModel::None, &candidate(70, 90, 18)), 0.0);
    }

    // Rules of the by-age model.

    #[test]
    fn young_players_grow_older_players_decline_slowly_at_first_and_then_a_lot() {
        let at = |age| summer_points(AgeingModel::ByAge, &candidate(70, 80, age));
        assert!(at(17) > at(21) && at(21) > at(25) && at(25) > 0.0, "{} {} {}", at(17), at(21), at(25));
        assert!(at(29) >= 0.0 && at(30) >= 0.0, "no decline before 31");
        assert!((-1.0..0.0).contains(&at(31)), "slowly from 31: {}", at(31));
        assert!(at(34) < at(31) && at(34) > -2.0, "a bit more at 33 and 34: {}", at(34));
        assert!(at(37) < -3.0 && at(37) < 2.0 * at(34).min(at(31)), "a lot after 35: {}", at(37));
        let by_age: Vec<f32> = (28..=40).map(at).collect();
        assert!(by_age.windows(2).all(|w| w[0] >= w[1]), "never better for being older: {by_age:?}");
    }

    #[test]
    fn a_young_player_below_his_potential_grows_most_and_one_past_it_grows_little() {
        let at = |overall, potential| summer_points(AgeingModel::ByAge, &candidate(overall, potential, 19));
        assert!(at(60, 85) > at(60, 65) && at(60, 65) > at(60, 60) && at(60, 60) > at(70, 60), "{} {} {} {}", at(60, 85), at(60, 65), at(60, 60), at(70, 60));
        assert!(at(70, 60) > 0.0, "a soft limit: still a little growth past it");
        let old = |potential| summer_points(AgeingModel::ByAge, &candidate(70, potential, 35));
        assert_eq!(old(99), old(50), "potential does not slow a decline");
    }

    #[test]
    fn a_bigger_real_rise_replaces_the_players_own_change_but_a_real_decline_or_a_smaller_rise_does_not() {
        let mut w = world_with_two_clubs(0);
        w.year = 2014;
        let squad = w.clubs[&1].squad.clone();
        for (i, id) in squad.iter().enumerate() {
            let p = w.players.get_mut(id).unwrap();
            p.birth_date = "1978-06-01".into(); // 36: our own change is about -3
            p.real_change = match i {
                0 => 6,  // really rose by 6
                1 => 1,  // a rise of 1 is still better than our decline of about 3
                2 => -8, // really fell more
                _ => 0,
            };
        }
        let before: Vec<u8> = squad.iter().map(|id| w.players[id].overall).collect();
        w.age_club(1, &mut Rng::new(3));
        let change = |i: usize| w.players[&squad[i]].overall as i16 - before[i] as i16;
        assert!((5..=6).contains(&change(0)), "the real rise is used: {}", change(0));
        assert!((0..=2).contains(&change(1)), "a rise of 1 beats a decline of 3: {}", change(1));
        assert!(change(2) > -8 && change(2) < 0, "a real decline does not replace ours: {}", change(2));
        assert!(squad.iter().all(|id| w.players[id].real_change == 0), "used up");
    }

    #[test]
    fn nobody_the_table_says_is_growing_or_steady_ever_goes_down_whatever_the_draw() {
        for age_year in [1996, 1991, 1989, 1987, 1984] {
            // 18, 23, 25, 27 and 30 in 2014; keepers count as two years younger, so the same holds for them
            for seed in 1..=40 {
                let mut w = world_with_two_clubs(0);
                w.year = 2014;
                for p in w.players.values_mut() {
                    p.birth_date = format!("{age_year}-06-01");
                    p.potential = 66; // almost no room: the case where luck used to take points off
                }
                let before: Vec<u8> = w.clubs[&1].squad.iter().map(|id| w.players[id].overall).collect();
                w.age_club(1, &mut Rng::new(seed));
                let after: Vec<u8> = w.clubs[&1].squad.iter().map(|id| w.players[id].overall).collect();
                assert!(before.iter().zip(&after).all(|(b, a)| a >= b), "born {age_year}, seed {seed}: {before:?} -> {after:?}");
            }
        }
    }

    #[test]
    fn a_change_moves_the_overall_with_the_position_it_belongs_to_not_the_first_listed_one() {
        use crate::attrs::Attr::*;
        use crate::lineup::{formula_at, overall_position};
        // Listed RM first, but his attributes are a full-back's: his overall belongs to RB.
        let mut w = world_with_two_clubs(0);
        w.year = 2014;
        let id = w.clubs[&1].squad[1];
        let defender = [(Marking, 85), (StandingTackle, 85), (SlidingTackle, 85), (Interceptions, 85), (Finishing, 30)];
        let (club, xp) = (Some(1), 0);
        *w.players.get_mut(&id).unwrap() = player_with_attrs(id, 80, &[Position::Rm, Position::Rb], &defender);
        let p = w.players.get_mut(&id).unwrap();
        (p.club, p.xp, p.potential, p.birth_date, p.real_change) = (club, xp, 90, "1996-01-01".into(), 6);
        assert_eq!(overall_position(&w.players[&id]), Some(Position::Rb));
        let gap = |w: &World| w.players[&id].overall as f32 - formula_at(&w.players[&id], Position::Rb).unwrap();
        let before = gap(&w);
        w.age_club(1, &mut Rng::new(2));
        assert!(w.players[&id].overall >= 84, "the real rise was applied: {}", w.players[&id].overall);
        assert!((gap(&w) - before).abs() < 1.5, "his overall and his formula at the position it belongs to move together: {before} -> {}", gap(&w));
    }

    #[test]
    fn goalkeepers_change_later() {
        let at = |keeper, age| summer_points(AgeingModel::ByAge, &Candidate { overall: 75, potential: 80, age, keeper });
        assert!(at(true, 33) > at(false, 33) && at(true, 24) > at(false, 24));
    }

    #[test]
    fn a_summer_lowers_old_players_raises_young_ones_and_leaves_other_clubs_alone() {
        let mut w = world_with_two_clubs(0);
        for p in w.players.values_mut() {
            p.birth_date = "1978-06-01".into(); // 36 in 2014
            p.potential = 90;
        }
        w.year = 2014;
        let total = |w: &World| w.clubs[&1].squad.iter().map(|id| w.players[id].overall as u32).sum::<u32>();
        let before = total(&w);
        let changed = w.age_club(1, &mut Rng::new(5));
        let after = total(&w);
        assert!(after < before, "36 year olds decline: {before} {after}");
        assert!(changed.iter().all(|c| c.levels == 0 && c.overall_after != c.overall_before));
        assert!(w.players.values().filter(|p| p.club == Some(2)).all(|p| p.overall == 65), "only the user's club");
        for p in w.players.values_mut().filter(|p| p.club == Some(1)) {
            p.birth_date = "1996-01-01".into(); // 18 in 2014
            p.overall = 60;
        }
        w.age_club(1, &mut Rng::new(6));
        assert!(total(&w) > 11 * 60, "18 year olds with a gap to potential grow: {}", total(&w));
    }

    #[test]
    fn a_level_is_worth_less_to_an_older_player() {
        let at = |age| level_points(ExperienceModel::MatchBased, &candidate(70, 90, age));
        assert!(at(19) > at(27) && at(27) > at(34) && at(34) > 0.0);
    }

    #[test]
    fn a_level_past_potential_is_worth_less_but_still_something() {
        let at = |potential| level_points(ExperienceModel::MatchBased, &candidate(80, potential, 22));
        let (below, over_by_2, over_by_14) = (at(90), at(78), at(66));
        assert!(below > over_by_2 && over_by_2 > over_by_14 && over_by_14 > 0.0, "{below} {over_by_2} {over_by_14}");
        assert!(over_by_2 > 0.6 * below, "a little over potential is a soft limit, not a wall: {over_by_2} against {below}");
    }

    #[test]
    fn random_rounding_keeps_the_average() {
        let mut rng = Rng::new(2);
        for x in [1.3_f32, 0.1, -0.4, -2.75, 0.0] {
            let mean = (0..20_000).map(|_| random_round(x, &mut rng) as f32).sum::<f32>() / 20_000.0;
            assert!((mean - x).abs() < 0.03, "{x} rounded to {mean} on average");
        }
    }

    // The part that touches the world.

    /// Two clubs of eleven players with attributes at 50, overall 65, potential 90, born in 1996.
    fn world_with_two_clubs(xp: u16) -> World {
        let mut w = World::new(15, 2014);
        add_club_with_eleven(&mut w, 1, 1, 65);
        add_club_with_eleven(&mut w, 2, 1, 65);
        for id in w.players.keys().copied().collect::<Vec<_>>() {
            let p = w.players.get_mut(&id).unwrap();
            let club = p.club;
            *p = player_with_attrs(id, 65, &p.positions.clone(), &[]);
            (p.club, p.potential, p.xp, p.birth_date) = (club, 90, xp, "1996-01-01".into());
        }
        w
    }

    #[test]
    fn a_full_bar_is_a_level_and_the_xp_over_the_line_stays() {
        let mut w = world_with_two_clubs(250);
        let changed = w.level_up_club(1, &mut Rng::new(1));
        assert_eq!(changed.len(), 11);
        assert!(changed.iter().all(|c| c.levels == 2 && c.overall_after >= c.overall_before));
        assert!(w.players.values().filter(|p| p.club == Some(1)).all(|p| p.xp == 50), "two levels leave 50 XP");
        assert!(w.players.values().filter(|p| p.club == Some(1)).any(|p| p.overall > 65));
        assert!(w.players.values().filter(|p| p.club == Some(2)).all(|p| p.overall == 65 && p.xp == 250), "only the user's club levels up");
        // Below the line nothing happens.
        let mut w = world_with_two_clubs(99);
        assert!(w.level_up_club(1, &mut Rng::new(1)).is_empty());
        assert!(w.players.values().all(|p| p.xp == 99 && p.overall == 65));
    }

    #[test]
    fn with_the_none_model_nobody_levels_up_but_the_bar_is_still_emptied() {
        let mut w = world_with_two_clubs(150);
        w.level_up_club_with(ExperienceModel::None, 1, &mut Rng::new(1));
        assert!(w.players.values().all(|p| p.overall == 65));
        assert!(w.players.values().filter(|p| p.club == Some(1)).all(|p| p.xp == 50), "the bar is emptied by the levels it paid for");
    }

    #[test]
    fn a_level_moves_the_attributes_and_the_overall_together_so_every_screen_agrees() {
        let mut w = world_with_two_clubs(1000);
        w.level_up_club(1, &mut Rng::new(4));
        let (mut gains, mut expected) = (vec![], vec![]);
        for p in w.players.values().filter(|p| p.club == Some(1)) {
            let main = p.positions[0];
            // What the overall formula says the attribute changes are worth.
            let worth: f32 = weights(main).iter().map(|&(a, w)| w * (p.attrs[a].unwrap() as f32 - 50.0)).sum();
            assert!(weights(main).iter().all(|&(a, _)| p.attrs[a].unwrap() > 50), "ten levels raised every attribute of the position");
            gains.push(p.overall as f32 - 65.0);
            expected.push(worth);
            assert_eq!(rating_at(p, main), p.overall, "his rating at his own position is his overall");
        }
        let mean = |v: &Vec<f32>| v.iter().sum::<f32>() / v.len() as f32;
        assert!((mean(&gains) - mean(&expected)).abs() < 1.0, "the overall gained {} while the attributes are worth {}", mean(&gains), mean(&expected));
    }
}
