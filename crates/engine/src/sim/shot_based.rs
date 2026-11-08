//! The shot-based match model: a match is a timeline of shots, fouls and what follows from them. Possession is worked
//! out first from the midfields and decides how many shots each side gets; each shot picks a shooter, then is off
//! target, saved or a goal; fouls bring cards and missed shots bring corners. The score is the goals on the timeline.
//! Pure functions, no `World`. Design and real figures: `research/game-features/match-simulation-methods.md`.
use super::cheap::{poisson, shootout};
use super::{Event, MatchDetail, MatchInput, MatchPlayer, MatchReport, MatchTeam};
use crate::model::{Line, Position};
use crate::outcome::{Outcome, Side};
use crate::rng::Rng;

// Tuned with `dev-tools/simtool --model shot-based` against the real big-five figures (shots 24.6, 36% on target,
// corners 9.9, fouls 25.3, yellow 4.10, red 0.20 per match) and the real tables: see
// `research/game-features/simulation-balance.md`. Rerun the simtool after changing any of them.

/// Shots a side takes in 90 minutes against an equal side with half the ball, before the home bonus.
const BASE_SHOTS: f64 = 13.0;
/// The home side's shots are multiplied by this.
const HOME_SHOT_FACTOR: f64 = 1.17;
/// Each rating point of the attack against the other side's defence changes the shots by this share (as an exponent).
const SHOT_RATE_PER_POINT: f64 = 0.024;
/// Chance a shot is on target.
const ON_TARGET: f64 = 0.36;
/// Chance a shot on target is a goal, for a shooter with the reference finishing against a keeper of the reference rating.
const GOAL_PER_SHOT_ON_TARGET: f64 = 0.296;
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
/// The same for setting up goals, from creativity.
const ASSISTS_PER_CREATIVITY_POINT: f64 = 0.05;
/// A side that leads keeps this share of its later shots: teams sit back, which is also why draws are common.
const LEADER_KEEPS_SHOOTING: f64 = 0.5;
/// Chance a shot that is not scored wins a corner.
const CORNER_PER_MISSED_SHOT: f64 = 0.45;
/// Chance a goal is set up by a teammate.
const GOAL_ASSISTED: f64 = 0.70;
/// Share of the minutes a 90-minute period puts in the first half (real goals: 44.2%).
const FIRST_HALF_SHARE: f64 = 0.42;

/// Fouls a side commits in 90 minutes.
const FOULS_PER_SIDE: f64 = 12.65;
/// Chance a foul is booked, and chance it is a straight red card.
const YELLOW_PER_FOUL: f64 = 0.18;
const RED_PER_FOUL: f64 = 0.004;
/// Each point of aggression multiplies a player's share of his side's fouls by this (as an exponent): a 75 commits
/// about 2.5 times the fouls of a 50. Real Messi is booked about 0.09 times a match and Ronaldo 0.125, against about
/// 0.2 for an average outfield player (research/game-features/simulation-balance.md).
const FOULS_PER_AGGRESSION_POINT: f64 = 0.037;
/// No player commits more than this share of his side's fouls.
const MAX_FOUL_SHARE: f64 = 0.2;
/// A player already on a yellow card is booked this share as often: he plays more carefully. Without it second
/// yellows make red cards about two and a half times as common as in real football.
const BOOKED_CAUTION: f64 = 0.25;

/// The home side's share of the ball against an equal side: a little over half.
const HOME_POSSESSION: f64 = 0.52;
/// Each rating point of the midfield above the other midfield adds this share of the ball.
const POSSESSION_PER_POINT: f64 = 0.008;
/// How far chance moves possession in one match (a triangular spread of plus or minus this).
const POSSESSION_NOISE: f64 = 0.06;
/// The share of the ball is kept between these (a match is never one-sided beyond this).
const POSSESSION_RANGE: (f64, f64) = (0.30, 0.70);

/// Part of a match that is played out: the 90 minutes, or the 30 of extra time.
#[derive(Clone, Copy)]
enum Period {
    Regulation,
    ExtraTime,
}

impl Period {
    /// Its length against 90 minutes, which scales how many things happen.
    fn share(self) -> f64 {
        match self {
            Period::Regulation => 1.0,
            Period::ExtraTime => 1.0 / 3.0,
        }
    }

