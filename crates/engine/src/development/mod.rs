//! Experience: what playing matches does to the players of the club the user manages. Every match gives each player
//! 1 to 10 experience points (XP), worked out by `match_xp`, and his XP adds up in a bar (`Player::xp`). Each time the
//! bar reaches `XP_PER_LEVEL` he levels up: `level_points` overall points, applied to his attributes and overall by
//! `World::level_up_club`, and the XP over the line stays in the bar. Everyone else follows the real data
//! (`source::advance_ratings`), and a level is kept when the ratings move on to the next real edition, because that
//! move is a difference (`follow_real_change`). An experience model is the pair of pure functions `match_xp` and
//! `level_points`, chosen by `ExperienceModel`.
//! Adding a model: a new file, an `ExperienceModel` variant and an arm in each function; the contract tests below run
//! for it through `ExperienceModel::ALL`.
//!
//! - `none`: no XP, no levels.
//! - `match_based`: XP from minutes, form and player of the match, less for a high overall; a level is worth less to an
//!   older player and past his potential.
//!
//! Design and real figures: `research/game-features/experience.md`.
mod match_based;

use crate::attrs::Attr;
use crate::model::{ClubId, Line, PlayerId, Position, World};
use crate::overall::{overall_change, weights};
use crate::rng::Rng;

/// Experience points that fill the bar and make a level.
pub const XP_PER_LEVEL: u16 = 100;

/// Which model decides the experience and what a level is worth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExperienceModel {
    /// No experience: the club follows the real career like everyone else.
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
    /// The part of the pitch he played on, which sets what a good rating is.
    pub line: Line,
    pub player_of_the_match: bool,
}

/// The experience points (0 to 10) one match gives a player; 0 if he did not play.
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
}

/// Overall points one level gives the player. Can be fractional.
pub fn level_points(model: ExperienceModel, who: &Candidate) -> f32 {
    match model {
        ExperienceModel::None => 0.0,
        ExperienceModel::MatchBased => match_based::level_points(who),
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
            let (position, before, mut levels) = (p.positions[0], p.overall, 0u8);
            while p.xp >= XP_PER_LEVEL {
                p.xp -= XP_PER_LEVEL;
                let points = level_points(model, &Candidate { overall: p.overall, potential: p.potential, age: age(year, &p.birth_date) });
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

/// Moves the attributes that count for `position` by `points` each, and the overall by what that is worth. An
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

/// His age in years at the start of the season `year`, from a "1995-01-01" birth date; 25 if it cannot be read.
fn age(year: u16, birth_date: &str) -> u8 {
    birth_date.get(..4).and_then(|y| y.parse::<u16>().ok()).map_or(25, |born| year.saturating_sub(born).min(99) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lineup::rating_at;
    use crate::testing::{add_club_with_eleven, player_with_attrs};

    fn candidate(overall: u8, potential: u8, age: u8) -> Candidate {
        Candidate { overall, potential, age }
    }

    fn play(minutes: u16, rating: Option<f32>, overall: u8, motm: bool) -> MatchPlay {
        MatchPlay { minutes, rating, neutral_rating: 6.3 + 0.03 * (overall as f32 - 67.0), overall, line: Line::Midfield, player_of_the_match: motm }
    }

    // Rules every model must keep; a new model is added to `ExperienceModel::ALL` and these run for it too.

    #[test]
    fn every_model_gives_between_none_and_ten_xp_a_match_and_a_level_of_a_couple_of_points_at_most() {
        for &model in ExperienceModel::ALL {
            for minutes in [0, 5, 45, 90, 120] {
                for rating in [None, Some(3.0), Some(6.6), Some(10.0)] {
                    for overall in [1, 45, 70, 99] {
                        let xp = match_xp(model, &play(minutes, rating, overall, minutes > 60));
                        assert!(xp <= 10, "{model:?}: {xp}");
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
    fn the_same_match_gives_a_high_overall_less_xp_than_a_low_one() {
        let xp = |overall| match_xp(ExperienceModel::MatchBased, &play(90, Some(7.5), overall, false));
        assert!(xp(55) > xp(75) && xp(75) > xp(95), "{} {} {}", xp(55), xp(75), xp(95));
        assert!(xp(55) >= 2 * xp(95), "{} against {}", xp(55), xp(95));
        assert!(xp(95) >= 1, "even a star gets something for playing");
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
    fn the_age_comes_from_the_birth_year() {
        assert_eq!((age(2014, "1995-01-01"), age(2014, "garbage")), (19, 25));
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
