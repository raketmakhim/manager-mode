//! Fitness between matches: playing lowers a player's fitness (0 to 100), rest brings it back, and a player who is not
//! fully fit starts his next match already tired (`sim::FatigueModel`). A match costs what the tiring model says the
//! player lost in it (`sim::match_tiredness`), so a midfielder who plays 90 minutes loses more than a centre-back and
//! a goalkeeper loses nothing. Rest is worked out when a match is about to be played, from the days since the last one.
//!
//! Wear is the second, slower effect: the more matches a player plays in a row (each within a week of the one before),
//! the lower the fitness he can recover to (a ceiling of 100 minus his wear). A week without a match clears it
//! completely, so resting a tired player for one match gives him his recovery back.
//!
//! Every number between matches is in `FitnessSettings`, kept on the `World`, so a test or a tool can change them. The
//! numbers of the tiring inside a match are constants in `sim/fatigue.rs`.
//! Not here yet: injuries, training, rotation chosen by the user beyond the lineup screen.
//! Design and the figures behind it: `research/game-features/fitness.md`.
use crate::attrs::Attr;
use crate::model::{ClubId, Player, Position, World, FULL_FITNESS};
use crate::sim::{match_tiredness, RATING_LOSS_AT_SPENT, REFERENCE_STAMINA};

/// The numbers behind fitness between matches. All are our own guesses (no source gives them), to tune by feel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FitnessSettings {
    /// Rest brings back half of the fitness still missing every this many days, so the first day gives the most and
    /// each later day less (after a full match: about 70% fit one day later, 84% after three days, 96% after a week).
    pub recovery_half_life_days: f64,
    /// Fitness points a player loses who ends a match fully spent: a match costs this times how tired he got.
    pub loss_at_spent: f64,
    /// Tiredness (0 to 1) a player starts a match with when his fitness is 0, in proportion for the fitness between:
    /// at 50 fitness he starts as tired as one who has played about a quarter of a match.
    pub start_tiredness_at_no_fitness: f64,
    /// A match within this many days of the last one keeps a run going; a longer gap (a week without a match) ends it
    /// and clears the wear.
    pub wear_clear_days: u32,
    /// Each stamina point above the average (70) shortens the recovery half-life by this share (as an exponent): at
    /// 0.0144 a stamina of 90 recovers in about 0.75 of the time and a stamina of 50 in about 1.33 times. 0 turns it off.
    pub recovery_stamina_effect: f64,
    /// The same for wear: each stamina point above 70 cuts his wear by this share (as an exponent), so a stamina of 90
    /// wears 0.67 as much and a 50 about 1.5 times. 0 turns it off.
    pub wear_stamina_effect: f64,
    /// Matches in a row that cost no wear.
    pub wear_free_matches: u16,
    /// Fitness points his ceiling drops for each match in a row beyond the free ones.
    pub wear_per_match: f64,
    /// The most wear there is: a player cannot be held below 100 minus this.
    pub wear_max: f64,
    /// A match shorter than this many minutes is not part of a run (a short cameo neither wears nor rests him), and
    /// neither is a match that costs a player no fitness (a goalkeeper).
    pub wear_min_minutes: u16,
}

impl Default for FitnessSettings {
    fn default() -> Self {
        FitnessSettings { recovery_half_life_days: 2.0, loss_at_spent: 80.0, start_tiredness_at_no_fitness: 0.6, recovery_stamina_effect: 0.0144, wear_stamina_effect: 0.02, wear_clear_days: 7, wear_free_matches: 3, wear_per_match: 1.5, wear_max: 15.0, wear_min_minutes: 45 }
    }
}

impl FitnessSettings {
    /// What a player's stamina does to a rate: 1 at average stamina, below 1 for more, above 1 for less, by `effect` per
    /// point.
    fn stamina_factor(stamina: u8, effect: f64) -> f64 {
        (effect * (REFERENCE_STAMINA - stamina as f64)).exp()
    }

    /// How tired (0 to 1) a player of this fitness starts a match.
    pub fn start_tiredness(&self, fitness: u8) -> f64 {
        self.start_tiredness_at_no_fitness * (FULL_FITNESS - fitness.min(FULL_FITNESS)) as f64 / FULL_FITNESS as f64
    }
}

impl Player {
    /// His stamina (0-99); his overall if the data lacks it (FIFA 15).
    fn stamina(&self) -> u8 {
        self.attrs[Attr::Stamina].unwrap_or(self.overall)
    }

    /// His wear on `day`: how far below 100 his fitness cannot recover, from the run of matches he is in. Zero once a
    /// week has passed since his last match.
    fn wear_on(&self, day: u32, s: &FitnessSettings) -> f64 {
        if self.matches_in_a_row == 0 || day > self.last_match_day + s.wear_clear_days {
            return 0.0;
        }
        let wear = (s.wear_per_match * self.matches_in_a_row.saturating_sub(s.wear_free_matches) as f64).min(s.wear_max);
        wear * FitnessSettings::stamina_factor(self.stamina(), s.wear_stamina_effect)
    }