    /// A random minute in the period; late minutes of each half are likelier (real goals come late).
    fn minute(self, rng: &mut Rng) -> u8 {
        let rising = |length: f64, rng: &mut Rng| 1 + ((length * rng.unit().sqrt()) as u8).min(length as u8 - 1);
        match self {
            Period::Regulation if rng.unit() < FIRST_HALF_SHARE => rising(45.0, rng),
            Period::Regulation => 45 + rising(45.0, rng),
            Period::ExtraTime => 90 + rising(30.0, rng),
        }
    }
}

/// Something a side does in the timeline, before it is played out.
#[derive(Clone, Copy)]
enum Action {
    Shot,
    Foul,
}

struct Planned {
    minute: u8,
    side: Side,
    action: Action,
}

pub(super) fn simulate(input: &MatchInput, rng: &mut Rng) -> MatchReport {
    let home_possession = home_possession(input, rng);
    let mut events = play_period(input, home_possession, Period::Regulation, (0, 0), rng);
    let full_time = goals(&events);
    let (mut extra_time, mut shootout_score) = (None, None);
    if input.must_have_winner && full_time.0 == full_time.1 {
        let extra_events = play_period(input, home_possession, Period::ExtraTime, full_time, rng);
        let extra = goals(&extra_events);
        events.extend(extra_events);
        extra_time = Some(extra);
        shootout_score = (extra.0 == extra.1).then(|| shootout(rng));
    }
    let lineups = [input.home, input.away].map(|t| t.players.map(|p| p.id));
    let positions = [input.home, input.away].map(|t| t.players.map(|p| p.position));
    let slot_ratings = [input.home, input.away].map(|t| t.ratings());
    let detail = MatchDetail { home_possession: (home_possession * 100.0).round() as u8, lineups, positions, slot_ratings, events };
    MatchReport { outcome: Outcome { full_time, extra_time, shootout: shootout_score }, detail: Some(detail) }
}

fn team(input: &MatchInput, side: Side) -> &MatchTeam {
    match side {
        Side::Home => &input.home,
        Side::Away => &input.away,
    }
}

fn other(side: Side) -> Side {
    match side {
        Side::Home => Side::Away,
        Side::Away => Side::Home,
    }
}

fn line_mean(team: &MatchTeam, line: Line) -> f64 {
    let ratings: Vec<f64> = team.players.iter().filter(|p| p.position.line() == line).map(|p| p.rating as f64).collect();
    if ratings.is_empty() { 0.0 } else { ratings.iter().sum::<f64>() / ratings.len() as f64 }
}

/// What the side threatens with: the forwards count double the midfield.
fn attack(team: &MatchTeam) -> f64 {
    (2.0 * line_mean(team, Line::Attack) + line_mean(team, Line::Midfield)) / 3.0
}

/// What the side defends with: the back line counts double the midfield.
fn defence(team: &MatchTeam) -> f64 {
    (2.0 * line_mean(team, Line::Defence) + line_mean(team, Line::Midfield)) / 3.0
}

/// The home side's share of the ball (0 to 1) from the two midfields, plus chance.
fn home_possession(input: &MatchInput, rng: &mut Rng) -> f64 {
    let gap = line_mean(&input.home, Line::Midfield) - line_mean(&input.away, Line::Midfield);
    let chance = rng.unit() - rng.unit();
    (HOME_POSSESSION + POSSESSION_PER_POINT * gap + POSSESSION_NOISE * chance).clamp(POSSESSION_RANGE.0, POSSESSION_RANGE.1)
}

/// Every shot and foul of a period with its minute, in time order.
fn plan(input: &MatchInput, home_possession: f64, period: Period, rng: &mut Rng) -> Vec<Planned> {
    let mut planned = vec![];
    for side in [Side::Home, Side::Away] {
        let (me, them) = (team(input, side), team(input, other(side)));
        let (possession, home_bonus) = match side {
            Side::Home => (home_possession, HOME_SHOT_FACTOR),
            Side::Away => (1.0 - home_possession, 1.0),
        };
        let shot_rate = BASE_SHOTS * home_bonus * (SHOT_RATE_PER_POINT * (attack(me) - defence(them))).exp() * 2.0 * possession;
        for (action, rate) in [(Action::Shot, shot_rate), (Action::Foul, FOULS_PER_SIDE)] {
            for _ in 0..poisson(rate * period.share(), rng) {
                planned.push(Planned { minute: period.minute(rng), side, action });
            }
        }
    }
    planned.sort_by_key(|p| p.minute);
    planned
}

