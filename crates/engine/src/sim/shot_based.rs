//! The shot-based match model: a match is played minute by minute, and in each minute a side may take shots and
//! commit fouls. Possession is worked out first from the midfields and decides how many shots each side gets; each
//! shot picks a shooter, then is off target, saved or a goal; fouls bring cards and missed shots bring corners.
//! The score is the goals so far. Who shoots, scores and assists is the `scoring` model's job, who fouls and is
//! carded the `discipline` model's and who is changed when the `substitution` model's; this file only sequences
//! the minutes. A `LiveMatch` can be advanced a minute at a time and changed by the user in between; `simulate_with`
//! plays one out at once. Pure functions and plain state, no `World`. Design and real figures:
//! `research/game-features/match-simulation-methods.md`.
use super::cheap::{poisson, shootout};
use super::discipline::{self, Card, DisciplineModel, DEFAULT_DISCIPLINE_MODEL, FOULS_PER_SIDE};
use super::fatigue::{self, FatigueModel, DEFAULT_FATIGUE_MODEL};
use super::scoring::{self, ScoringModel, DEFAULT_SCORING_MODEL};
use super::substitution::{self, SubstitutionModel, DEFAULT_SUBSTITUTION_MODEL};
use super::{Event, MatchDetail, MatchInput, MatchPlayer, MatchReport, MatchTeam};
use crate::model::{Line, PlayerId, Position};
use crate::outcome::{Outcome, Side};
use crate::rng::Rng;
use std::ops::RangeInclusive;

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
/// Share of the events of 90 minutes that fall in the first half (real goals: 44.2%).
const FIRST_HALF_SHARE: f64 = 0.41;
/// Extra time is 30 minutes, a third of a match, so a third of the events.
const EXTRA_TIME_SHARE: f64 = 1.0 / 3.0;
/// The sides' strengths are worked out again every this many minutes (and at once after a change or a red card).
const STRENGTH_REFRESH_MINUTES: u8 = 5;

/// The home side's share of the ball against an equal side: a little over half.
const HOME_POSSESSION: f64 = 0.52;
/// Each rating point of the midfield above the other midfield adds this share of the ball.
const POSSESSION_PER_POINT: f64 = 0.008;
/// How far chance moves possession in one match (a triangular spread of plus or minus this).
const POSSESSION_NOISE: f64 = 0.06;
/// The share of the ball is kept between these (a match is never one-sided beyond this).
const POSSESSION_RANGE: (f64, f64) = (0.30, 0.70);

/// The share of a 90-minute rate that falls in `minute` (1 to 120): late minutes of each half are likelier (real
/// goals come late). Each half's minutes rise in a straight line, so the share of minute k of n is (k/n)^2 less
/// ((k-1)/n)^2.
fn minute_weight(minute: u8) -> f64 {
    let rising = |k: u8, length: f64| (k as f64 / length).powi(2) - ((k - 1) as f64 / length).powi(2);
    match minute {
        1..=45 => FIRST_HALF_SHARE * rising(minute, 45.0),
        46..=90 => (1.0 - FIRST_HALF_SHARE) * rising(minute - 45, 45.0),
        _ => EXTRA_TIME_SHARE * rising(minute - 90, 30.0),
    }
}

/// The parts of the match that are separate, swappable models. `DEFAULT_PARTS` is what every competition uses.
#[derive(Clone, Copy, Debug)]
pub struct Parts {
    pub scoring: ScoringModel,
    pub discipline: DisciplineModel,
    pub substitution: SubstitutionModel,
    pub fatigue: FatigueModel,
}

pub const DEFAULT_PARTS: Parts = Parts { scoring: DEFAULT_SCORING_MODEL, discipline: DEFAULT_DISCIPLINE_MODEL, substitution: DEFAULT_SUBSTITUTION_MODEL, fatigue: DEFAULT_FATIGUE_MODEL };

