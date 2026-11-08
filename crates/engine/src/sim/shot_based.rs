//! The shot-based match model: a match is a timeline of shots, fouls and what follows from them. Possession is worked
//! out first from the midfields and decides how many shots each side gets; each shot picks a shooter, then is off
//! target, saved or a goal; fouls bring cards and missed shots bring corners. The score is the goals on the timeline.
//! Who shoots, scores and assists is the `scoring` model's job and who fouls and is carded the `discipline` model's;
//! this file only sequences the timeline. Pure functions, no `World`. Design and real figures: `research/game-features/match-simulation-methods.md`.
use super::cheap::{poisson, shootout};
use super::discipline::{self, Card, DisciplineModel, DEFAULT_DISCIPLINE_MODEL, FOULS_PER_SIDE};
use super::scoring::{self, ScoringModel, DEFAULT_SCORING_MODEL};
use super::{Event, MatchDetail, MatchInput, MatchReport, MatchTeam};
use crate::model::{Line, PlayerId};
use crate::outcome::{Outcome, Side};
use crate::rng::Rng;

// Tuned with `dev-tools/simtool --model shot-based` against the real big-five figures (shots 24.6, 36% on target,
// corners 9.9 per match) and the real tables: see
// `research/game-features/simulation-balance.md`. Rerun the simtool after changing any of them.

/// Shots a side takes in 90 minutes against an equal side with half the ball, before the home bonus.
const BASE_SHOTS: f64 = 13.0;
/// The home side's shots are multiplied by this.
const HOME_SHOT_FACTOR: f64 = 1.17;
/// Each rating point of the attack against the other side's defence changes the shots by this share (as an exponent).
const SHOT_RATE_PER_POINT: f64 = 0.024;
/// Chance a shot is on target.
const ON_TARGET: f64 = 0.36;
/// A side that leads keeps this share of its later shots: teams sit back, which is also why draws are common.
const LEADER_KEEPS_SHOOTING: f64 = 0.5;
/// Chance a shot that is not scored wins a corner.
const CORNER_PER_MISSED_SHOT: f64 = 0.45;
/// Share of the minutes a 90-minute period puts in the first half (real goals: 44.2%).
const FIRST_HALF_SHARE: f64 = 0.42;

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

/// The parts of the match that are separate, swappable models. `DEFAULT_PARTS` is what every competition uses.
#[derive(Clone, Copy, Debug)]
pub struct Parts {
    pub scoring: ScoringModel,
    pub discipline: DisciplineModel,
}

pub const DEFAULT_PARTS: Parts = Parts { scoring: DEFAULT_SCORING_MODEL, discipline: DEFAULT_DISCIPLINE_MODEL };

pub(super) fn simulate(input: &MatchInput, rng: &mut Rng) -> MatchReport {
    simulate_with(input, DEFAULT_PARTS, rng)
}