/// Plays one period and returns its events. `score` is the score when the period starts.
fn play_period(input: &MatchInput, home_possession: f64, period: Period, score: (u8, u8), rng: &mut Rng) -> Vec<Event> {
    let mut timeline = Timeline { input, score: [score.0, score.1], yellows: [[false; 11]; 2], events: vec![] };
    for p in plan(input, home_possession, period, rng) {
        match p.action {
            Action::Shot => timeline.shot(p.minute, p.side, rng),
            Action::Foul => timeline.foul(p.minute, p.side, rng),
        }
    }
    timeline.events
}

/// The match as it is played out, event by event.
struct Timeline<'a> {
    input: &'a MatchInput,
    /// Goals so far: home, away.
    score: [u8; 2],
    /// Which players of each side (by slot) are on a yellow card.
    yellows: [[bool; 11]; 2],
    events: Vec<Event>,
}

fn side_index(side: Side) -> usize {
    match side {
        Side::Home => 0,
        Side::Away => 1,
    }
}

impl Timeline<'_> {
    fn shot(&mut self, minute: u8, side: Side, rng: &mut Rng) {
        let (mine, theirs) = (side_index(side), side_index(other(side)));
        if self.score[mine] > self.score[theirs] && rng.unit() >= LEADER_KEEPS_SHOOTING {
            return;
        }
        let (me, them) = (team(self.input, side), team(self.input, other(side)));
        let shooter = me.players[pick(&me.players, shot_weight, MAX_SHOT_SHARE, rng)];
        let on_target = rng.unit() < ON_TARGET;
        self.events.push(Event::Shot { minute, side, shooter: shooter.id, on_target });
        if on_target {
            let keeper = them.players[0];
            let finishing = (FINISHING_PER_POINT * (shooter.finishing as f64 - REFERENCE_RATING)).exp().clamp(FINISHING_FACTOR_RANGE.0, FINISHING_FACTOR_RANGE.1);
            let keeping = (KEEPER_PER_POINT * (REFERENCE_RATING - keeper.rating as f64)).exp().clamp(KEEPER_FACTOR_RANGE.0, KEEPER_FACTOR_RANGE.1);
            if rng.unit() < GOAL_PER_SHOT_ON_TARGET * finishing * keeping {
                self.score[mine] += 1;
                let assister = (rng.unit() < GOAL_ASSISTED).then(|| {
                    let weight = |p: &MatchPlayer| if p.id == shooter.id { 0.0 } else { assist_weight(p) };
                    me.players[pick(&me.players, weight, MAX_ASSIST_SHARE, rng)].id
                });
                self.events.push(Event::Goal { minute, side, scorer: shooter.id, assister });
                return;
            }
            self.events.push(Event::Save { minute, side: other(side), keeper: keeper.id });
        }
        if rng.unit() < CORNER_PER_MISSED_SHOT {
            self.events.push(Event::Corner { minute, side });
        }
    }

    fn foul(&mut self, minute: u8, side: Side, rng: &mut Rng) {
        let players = &team(self.input, side).players;
        let slot = pick(players, foul_weight, MAX_FOUL_SHARE, rng);
        let player = players[slot].id;
        self.events.push(Event::Foul { minute, side, player });
        let booked = &mut self.yellows[side_index(side)][slot];
        let caution = if *booked { BOOKED_CAUTION } else { 1.0 };
        if rng.unit() < YELLOW_PER_FOUL * caution {
            let second = std::mem::replace(booked, true);
            self.events.push(if second { Event::Red { minute, side, player } } else { Event::Yellow { minute, side, player } });
        } else if rng.unit() < RED_PER_FOUL {
            self.events.push(Event::Red { minute, side, player });
        }
    }
}