pub(super) fn simulate(input: &MatchInput, rng: &mut Rng) -> MatchReport {
    simulate_with(input, DEFAULT_PARTS, rng)
}

/// Plays a whole match with the given parts, nobody changing the sides by hand.
pub fn simulate_with(input: &MatchInput, parts: Parts, rng: &mut Rng) -> MatchReport {
    LiveMatch::new(input, parts, [false, false], rng).into_report(rng)
}

fn other(side: Side) -> Side {
    match side {
        Side::Home => Side::Away,
        Side::Away => Side::Home,
    }
}

fn side_index(side: Side) -> usize {
    match side {
        Side::Home => 0,
        Side::Away => 1,
    }
}

/// The mean rating of the players of one line; an empty slot (nobody, or a player sent off) counts as 0.
fn line_mean(team: &MatchTeam, line: Line) -> f64 {
    let (sum, count) = team.players.iter().filter(|p| p.position.line() == line).fold((0.0, 0), |(sum, count), p| (sum + p.rating as f64, count + 1));
    if count == 0 { 0.0 } else { sum / count as f64 }
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

/// Why a change by hand was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubstitutionError {
    /// The side has used all the changes the rules allow.
    NoneLeft,
    /// The player to take off is not on the pitch.
    NotOnPitch,
    /// The player to bring on is not on the bench (or has already been on).
    NotOnBench,
    /// A goalkeeper may only be replaced by a goalkeeper and a goalkeeper may only replace a goalkeeper.
    Keeper,
}

/// Why a change of positions by hand was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TacticsError {
    /// One of the players is not on the pitch.
    NotOnPitch,
    /// A goalkeeper may not swap with an outfield player.
    Keeper,
}

/// A match being played, a minute at a time. The user can change a side between minutes.
#[derive(Clone, Debug)]
pub struct LiveMatch {
    /// The sides as they started, for the lineups in the report.
    input: MatchInput,
    parts: Parts,
    /// Home and away as they stand: a player who is sent off becomes an empty slot, which never shoots, assists or
    /// fouls and has no rating.
    teams: [MatchTeam; 2],
    home_possession: f64,
    /// Goals so far: home, away.
    score: [u8; 2],
    /// Which players of each side (by slot) are on a yellow card.
    yellows: [[bool; 11]; 2],
    substitutions_used: [u8; 2],
    /// The minute each player of each side (by slot) came on: 0 for a starter.
    came_on: [[u8; 11]; 2],
    /// (attack, defence) of each side as the players are now, tired ones rated lower; worked out again every few minutes.
    strength: [(f64, f64); 2],
    /// The minutes at which a side that the `substitution` model runs makes a change; empty for a side run by the user.
    planned_changes: [Vec<u8>; 2],
    /// Minutes played so far: 0 before kick-off, 90 at full time, up to 120 after extra time.
    minute: u8,
    events: Vec<Event>,
}

impl LiveMatch {
    /// A match about to kick off. `by_hand[side]` is true for a side whose changes the user makes.
    pub fn new(input: &MatchInput, parts: Parts, by_hand: [bool; 2], rng: &mut Rng) -> LiveMatch {
        let home_possession = home_possession(input, rng);
        let planned_changes = by_hand.map(|by_hand| if by_hand { vec![] } else { substitution::plan(parts.substitution, input.max_substitutions, rng) });
        let mut live = LiveMatch {
            input: *input,
            parts,
            teams: [input.home, input.away],
            home_possession,
            score: [0, 0],
            yellows: [[false; 11]; 2],
            substitutions_used: [0, 0],
            came_on: [[0; 11]; 2],
            strength: [(0.0, 0.0); 2],
            planned_changes,
            minute: 0,
            events: vec![],
        };
        live.refresh_strength();
        live
    }

    /// Minutes played so far.
    pub fn minute(&self) -> u8 {
        self.minute
    }

    /// Goals so far: (home, away).
    pub fn score(&self) -> (u8, u8) {
        (self.score[0], self.score[1])
    }

