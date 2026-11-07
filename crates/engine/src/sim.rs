//! The cheap match simulation: two team strengths and a seeded RNG in, an `Outcome` out.
//! Pure functions, no `World`. Scorers and assisters come later as a second layer on the same goals.
use crate::outcome::Outcome;
use crate::rng::Rng;

// The numbers below were tuned with `dev-tools/simtool` so the big five leagues (FIFA 15 ratings) match their
// real 2014/15 to 2023/24 tables: see `research/game-features/simulation-balance.md`. Rerun the simtool after
// changing anything that moves ratings or goals.

/// Goals a team is expected to score against an equal side away from home, before the home bonus.
const BASE_GOALS: f64 = 1.29;
/// Home side's goals are multiplied by this, the away side's divided by it.
const HOME_FACTOR: f64 = 1.14;
/// Each rating point of strength difference changes the goals by this share (as an exponent), split between the sides.
const GOAL_RATE_PER_POINT: f64 = 0.14;
/// Two independent goal counts make fewer draws than real football, where low scores cluster. This is the
/// Dixon-Coles correction (rho): negative values make 0-0 and 1-1 likelier and 1-0 and 0-1 less likely.
const LOW_SCORE_CORRECTION: f64 = -0.10;
/// Chance a penalty is scored.
const PENALTY_SCORED: f64 = 0.75;

/// A team's strength: the mean of the best eleven ratings (0-99) given. Give it the lineup's ratings, each player
/// rated in his slot (`World::lineup_ratings`), and an empty slot as 0. Fewer than eleven ratings: the mean of those
/// given; none: 0.
pub fn team_strength(overalls: &[u8]) -> f64 {
    let mut best = overalls.to_vec();
    best.sort_unstable_by(|a, b| b.cmp(a));
    best.truncate(11);
    if best.is_empty() { 0.0 } else { best.iter().map(|&o| o as f64).sum::<f64>() / best.len() as f64 }
}

/// Plays one match. `must_have_winner` is true for a cup tie: a level score goes to 30 minutes of extra time
/// and then to a shootout. A league match stays level.
pub fn simulate_match(home: f64, away: f64, must_have_winner: bool, rng: &mut Rng) -> Outcome {
    let edge = GOAL_RATE_PER_POINT * (home - away) / 2.0;
    let home_rate = BASE_GOALS * HOME_FACTOR * edge.exp();
    let away_rate = BASE_GOALS / HOME_FACTOR * (-edge).exp();
    let full_time = full_time_score(home_rate, away_rate, rng);
    if !must_have_winner || full_time.0 != full_time.1 {
        return Outcome { full_time, extra_time: None, shootout: None };
    }
    let extra = (poisson(home_rate / 3.0, rng), poisson(away_rate / 3.0, rng));
    let shootout = (extra.0 == extra.1).then(|| shootout(rng));
    Outcome { full_time, extra_time: Some(extra), shootout }
}

/// The score after 90 minutes. Draws two independent Poisson goal counts and keeps the pair with a probability
/// that is higher for 0-0 and 1-1 and lower for 1-0 and 0-1 (rejection sampling, so it needs no tables).
fn full_time_score(home_rate: f64, away_rate: f64, rng: &mut Rng) -> (u8, u8) {
    let weight = |home: u8, away: u8| match (home, away) {
        (0, 0) => 1.0 - home_rate * away_rate * LOW_SCORE_CORRECTION,
        (0, 1) => 1.0 + home_rate * LOW_SCORE_CORRECTION,
        (1, 0) => 1.0 + away_rate * LOW_SCORE_CORRECTION,
        (1, 1) => 1.0 - LOW_SCORE_CORRECTION,
        _ => 1.0,
    };
    let ceiling = [weight(0, 0), weight(0, 1), weight(1, 0), weight(1, 1), 1.0].into_iter().fold(0.0, f64::max);
    loop {
        let score = (poisson(home_rate, rng), poisson(away_rate, rng));
        if rng.unit() * ceiling <= weight(score.0, score.1).max(0.0) {
            return score;
        }
    }
}

/// Number of events with an average of `rate` per match (Knuth's method).
fn poisson(rate: f64, rng: &mut Rng) -> u8 {
    let limit = (-rate).exp();
    let mut count = 0;
    let mut product = rng.unit();
    while product > limit {
        count += 1;
        product *= rng.unit();
    }
    count
}