/// How likely a player is to commit a foul: more for an aggressive one. The goalkeeper and an empty slot never do.
fn foul_weight(p: &MatchPlayer) -> f64 {
    if p.is_empty() || p.position == Position::Gk {
        return 0.0;
    }
    (FOULS_PER_AGGRESSION_POINT * p.aggression as f64).exp()
}

/// How likely a player is to take a shot: the shots per 90 minutes of his slot, raised by how well he finishes.
// The three line averages (defenders 0.48, midfielders 1.45, attackers 2.81) are real Premier League 2014/15 figures
// (research/game-features/match-simulation-methods.md). The split inside the midfield and attack is our own guess,
// chosen so a 4-4-2's midfield still averages 1.45: wide and attacking midfielders shoot about twice as often as
// central ones.
fn shot_weight(p: &MatchPlayer) -> f64 {
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
    per_90 * (SHOTS_PER_FINISHING_POINT * p.finishing as f64).exp()
}

/// How likely a player is to set up a goal: the assists per 90 minutes of his slot, raised by his creativity. The
/// line averages (defenders 0.04, midfielders 0.12, attackers 0.11) are real; the split inside them is our own guess.
fn assist_weight(p: &MatchPlayer) -> f64 {
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
    per_90 * (ASSISTS_PER_CREATIVITY_POINT * p.creativity as f64).exp()
}

/// The index of a player chosen at random with chances in proportion to `weight`, except that nobody gets more than
/// `max_share` of the chances (the rest is shared by the others in proportion). If every weight is 0, the first.
fn pick(players: &[MatchPlayer; 11], weight: impl Fn(&MatchPlayer) -> f64, max_share: f64, rng: &mut Rng) -> usize {
    let mut weights = players.each_ref().map(weight);
    // Cutting one player down can push another over the limit, so go round a few times.
    for _ in 0..3 {
        let total: f64 = weights.iter().sum();
        for w in weights.iter_mut().filter(|w| **w > max_share * total) {
            *w = max_share / (1.0 - max_share) * (total - *w);
        }
    }
    let mut left = rng.unit() * weights.iter().sum::<f64>();
    for (i, w) in weights.iter().enumerate() {
        left -= w;
        if left < 0.0 {
            return i;
        }
    }
    0
}