    /// Everything that has happened, in time order.
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// A side as it stands: who is on the pitch in each slot and who is left on the bench (id 0 is an empty place).
    pub fn team(&self, side: Side) -> &MatchTeam {
        &self.teams[side_index(side)]
    }

    /// How tired each player of a side is now, by slot, from 0 (fresh) to 1 (spent).
    pub fn tiredness(&self, side: Side) -> [f64; 11] {
        let i = side_index(side);
        let ball_share = self.ball_share(side);
        std::array::from_fn(|slot| fatigue::tiredness(self.parts.fatigue, &self.teams[i].players[slot], self.minute.saturating_sub(self.came_on[i][slot]), ball_share))
    }

    /// Changes a side may still make.
    pub fn substitutions_left(&self, side: Side) -> u8 {
        self.input.max_substitutions.saturating_sub(self.substitutions_used[side_index(side)])
    }

    /// True once the final whistle has gone: 90 minutes (a cup tie that is level plays 30 more).
    pub fn is_over(&self) -> bool {
        match self.minute {
            0..=89 => false,
            90 => !(self.input.must_have_winner && self.score[0] == self.score[1]),
            91..=119 => false,
            _ => true,
        }
    }

    /// Plays the next minute. Does nothing once the match is over.
    pub fn advance(&mut self, rng: &mut Rng) {
        if self.is_over() {
            return;
        }
        self.minute += 1;
        let minute = self.minute;
        if minute.is_multiple_of(STRENGTH_REFRESH_MINUTES) {
            self.refresh_strength();
        }
        for side in [Side::Home, Side::Away] {
            let due = self.planned_changes[side_index(side)].iter().filter(|&&m| m == minute).count();
            for _ in 0..due {
                self.automatic_change(minute, side, rng);
            }
        }
        let weight = minute_weight(minute);
        for side in [Side::Home, Side::Away] {
            for _ in 0..poisson(self.shot_rate(side) * weight, rng) {
                self.shot(minute, side, rng);
            }
            for _ in 0..poisson(FOULS_PER_SIDE * weight, rng) {
                self.foul(minute, side, rng);
            }
        }
    }

    /// Swaps the slots of two players on the pitch. Each plays at his own rating in the other's slot. A goalkeeper
    /// cannot swap with an outfield player.
    pub fn swap_positions(&mut self, side: Side, a: PlayerId, b: PlayerId) -> Result<(), TacticsError> {
        let i = side_index(side);
        let players = &self.teams[i].players;
        let find = |id: PlayerId| players.iter().position(|p| p.id == id && !p.is_empty());
        let (Some(slot_a), Some(slot_b)) = (find(a), find(b)) else { return Err(TacticsError::NotOnPitch) };
        if (slot_a == 0) != (slot_b == 0) {
            return Err(TacticsError::Keeper);
        }
        let players = &mut self.teams[i].players;
        let (position_a, position_b) = (players[slot_a].position, players[slot_b].position);
        players.swap(slot_a, slot_b);
        for (slot, position) in [(slot_a, position_a), (slot_b, position_b)] {
            players[slot].position = position;
            players[slot].rating = players[slot].ratings[position.index()];
        }
        self.yellows[i].swap(slot_a, slot_b);
        self.came_on[i].swap(slot_a, slot_b);
        self.refresh_strength();
        Ok(())
    }