/// Five penalties each, then one more each until the scores differ. Never level.
fn shootout(rng: &mut Rng) -> (u8, u8) {
    let mut kicks = |n| (0..n).filter(|_| rng.unit() < PENALTY_SCORED).count() as u8;
    let (mut home, mut away) = (kicks(5), kicks(5));
    while home == away {
        home += kicks(1);
        away += kicks(1);
    }
    (home, away)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::outcome::Side;

    /// Home wins, draws and away wins out of 5000 league matches.
    fn tally(home: f64, away: f64) -> (u32, u32, u32) {
        let mut rng = Rng::new(7);
        let mut t = (0, 0, 0);
        for _ in 0..5000 {
            match simulate_match(home, away, false, &mut rng).winner() {
                Some(Side::Home) => t.0 += 1,
                None => t.1 += 1,
                Some(Side::Away) => t.2 += 1,
            }
        }
        t
    }

    #[test]
    fn strength_is_the_mean_of_the_best_eleven() {
        assert_eq!(team_strength(&[]), 0.0);
        assert_eq!(team_strength(&[80, 60]), 70.0);
        let mut squad = vec![70; 11];
        squad.extend([50; 10]); // the bench does not count
        assert_eq!(team_strength(&squad), 70.0);
    }

    #[test]
    fn a_stronger_team_wins_more_often_and_home_advantage_exists() {
        let (home_wins, _, away_wins) = tally(80.0, 70.0);
        assert!(home_wins > 2 * away_wins, "{home_wins} vs {away_wins}");
        let (home_wins, _, away_wins) = tally(70.0, 70.0);
        assert!(home_wins > away_wins, "equal sides: {home_wins} vs {away_wins}");
        let (home_wins, _, away_wins) = tally(70.0, 80.0);
        assert!(away_wins > home_wins);
    }

    #[test]
    fn equal_teams_score_about_as_many_goals_as_real_football() {
        let mut rng = Rng::new(1);
        let goals: u32 = (0..5000)
            .map(|_| simulate_match(70.0, 70.0, false, &mut rng).goals())
            .map(|(h, a)| (h + a) as u32)
            .sum();
        let per_match = goals as f64 / 5000.0;
        assert!((2.4..3.0).contains(&per_match), "{per_match} goals a match");
    }

    #[test]
    fn a_cup_tie_always_has_a_winner_and_a_league_match_has_no_extra_time() {
        let mut rng = Rng::new(3);
        for _ in 0..2000 {
            let cup = simulate_match(70.0, 70.0, true, &mut rng);
            assert!(cup.winner().is_some());
            let league = simulate_match(70.0, 70.0, false, &mut rng);
            assert!(league.extra_time.is_none() && league.shootout.is_none());
        }
    }

    #[test]
    fn extra_time_and_a_shootout_only_follow_a_level_score() {
        let mut rng = Rng::new(5);
        let mut saw_shootout = false;
        for _ in 0..2000 {
            let o = simulate_match(70.0, 70.0, true, &mut rng);
            if o.full_time.0 != o.full_time.1 {
                assert!(o.extra_time.is_none() && o.shootout.is_none());
            }
            if o.shootout.is_some() {
                saw_shootout = true;
                assert_eq!(o.goals().0, o.goals().1, "a shootout follows a level match");
            }
        }
        assert!(saw_shootout);
    }

    #[test]
    fn the_same_seed_gives_the_same_match() {
        let play = |seed| simulate_match(75.0, 68.0, true, &mut Rng::new(seed));
        assert_eq!(play(4), play(4));
    }

    /// The chance two independent Poisson counts with these averages end level.
    fn independent_draw_chance(home_rate: f64, away_rate: f64) -> f64 {
        let pmf = |rate: f64, k: i32| (-rate).exp() * rate.powi(k) / (1..=k).map(|i| i as f64).product::<f64>();
        (0..20).map(|k| pmf(home_rate, k) * pmf(away_rate, k)).sum()
    }

    #[test]
    fn draws_are_more_common_than_independent_goal_counts_would_give() {
        // Equal teams: the rates are the base goals with the home factor applied each way.
        let (home_rate, away_rate) = (BASE_GOALS * HOME_FACTOR, BASE_GOALS / HOME_FACTOR);
        let independent = independent_draw_chance(home_rate, away_rate);
        let (_, draws, _) = tally(70.0, 70.0);
        let simulated = draws as f64 / 5000.0;
        assert!(simulated > independent + 0.015, "simulated {simulated:.3} against {independent:.3} without the correction");
    }
}