/// Goals scored by each side: (home, away).
fn goals(events: &[Event]) -> (u8, u8) {
    let count = |wanted: Side| events.iter().filter(|e| matches!(e, Event::Goal { side, .. } if *side == wanted)).count() as u8;
    (count(Side::Home), count(Side::Away))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(home: u8, away: u8, must_have_winner: bool) -> MatchInput {
        MatchInput { home: MatchTeam::flat(home), away: MatchTeam::flat(away), must_have_winner }
    }

    fn play(input: &MatchInput, seed: u64) -> (MatchReport, MatchDetail) {
        let report = simulate(input, &mut Rng::new(seed));
        let detail = report.detail.clone().expect("the shot-based model gives detail");
        (report, detail)
    }

    #[test]
    fn events_come_in_time_order_and_every_goal_follows_a_shot_by_the_scorer() {
        for seed in 0..200 {
            let (_, detail) = play(&input(72, 70, true), seed);
            assert!(detail.events.windows(2).all(|w| minute(&w[0]) <= minute(&w[1])), "seed {seed}");
            for (i, e) in detail.events.iter().enumerate() {
                if let Event::Goal { side, scorer, assister, .. } = *e {
                    assert!(matches!(detail.events[i - 1], Event::Shot { side: s, shooter, on_target: true, .. } if s == side && shooter == scorer));
                    assert_ne!(assister, Some(scorer), "nobody sets up his own goal");
                }
            }
        }
    }

    fn minute(e: &Event) -> u8 {
        match *e {
            Event::Goal { minute, .. } | Event::Shot { minute, .. } | Event::Save { minute, .. } | Event::Corner { minute, .. } => minute,
            Event::Foul { minute, .. } | Event::Yellow { minute, .. } | Event::Red { minute, .. } => minute,
        }
    }

    #[test]
    fn possession_goes_to_the_stronger_midfield_and_stays_in_range() {
        let (mut strong, mut weak) = (0, 0);
        for seed in 0..300 {
            let (_, d) = play(&input(80, 70, false), seed);
            assert!((30..=70).contains(&d.home_possession));
            strong += d.home_possession as u32;
            let (_, d) = play(&input(70, 80, false), seed);
            weak += d.home_possession as u32;
        }
        assert!(strong > weak + 300 * 10, "{strong} against {weak}");
    }

    #[test]
    fn forwards_score_far_more_than_defenders() {
        let line_of = |id: u32| MatchTeam::flat(70).players[id as usize].position.line();
        let (mut attackers, mut defenders) = (0, 0);
        for seed in 0..500 {
            for e in play(&input(70, 70, false), seed).1.events {
                if let Event::Goal { scorer, .. } = e {
                    match line_of(scorer) {
                        Line::Attack => attackers += 1,
                        Line::Defence => defenders += 1,
                        _ => {}
                    }
                }
            }
        }
        // Two forwards against four defenders on the pitch.
        assert!(attackers / 2 > 3 * (defenders / 4), "{attackers} by forwards, {defenders} by defenders");
    }

    #[test]
    fn every_shot_on_target_is_a_goal_or_a_save_and_a_second_yellow_is_a_red() {
        for seed in 0..300 {
            let events = play(&input(70, 70, false), seed).1.events;
            let count = |f: fn(&Event) -> bool| events.iter().filter(|e| f(e)).count();
            let on_target = count(|e| matches!(e, Event::Shot { on_target: true, .. }));
            assert_eq!(on_target, count(|e| matches!(e, Event::Goal { .. })) + count(|e| matches!(e, Event::Save { .. })));
            // Each player is booked once; a second booking is shown as a red card.
            let mut booked = vec![];
            for e in &events {
                if let Event::Yellow { side, player, .. } = *e {
                    assert!(!booked.contains(&(side, player)), "seed {seed}: booked twice");
                    booked.push((side, player));
                }
            }
        }
    }

    #[test]
    fn nobody_takes_more_than_the_largest_share_of_the_shots() {
        let mut team = MatchTeam::flat(60);
        team.players[10].finishing = 99; // one star striker in a weak side
        let mut rng = Rng::new(1);
        let star = (0..5000).filter(|_| pick(&team.players, shot_weight, MAX_SHOT_SHARE, &mut rng) == 10).count() as f64 / 5000.0;
        assert!((MAX_SHOT_SHARE - 0.03..MAX_SHOT_SHARE + 0.03).contains(&star), "the star took {star} of the shots");
    }

    #[test]
    fn an_empty_slot_never_shoots_or_sets_up_a_goal() {
        let mut team = MatchTeam::flat(70);
        team.players[10] = MatchPlayer { id: 0, position: Position::St, rating: 0, finishing: 0, creativity: 0, aggression: 0 };
        assert_eq!((shot_weight(&team.players[10]), assist_weight(&team.players[10])), (0.0, 0.0));
        let mut rng = Rng::new(2);
        assert!((0..2000).all(|_| pick(&team.players, shot_weight, MAX_SHOT_SHARE, &mut rng) != 10));
    }

    #[test]
    fn aggressive_players_foul_more_and_the_goalkeeper_never_does() {
        let mut team = MatchTeam::flat(70);
        team.players[5].aggression = 90;
        team.players[6].aggression = 30;
        assert!(foul_weight(&team.players[5]) > 4.0 * foul_weight(&team.players[6]));
        assert_eq!(foul_weight(&team.players[0]), 0.0, "slot 0 is the goalkeeper");
        let mut rng = Rng::new(4);
        let mut fouls = [0u32; 11];
        for _ in 0..5000 {
            fouls[pick(&team.players, foul_weight, MAX_FOUL_SHARE, &mut rng)] += 1;
        }
        assert_eq!(fouls[0], 0);
        assert!(fouls[5] > 3 * fouls[6], "{} fouls against {}", fouls[5], fouls[6]);
        assert!(fouls[5] as f64 / 5000.0 <= MAX_FOUL_SHARE + 0.03, "no one takes more than the share limit");
    }
}