    /// Changes a side to a new formation (`slots`, the goalkeeper first, as `Formation::slots` gives them). The
    /// goalkeeper stays; the outfield players go to the slots where they are rated highest: the best (player, slot)
    /// match first, then the next, ties to the lower player id and the earlier slot, as in `lineup::pick_lineup`. A
    /// player's card and the minute he came on go with him. A side with a player sent off leaves one slot empty.
    pub fn set_formation(&mut self, side: Side, slots: [Position; 11]) {
        let i = side_index(side);
        let (old, booked, came_on) = (self.teams[i].players, self.yellows[i], self.came_on[i]);
        let mut pairs: Vec<(u8, PlayerId, usize, usize)> = vec![]; // (rating in the new slot, player id, old slot, new slot)
        for from in (1..11).filter(|&from| !old[from].is_empty()) {
            for (to, position) in slots.iter().enumerate().skip(1) {
                pairs.push((old[from].ratings[position.index()], old[from].id, from, to));
            }
        }
        pairs.sort_by_key(|&(rating, id, _, to)| (std::cmp::Reverse(rating), id, to));
        let (mut filled, mut moved) = ([None; 11], [false; 11]);
        for (_, _, from, to) in pairs {
            if filled[to].is_none() && !moved[from] {
                (filled[to], moved[from]) = (Some(from), true);
            }
        }
        self.teams[i].players[0].position = slots[0];
        for to in 1..11 {
            let player = filled[to].map_or(MatchPlayer::empty(slots[to]), |from| MatchPlayer { position: slots[to], rating: old[from].ratings[slots[to].index()], ..old[from] });
            (self.teams[i].players[to], self.yellows[i][to], self.came_on[i][to]) = (player, filled[to].is_some_and(|from| booked[from]), filled[to].map_or(0, |from| came_on[from]));
        }
        self.refresh_strength();
    }

    /// Hands a side run by hand over to the `substitution` model for the rest of the match, as if the user stopped
    /// choosing: it makes the changes it has left, at minutes after now.
    pub fn hand_over(&mut self, side: Side, rng: &mut Rng) {
        let left = self.substitutions_left(side) as usize;
        let later = substitution::plan(self.parts.substitution, self.input.max_substitutions, rng).into_iter().filter(|&m| m > self.minute);
        self.planned_changes[side_index(side)] = later.take(left).collect();
    }

    /// Replaces player `off` by bench player `on` for `side`, now (between the minute just played and the next).
    pub fn substitute(&mut self, side: Side, off: PlayerId, on: PlayerId) -> Result<(), SubstitutionError> {
        if self.substitutions_left(side) == 0 {
            return Err(SubstitutionError::NoneLeft);
        }
        let team = &self.teams[side_index(side)];
        let slot = team.players.iter().position(|p| p.id == off && !p.is_empty()).ok_or(SubstitutionError::NotOnPitch)?;
        let place = team.bench.iter().position(|p| p.id == on && !p.is_empty()).ok_or(SubstitutionError::NotOnBench)?;
        if (slot == 0) != (team.bench[place].position == Position::Gk) {
            return Err(SubstitutionError::Keeper);
        }
        self.change(self.minute, side, off, on);
        Ok(())
    }

    /// Plays whatever is left of the match and gives the report: the score, with extra time and a shootout for a cup
    /// tie that needs them, and the events.
    pub fn into_report(mut self, rng: &mut Rng) -> MatchReport {
        while !self.is_over() {
            self.advance(rng);
        }
        let full_time = goals(&self.events, 1..=90);
        let (mut extra_time, mut shootout_score) = (None, None);
        if self.minute > 90 {
            let extra = goals(&self.events, 91..=120);
            extra_time = Some(extra);
            shootout_score = (extra.0 == extra.1).then(|| shootout(rng));
        }
        let lineups = [self.input.home, self.input.away].map(|t| t.players.map(|p| p.id));
        let positions = [self.input.home, self.input.away].map(|t| t.players.map(|p| p.position));
        let slot_ratings = [self.input.home, self.input.away].map(|t| t.ratings());
        let detail = MatchDetail { home_possession: (self.home_possession * 100.0).round() as u8, lineups, positions, slot_ratings, events: self.events };
        MatchReport { outcome: Outcome { full_time, extra_time, shootout: shootout_score }, detail: Some(detail) }
    }

    /// A side's share of the ball (0 to 1) in this match.
    fn ball_share(&self, side: Side) -> f64 {
        match side {
            Side::Home => self.home_possession,
            Side::Away => 1.0 - self.home_possession,
        }
    }