pub fn simulate_with(input: &MatchInput, parts: Parts, rng: &mut Rng) -> MatchReport {
    let home_possession = home_possession(input, rng);
    let mut events = play_period(input, parts, home_possession, Period::Regulation, (0, 0), &[], rng);
    let full_time = goals(&events);
    let (mut extra_time, mut shootout_score) = (None, None);
    if input.must_have_winner && full_time.0 == full_time.1 {
        let extra_events = play_period(input, parts, home_possession, Period::ExtraTime, full_time, &events, rng);
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

/// Plays one period and returns its events. `score` is the score when the period starts and `earlier` the events before it (whoever was sent off stays off).
fn play_period(input: &MatchInput, parts: Parts, home_possession: f64, period: Period, score: (u8, u8), earlier: &[Event], rng: &mut Rng) -> Vec<Event> {
    let teams = [input.home, input.away];
    let mut timeline = Timeline { teams, parts, score: [score.0, score.1], yellows: [[false; 11]; 2], events: vec![] };
    for e in earlier {
        if let Event::Red { side, player, .. } = *e {
            timeline.send_off(side, player);
        }
    }
    for p in plan(input, home_possession, period, rng) {
        match p.action {
            Action::Shot => timeline.shot(p.minute, p.side, rng),
            Action::Foul => timeline.foul(p.minute, p.side, rng),
        }
    }
    timeline.events
}

/// The match as it is played out, event by event.
struct Timeline {
    /// Home and away as they stand: a player who is sent off is replaced by an empty slot, which never shoots, assists or fouls.
    teams: [MatchTeam; 2],
    parts: Parts,
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

impl Timeline {
    fn shot(&mut self, minute: u8, side: Side, rng: &mut Rng) {
        let (mine, theirs) = (side_index(side), side_index(other(side)));
        if self.score[mine] > self.score[theirs] && rng.unit() >= LEADER_KEEPS_SHOOTING {
            return;
        }
        let (me, them) = (self.teams[mine], self.teams[theirs]);
        let shooter = me.players[scoring::shooter(self.parts.scoring, &me.players, rng)];
        let on_target = rng.unit() < ON_TARGET;
        self.events.push(Event::Shot { minute, side, shooter: shooter.id, on_target });
        if on_target {
            let keeper = them.players[0];
            if rng.unit() < scoring::goal_chance(self.parts.scoring, &shooter, &keeper) {
                self.score[mine] += 1;
                let assister = scoring::assister(self.parts.scoring, &me.players, &shooter, rng).map(|slot| me.players[slot].id);
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
        let players = &self.teams[side_index(side)].players;
        let booked = &mut self.yellows[side_index(side)];
        let foul = discipline::foul(self.parts.discipline, players, booked, rng);
        let player = players[foul.slot].id;
        self.events.push(Event::Foul { minute, side, player });
        match foul.card {
            Card::None => {}
            Card::Yellow => {
                booked[foul.slot] = true;
                self.events.push(Event::Yellow { minute, side, player });
            }
            Card::Red => {
                self.events.push(Event::Red { minute, side, player });
                self.send_off(side, player);
            }
        }
    }

    /// Takes the player off the pitch for the rest of the match. The goalkeeper is never sent off here (he never
    /// fouls), so slot 0 stays and the other side always has a keeper to beat.
    fn send_off(&mut self, side: Side, player: PlayerId) {
        let players = &mut self.teams[side_index(side)].players;
        if let Some(slot) = players.iter().skip(1).position(|p| p.id == player) {
            players[slot + 1].id = 0;
        }
    }
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

    #[test]
    fn every_combination_of_parts_keeps_the_timeline_rules() {
        for &scoring in ScoringModel::ALL {
            for &discipline in DisciplineModel::ALL {
                let parts = Parts { scoring, discipline };
                for seed in 0..100 {
                    let report = simulate_with(&input(72, 70, true), parts, &mut Rng::new(seed));
                    let events = report.detail.expect("detail").events;
                    let count = |f: fn(&Event) -> bool| events.iter().filter(|e| f(e)).count();
                    let on_target = count(|e| matches!(e, Event::Shot { on_target: true, .. }));
                    assert_eq!(on_target, count(|e| matches!(e, Event::Goal { .. })) + count(|e| matches!(e, Event::Save { .. })), "{parts:?} seed {seed}");
                    assert!(events.windows(2).all(|w| minute(&w[0]) <= minute(&w[1])), "{parts:?} seed {seed}");
                }
            }
        }
    }

    #[test]
    fn a_player_sent_off_does_nothing_more_in_the_match_extra_time_included() {
        for seed in 0..400 {
            let events = play(&input(70, 70, true), seed).1.events;
            let mut off = vec![];
            for e in &events {
                match *e {
                    Event::Red { side, player, .. } => {
                        assert!(!off.contains(&(side, player)), "seed {seed}: sent off twice");
                        off.push((side, player));
                    }
                    Event::Shot { side, shooter: p, .. } | Event::Foul { side, player: p, .. } | Event::Yellow { side, player: p, .. } => {
                        assert!(!off.contains(&(side, p)), "seed {seed}: acted after the red card");
                    }
                    Event::Goal { side, scorer, assister, .. } => {
                        assert!(!off.contains(&(side, scorer)) && assister.is_none_or(|a| !off.contains(&(side, a))), "seed {seed}");
                    }
                    _ => {}
                }
            }
        }
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
}