    /// His fitness (0-100) on `day` of the season, after resting since it was last worked out, and never above the
    /// ceiling his wear allows.
    pub fn fitness_on(&self, day: u32, s: &FitnessSettings) -> u8 {
        let missing = (FULL_FITNESS - self.fitness.min(FULL_FITNESS)) as f64;
        let half_life = s.recovery_half_life_days * FitnessSettings::stamina_factor(self.stamina(), s.recovery_stamina_effect);
        let recovered = FULL_FITNESS as f64 - missing * 0.5_f64.powf(day.saturating_sub(self.fitness_day) as f64 / half_life);
        recovered.min(FULL_FITNESS as f64 - self.wear_on(day, s)).round() as u8
    }

    /// Brings `fitness` up to date for `day`.
    fn rest_to(&mut self, day: u32, s: &FitnessSettings) {
        self.fitness = self.fitness_on(day, s);
        self.fitness_day = self.fitness_day.max(day);
    }

    /// After a match of `minutes` on `day`: rests him up to that day, takes off what the match cost him and adds it to
    /// his run of matches for his wear.
    pub(crate) fn tire(&mut self, day: u32, minutes: u16, s: &FitnessSettings) {
        self.rest_to(day, s);
        // shortcut: the loss is worked out for his main position, not the slot he played (a full-back at centre-back
        // tires like a full-back), and with an even share of the ball. Pass the slot and the ball share when it matters.
        let position = self.positions.first().copied().unwrap_or(Position::Cm);
        let loss = (s.loss_at_spent * match_tiredness(position, self.stamina(), minutes)).round() as u8;
        self.fitness = self.fitness.saturating_sub(loss);
        if minutes >= s.wear_min_minutes && loss > 0 {
            let in_a_run = self.matches_in_a_row > 0 && day <= self.last_match_day + s.wear_clear_days;
            self.matches_in_a_row = if in_a_run { self.matches_in_a_row + 1 } else { 1 };
            self.last_match_day = day;
        }
    }
}

impl World {
    /// Brings both clubs of a fixture up to date for its day. Call it before the match is set up: the lineup the
    /// game picks and the players' tiredness at kick-off both read the fitness.
    pub fn rest_for(&mut self, fixture: usize) {
        let f = &self.fixtures[fixture];
        let (day, clubs) = (f.day_number(), [f.home, f.away]);
        for club in clubs {
            self.rest_club_to(club, day);
        }
    }

    /// Treats today as the day of the club's next match: rests its squad up to that day, so a screen shows the fitness
    /// the lineup will really have. The game has no days between matches yet; a calendar you move through would
    /// replace this. Does nothing if the club has no match left.
    pub fn rest_to_next_match(&mut self, club: ClubId) {
        let next = self.fixtures.iter().filter(|f| f.result.is_none() && f.involves(club)).map(|f| f.day_number()).min();
        if let Some(day) = next {
            self.rest_club_to(club, day);
        }
    }

    fn rest_club_to(&mut self, club: ClubId, day: u32) {
        let settings = self.fitness_settings;
        for id in &self.clubs[&club].squad {
            self.players.get_mut(id).expect("a squad player exists").rest_to(day, &settings);
        }
    }

    /// The summer: everyone is back to full fitness for the new season, with no wear.
    pub fn rest_for_summer(&mut self) {
        for p in self.players.values_mut() {
            (p.fitness, p.fitness_day) = (FULL_FITNESS, 0);
            (p.matches_in_a_row, p.last_match_day) = (0, 0);
        }
    }