    /// Works out each side's attack and defence from its players' ratings less their tiredness.
    fn refresh_strength(&mut self) {
        for side in [Side::Home, Side::Away] {
            let i = side_index(side);
            let mut tired = self.teams[i];
            let ball_share = self.ball_share(side);
            for (slot, p) in tired.players.iter_mut().enumerate() {
                p.rating = fatigue::tired_rating(self.parts.fatigue, p, self.minute.saturating_sub(self.came_on[i][slot]), ball_share);
            }
            self.strength[i] = (attack(&tired), defence(&tired));
        }
    }

    /// Shots a side takes in 90 minutes at the strengths on the pitch now.
    fn shot_rate(&self, side: Side) -> f64 {
        let home_bonus = if side == Side::Home { HOME_SHOT_FACTOR } else { 1.0 };
        let (attack, defence) = (self.strength[side_index(side)].0, self.strength[side_index(other(side))].1);
        BASE_SHOTS * home_bonus * (SHOT_RATE_PER_POINT * (attack - defence)).exp() * 2.0 * self.ball_share(side)
    }

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

    /// A change the `substitution` model makes for a side the user does not run.
    fn automatic_change(&mut self, minute: u8, side: Side, rng: &mut Rng) {
        let team = self.teams[side_index(side)];
        let Some(change) = substitution::choose(self.parts.substitution, &team, &self.tiredness(side), rng) else { return };
        self.change(minute, side, team.players[change.slot].id, team.bench[change.bench].id);
    }

    /// Puts bench player `on` in the slot of `off`, who leaves the match, and records it. The newcomer starts with a
    /// clean card and plays at his own rating in that slot.
    fn change(&mut self, minute: u8, side: Side, off: PlayerId, on: PlayerId) {
        let (team, booked) = (&mut self.teams[side_index(side)], &mut self.yellows[side_index(side)]);
        let (Some(slot), Some(place)) = (team.players.iter().position(|p| p.id == off), team.bench.iter().position(|p| p.id == on)) else { return };
        let (position, newcomer) = (team.players[slot].position, team.bench[place]);
        let rating = newcomer.ratings[position.index()];
        team.players[slot] = MatchPlayer { position, rating, ..newcomer };
        team.bench[place].id = 0;
        booked[slot] = false;
        self.came_on[side_index(side)][slot] = minute;
        self.substitutions_used[side_index(side)] += 1;
        self.events.push(Event::Substitution { minute, side, off, on, position, rating });
        self.refresh_strength();
    }

    /// Takes the player off the pitch for the rest of the match. The goalkeeper is never sent off here (he never
    /// fouls), so slot 0 stays and the other side always has a keeper to beat.
    fn send_off(&mut self, side: Side, player: PlayerId) {
        let players = &mut self.teams[side_index(side)].players;
        if let Some(slot) = players.iter().skip(1).position(|p| p.id == player) {
            players[slot + 1].id = 0;
            players[slot + 1].rating = 0;
            self.refresh_strength();
        }
    }
}