    /// The rating points a player of this fitness has lost before a match starts, for choosing a lineup.
    pub(crate) fn rating_cost_of_fitness(&self, fitness: u8) -> u8 {
        (RATING_LOSS_AT_SPENT * self.fitness_settings.start_tiredness(fitness)).round() as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::player;

    fn midfielder() -> Player {
        with_stamina(70)
    }

    fn with_stamina(stamina: u8) -> Player {
        let mut p = player(1, 70, &[Position::Cm]);
        p.attrs.set(Attr::Stamina, stamina);
        p
    }

    fn s() -> FitnessSettings {
        FitnessSettings::default()
    }

    #[test]
    fn rest_brings_fitness_back_fastest_on_the_first_day_and_never_past_full() {
        let mut p = midfielder();
        p.fitness = 40;
        let by_day: Vec<u8> = (0..10).map(|d| p.fitness_on(d, &s())).collect();
        assert_eq!(by_day[0], 40);
        let gains: Vec<u8> = by_day.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(gains.windows(2).all(|g| g[0] >= g[1]), "each day gives back less than the one before: {gains:?}");
        assert!(by_day[9] >= 95 && by_day[9] <= FULL_FITNESS);
        p.fitness = FULL_FITNESS;
        assert_eq!(p.fitness_on(50, &s()), FULL_FITNESS);
    }

    #[test]
    fn a_full_match_costs_a_midfielder_fitness_and_most_of_it_is_back_after_a_week_but_not_after_a_day() {
        let mut p = midfielder();
        p.tire(10, 90, &s());
        let lost = FULL_FITNESS - p.fitness;
        assert!((30..=55).contains(&lost), "a 90 minute match should cost 30 to 55 points, not {lost}");
        assert!(p.fitness_on(17, &s()) >= 95, "a week of rest is about full recovery: {}", p.fitness_on(17, &s()));
        assert!((80..95).contains(&p.fitness_on(13, &s())), "three days: mostly back, not all: {}", p.fitness_on(13, &s()));
        assert!(p.fitness_on(11, &s()) < 80, "one day is not enough: {}", p.fitness_on(11, &s()));
    }

    #[test]
    fn a_player_who_plays_every_other_day_wears_down_and_a_goalkeeper_does_not() {
        let (mut outfield, mut keeper) = (midfielder(), player(2, 70, &[Position::Gk]));
        for day in [10, 12, 14, 16] {
            outfield.tire(day, 90, &s());
            keeper.tire(day, 90, &s());
        }
        assert!(outfield.fitness < 30, "{}", outfield.fitness);
        assert_eq!(keeper.fitness, FULL_FITNESS);
        assert_eq!(keeper.wear_on(16, &s()), 0.0);
    }

    #[test]
    fn fewer_minutes_cost_less_and_extra_time_costs_more() {
        let after = |minutes| {
            let mut p = midfielder();
            p.tire(10, minutes, &s());
            p.fitness
        };
        assert!(after(30) > after(90) && after(90) > after(120));
    }

    /// A midfielder who has played one match a week for `matches` weeks, from day 10.
    fn ever_present(matches: u32) -> Player {
        let mut p = midfielder();
        for week in 0..matches {
            p.tire(10 + 7 * week, 90, &s());
        }
        p
    }

    #[test]
    fn a_long_run_of_matches_lowers_what_a_player_can_recover_to_and_a_week_without_one_clears_it() {
        assert_eq!(ever_present(3).wear_on(10 + 7 * 2, &s()), 0.0, "a few matches in a row cost no wear");
        let (short, long) = (ever_present(6), ever_present(20));
        let day = |p: &Player| p.last_match_day + 6;
        assert!(long.wear_on(day(&long), &s()) > short.wear_on(day(&short), &s()), "the longer the run the more wear");
        assert!(long.wear_on(day(&long), &s()) <= s().wear_max);
        assert!(long.fitness_on(day(&long), &s()) < 90, "an ever-present cannot get back to full: {}", long.fitness_on(day(&long), &s()));
        assert!(long.fitness_on(day(&long), &s()) < ever_present(2).fitness_on(10 + 7 + 6, &s()));
        // A week with no match clears it completely.
        let rested = long.last_match_day + s().wear_clear_days + 1;
        assert_eq!(long.wear_on(rested, &s()), 0.0);
        assert!(long.fitness_on(rested, &s()) >= 95, "a rested player is back to normal: {}", long.fitness_on(rested, &s()));
    }

    #[test]
    fn a_missed_week_ends_the_run_and_a_cameo_does_not_count() {
        let mut p = ever_present(10);
        let next = p.last_match_day + 14; // skipped a week
        p.tire(next, 90, &s());
        assert_eq!(p.matches_in_a_row, 1, "the run starts again");
        let before = p.matches_in_a_row;
        p.tire(next + 7, 10, &s());
        assert_eq!((p.matches_in_a_row, p.last_match_day), (before, next), "ten minutes is not a match in the run");
    }

    #[test]
    fn a_player_with_more_stamina_recovers_faster_and_wears_less_and_the_settings_can_turn_that_off() {
        let after_a_day = |stamina, settings: &FitnessSettings| {
            let mut p = with_stamina(stamina);
            p.fitness = 40;
            p.fitness_on(1, settings)
        };
        assert!(after_a_day(90, &s()) > after_a_day(50, &s()), "same hole, the fitter player is back sooner");
        let after_a_season = |stamina| {
            let mut p = with_stamina(stamina);
            for week in 0..20 {
                p.tire(10 + 7 * week, 90, &s());
            }
            p.wear_on(p.last_match_day, &s())
        };
        assert!(after_a_season(90) < after_a_season(70) && after_a_season(70) < after_a_season(50), "{} {} {}", after_a_season(90), after_a_season(70), after_a_season(50));
        let off = FitnessSettings { recovery_stamina_effect: 0.0, wear_stamina_effect: 0.0, ..s() };
        assert_eq!(after_a_day(90, &off), after_a_day(50, &off), "with the effect off, stamina does not change recovery");
    }

    #[test]
    fn the_settings_change_the_result_in_the_expected_direction() {
        let after_a_day = |settings: FitnessSettings| {
            let mut p = midfielder();
            p.tire(10, 90, &settings);
            p.fitness_on(11, &settings)
        };
        let slow = FitnessSettings { recovery_half_life_days: 4.0, ..s() };
        assert!(after_a_day(slow) < after_a_day(s()), "slower recovery leaves him more tired");
        let harsh = FitnessSettings { loss_at_spent: 120.0, ..s() };
        assert!(after_a_day(harsh) < after_a_day(s()), "a bigger cost leaves him more tired");
        assert_eq!(s().start_tiredness(FULL_FITNESS), 0.0);
        assert!(s().start_tiredness(20) > s().start_tiredness(80));
    }

    #[test]
    fn the_game_picks_a_rested_player_by_the_settings_start_tiredness() {
        let mut w = World::new(15, 2014);
        assert_eq!(w.rating_cost_of_fitness(FULL_FITNESS), 0);
        let normal = w.rating_cost_of_fitness(40);
        w.fitness_settings.start_tiredness_at_no_fitness = 1.0;
        assert!(w.rating_cost_of_fitness(40) > normal);
    }

    #[test]
    fn the_summer_rests_everyone_and_clears_wear() {
        let mut w = World::new(15, 2014);
        let mut p = midfielder();
        (p.fitness, p.fitness_day) = (20, 90);
        (p.matches_in_a_row, p.last_match_day) = (9, 85);
        w.players.insert(p.id, p);
        w.rest_for_summer();
        let p = &w.players[&1];
        assert_eq!((p.fitness, p.fitness_day, p.matches_in_a_row, p.last_match_day), (FULL_FITNESS, 0, 0, 0));
        let p = w.players.get_mut(&1).unwrap();
        (p.fitness, p.fitness_day) = (20, 90);
        w.generate_fixtures(); // a new season's fixtures start the days again
        assert_eq!((w.players[&1].fitness, w.players[&1].fitness_day), (FULL_FITNESS, 0));
    }

    #[test]
    fn resting_to_the_next_match_brings_a_club_to_the_day_of_its_next_unplayed_match() {
        use crate::competition::{Competition, Rules};
        use crate::rng::Rng;
        use crate::testing::add_club_with_eleven;
        let mut w = World::new(0, 0);
        for id in 1..=4 {
            add_club_with_eleven(&mut w, id, 1, 60);
        }
        w.competitions.insert(1, Competition { id: 1, name: "L".into(), country: "X".into(), rules: Rules::double_round_robin(), level: None, clubs: (1..=4).collect() });
        w.start_season(&mut Rng::new(1));
        for p in w.players.values_mut() {
            p.fitness = 10;
        }
        let next = w.fixtures.iter().filter(|f| f.involves(1)).map(|f| f.day_number()).min().unwrap();
        w.rest_to_next_match(1);
        assert!(w.clubs[&1].squad.iter().all(|id| w.players[id].fitness_day == next && w.players[id].fitness > 10));
        assert!(w.clubs[&2].squad.iter().all(|id| w.players[id].fitness == 10), "only that club");
        for f in w.fixtures.iter_mut() {
            f.result = Some(crate::outcome::Outcome::full_time(0, 0));
        }
        w.rest_to_next_match(2); // nothing left to play: nothing changes
        assert!(w.clubs[&2].squad.iter().all(|id| w.players[id].fitness == 10));
    }

    #[test]
    fn resting_for_a_fixture_updates_both_squads_to_its_day() {
        use crate::competition::{Competition, Rules};
        use crate::rng::Rng;
        use crate::testing::add_club_with_eleven;
        let mut w = World::new(0, 0);
        for id in 1..=4 {
            add_club_with_eleven(&mut w, id, 1, 60);
        }
        w.competitions.insert(1, Competition { id: 1, name: "L".into(), country: "X".into(), rules: Rules::double_round_robin(), level: None, clubs: (1..=4).collect() });
        w.start_season(&mut Rng::new(1));
        for p in w.players.values_mut() {
            p.fitness = 10;
        }
        let (home, away) = (w.fixtures[0].home, w.fixtures[0].away);
        let day = w.fixtures[0].day_number();
        w.rest_for(0);
        for club in [home, away] {
            assert!(w.clubs[&club].squad.iter().all(|id| w.players[id].fitness_day == day && w.players[id].fitness > 10));
        }
        let other = (1..=4).find(|c| *c != home && *c != away).unwrap();
        assert!(w.clubs[&other].squad.iter().all(|id| w.players[id].fitness == 10), "only the two clubs of the fixture rest");
    }
}