/// Goals scored by each side in the given minutes: (home, away).
fn goals(events: &[Event], minutes: RangeInclusive<u8>) -> (u8, u8) {
    let count = |wanted: Side| events.iter().filter(|e| matches!(e, Event::Goal { side, minute, .. } if *side == wanted && minutes.contains(minute))).count() as u8;
    (count(Side::Home), count(Side::Away))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(home: u8, away: u8, must_have_winner: bool) -> MatchInput {
        MatchInput { home: MatchTeam::flat(home), away: MatchTeam::flat(away), must_have_winner, max_substitutions: 5 }
    }

    #[test]
    fn every_combination_of_parts_keeps_the_timeline_rules() {
        for &scoring in ScoringModel::ALL {
            for &discipline in DisciplineModel::ALL {
                for &substitution in SubstitutionModel::ALL {
                    let parts = Parts { scoring, discipline, substitution, fatigue: DEFAULT_FATIGUE_MODEL };
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
    }

    #[test]
    fn only_players_on_the_pitch_act_and_a_player_who_has_left_never_returns() {
        for seed in 0..400 {
            let detail = play(&input(70, 70, true), seed).1;
            // Who is on the pitch, by side; the keepers have id 0 in the flat test teams, so skip the id 0.
            let mut on: Vec<(Side, PlayerId)> = [Side::Home, Side::Away].iter().zip(detail.lineups).flat_map(|(&s, ids)| ids.map(move |id| (s, id))).collect();
            for e in &detail.events {
                match *e {
                    Event::Red { side, player, .. } => {
                        assert!(on.contains(&(side, player)), "seed {seed}: sent off while off the pitch");
                        on.retain(|x| *x != (side, player));
                    }
                    Event::Substitution { side, off, on: coming, .. } => {
                        assert!(on.contains(&(side, off)) && !on.contains(&(side, coming)), "seed {seed}: bad change");
                        on.retain(|x| *x != (side, off));
                        on.push((side, coming));
                    }
                    Event::Shot { side, shooter: p, .. } | Event::Foul { side, player: p, .. } | Event::Yellow { side, player: p, .. } => {
                        assert!(on.contains(&(side, p)), "seed {seed}: {p} acted while off the pitch");
                    }
                    Event::Goal { side, scorer, assister, .. } => {
                        assert!(on.contains(&(side, scorer)) && assister.is_none_or(|a| on.contains(&(side, a))), "seed {seed}");
                    }
                    _ => {}
                }
            }
        }
    }

    fn live(input: &MatchInput, by_hand: [bool; 2], seed: u64) -> (LiveMatch, Rng) {
        let mut rng = Rng::new(seed);
        (LiveMatch::new(input, DEFAULT_PARTS, by_hand, &mut rng), rng)
    }

    #[test]
    fn a_league_match_ends_at_90_minutes_and_a_level_cup_tie_goes_on_to_120() {
        for seed in 0..100 {
            let (mut league, mut rng) = live(&input(70, 70, false), [false, false], seed);
            while !league.is_over() {
                league.advance(&mut rng);
            }
            assert_eq!(league.minute(), 90);
            let (mut cup, mut rng) = live(&input(70, 70, true), [false, false], seed);
            while !cup.is_over() {
                cup.advance(&mut rng);
            }
            let at_90 = goals(cup.events(), 1..=90);
            assert_eq!(cup.minute(), if at_90.0 == at_90.1 { 120 } else { 90 }, "seed {seed}");
        }
    }

    #[test]
    fn a_side_run_by_hand_changes_nobody_until_the_user_does() {
        let (mut m, mut rng) = live(&input(70, 70, false), [true, false], 1);
        while !m.is_over() {
            m.advance(&mut rng);
        }
        let changes = |side| m.events().iter().filter(|e| matches!(e, Event::Substitution { side: s, .. } if *s == side)).count();
        assert_eq!(changes(Side::Home), 0);
        assert!(changes(Side::Away) > 0);
    }

    #[test]
    fn a_side_handed_over_makes_its_remaining_changes_later_and_never_more_than_it_has_left() {
        let (mut m, mut rng) = live(&input(70, 70, false), [true, false], 7);
        for _ in 0..30 {
            m.advance(&mut rng);
        }
        let (off, on) = (m.team(Side::Home).players[10].id, m.team(Side::Home).bench[6].id);
        m.substitute(Side::Home, off, on).unwrap();
        m.hand_over(Side::Home, &mut rng);
        while !m.is_over() {
            m.advance(&mut rng);
        }
        let home: Vec<u8> = m.events().iter().filter_map(|e| match *e { Event::Substitution { side: Side::Home, minute, .. } => Some(minute), _ => None }).collect();
        assert!(home[0] == 30 && home[1..].iter().all(|&minute| minute > 30), "{home:?}");
        assert!(home.len() <= 5, "{home:?}");
    }

    #[test]
    fn a_change_by_hand_swaps_the_players_and_is_refused_when_it_breaks_the_rules() {
        let (mut m, mut rng) = live(&input(70, 70, false), [true, true], 2);
        for _ in 0..60 {
            m.advance(&mut rng);
        }
        let (home, bench) = (m.team(Side::Home).players, m.team(Side::Home).bench);
        let (striker, forward_sub) = (home[10], bench[6]);
        assert_eq!(m.substitute(Side::Home, 999, forward_sub.id), Err(SubstitutionError::NotOnPitch));
        assert_eq!(m.substitute(Side::Home, striker.id, 999), Err(SubstitutionError::NotOnBench));
        assert_eq!(m.substitute(Side::Home, striker.id, bench[0].id), Err(SubstitutionError::Keeper), "a keeper cannot replace an outfield player");
        assert_eq!(m.substitute(Side::Home, home[0].id, forward_sub.id), Err(SubstitutionError::Keeper), "an outfield player cannot replace the keeper");
        assert_eq!(m.substitute(Side::Home, striker.id, forward_sub.id), Ok(()));
        assert_eq!(m.team(Side::Home).players[10].id, forward_sub.id);
        assert_eq!(m.team(Side::Home).bench[6].id, 0, "he has used his place on the bench");
        assert_eq!(m.substitutions_left(Side::Home), 4);
        assert_eq!(m.substitute(Side::Home, striker.id, forward_sub.id), Err(SubstitutionError::NotOnPitch), "the man who went off cannot be changed again");
        assert!(matches!(m.events().last(), Some(Event::Substitution { minute: 60, side: Side::Home, off, on, .. }) if *off == striker.id && *on == forward_sub.id));
        // All five changes used: the sixth is refused.
        for i in 1..5 {
            let (off, on) = (m.team(Side::Home).players[i].id, m.team(Side::Home).bench[i].id);
            assert_eq!(m.substitute(Side::Home, off, on), Ok(()), "change {i}");
        }
        assert_eq!(m.substitute(Side::Home, m.team(Side::Home).players[5].id, m.team(Side::Home).bench[5].id), Err(SubstitutionError::NoneLeft));
    }

    #[test]
    fn a_substitute_plays_at_his_own_rating_in_the_slot_he_takes() {
        let mut i = input(70, 70, false);
        i.home.bench[6].ratings[Position::Lb.index()] = 40; // the bench striker is poor at left back
        let mut rng = Rng::new(3);
        let mut m = LiveMatch::new(&i, DEFAULT_PARTS, [true, true], &mut rng);
        let (back, striker) = (m.team(Side::Home).players[1], m.team(Side::Home).bench[6]);
        assert_eq!(back.position, Position::Lb);
        m.substitute(Side::Home, back.id, striker.id).unwrap();
        assert_eq!((m.team(Side::Home).players[1].rating, m.team(Side::Home).players[1].position), (40, Position::Lb));
    }

    #[test]
    fn two_players_swapped_take_each_others_slot_at_their_own_rating() {
        let mut i = input(70, 70, false);
        i.home.players[10].ratings[Position::Cm.index()] = 50; // the striker (slot 10) is poor in midfield
        let mut rng = Rng::new(4);
        let mut m = LiveMatch::new(&i, DEFAULT_PARTS, [true, true], &mut rng);
        let (midfielder, striker) = (m.team(Side::Home).players[6], m.team(Side::Home).players[10]);
        (m.yellows[0][6], m.came_on[0][10]) = (true, 55); // the midfielder is booked and the striker came on at 55
        assert_eq!(m.swap_positions(Side::Home, midfielder.id, striker.id), Ok(()));
        assert_eq!((m.yellows[0][10], m.yellows[0][6], m.came_on[0][6], m.came_on[0][10]), (true, false, 55, 0), "the card and the minute follow the player");
        let (team, keeper) = (*m.team(Side::Home), m.team(Side::Home).players[0].id);
        assert_eq!((team.players[6].id, team.players[6].position, team.players[6].rating), (striker.id, midfielder.position, 50));
        assert_eq!((team.players[10].id, team.players[10].position), (midfielder.id, striker.position));
        assert_eq!(m.swap_positions(Side::Home, midfielder.id, 999), Err(TacticsError::NotOnPitch));
        assert_eq!(m.swap_positions(Side::Home, keeper, striker.id), Err(TacticsError::Keeper));
    }

    #[test]
    fn a_new_formation_puts_every_outfield_player_in_a_slot_once_and_keeps_the_keeper() {
        let (mut m, _) = live(&input(70, 70, false), [true, true], 5);
        let before: Vec<PlayerId> = m.team(Side::Home).players.iter().map(|p| p.id).collect();
        let slots = crate::lineup::Formation::ALL.iter().find(|f| f.slots() != m.team(Side::Home).players.map(|p| p.position)).unwrap().slots();
        let (booked, late) = (m.team(Side::Home).players[3].id, m.team(Side::Home).players[8].id);
        (m.yellows[0][3], m.came_on[0][8]) = (true, 70);
        m.set_formation(Side::Home, slots);
        let slot_of = |m: &LiveMatch, id| m.team(Side::Home).players.iter().position(|p| p.id == id).unwrap();
        assert!(m.yellows[0][slot_of(&m, booked)] && m.yellows[0].iter().filter(|&&y| y).count() == 1, "the card follows the player");
        assert_eq!(m.came_on[0][slot_of(&m, late)], 70, "so does the minute he came on");
        let team = m.team(Side::Home);
        let mut after: Vec<PlayerId> = team.players.iter().map(|p| p.id).collect();
        assert_eq!(after[0], before[0], "the keeper stays in goal");
        assert_eq!(team.players.map(|p| p.position), slots);
        let (mut a, mut b) = (after.clone(), before.clone());
        a.sort();
        b.sort();
        assert_eq!(a, b, "the same eleven");
        after.dedup();
        assert_eq!(after.len(), 11);
    }

    #[test]
    fn a_new_formation_with_a_player_sent_off_leaves_one_slot_empty() {
        let (mut m, _) = live(&input(70, 70, false), [true, true], 6);
        let defender = m.team(Side::Home).players[2].id;
        m.send_off(Side::Home, defender);
        let slots = crate::lineup::Formation::ALL.iter().find(|f| f.slots() != m.team(Side::Home).players.map(|p| p.position)).unwrap().slots();
        m.set_formation(Side::Home, slots);
        assert_eq!(m.team(Side::Home).players.iter().filter(|p| p.is_empty()).count(), 1);
    }

    #[test]
    fn a_player_sent_off_weakens_his_side_where_he_played() {
        let (mut m, _) = live(&input(70, 70, false), [true, true], 4);
        let (home_before, away_before) = (m.shot_rate(Side::Home), m.shot_rate(Side::Away));
        let defender = m.team(Side::Home).players[2].id;
        m.send_off(Side::Home, defender);
        assert!(m.shot_rate(Side::Away) > away_before, "a defender less: the other side shoots more");
        assert_eq!(m.shot_rate(Side::Home), home_before, "his own attack is the same");
        let forward = m.team(Side::Home).players[10].id;
        m.send_off(Side::Home, forward);
        assert!(m.shot_rate(Side::Home) < home_before, "a forward less: his side shoots less");
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
            Event::Foul { minute, .. } | Event::Yellow { minute, .. } | Event::Red { minute, .. } | Event::Substitution { minute, .. } => minute,
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
        let flat = MatchTeam::flat(70);
        let line_of = |id: u32| flat.players.iter().chain(&flat.bench).find(|p| p.id == id).expect("a player of the flat team").position.line();
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
