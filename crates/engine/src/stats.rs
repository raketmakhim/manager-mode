//! Player stats: what each player did, added up from the events of the matches played. A match gives each player in
//! its lineup one `PlayerStats` (a pure function of the match events); `World` adds them to the player's season
//! totals for the competition. Only matches whose model returns events (`MatchDetail`) count.
use crate::development::{match_xp, MatchPlay, DEFAULT_EXPERIENCE_MODEL};
use crate::model::{CompetitionId, PlayerId, Position, World};
use crate::outcome::{Outcome, Side};
use crate::rating::{neutral_rating, rate, RatingInput, RatingModel, TeamResult, DEFAULT_RATING_MODEL};
use crate::rng::Rng;
use crate::sim::{Event, LiveMatch, MatchDetail};
use std::collections::BTreeMap;

/// What a player did, in one match or added up over many. Counts, except `minutes`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerStats {
    pub appearances: u16,
    /// Minutes on the pitch.
    pub minutes: u16,
    pub goals: u16,
    pub assists: u16,
    pub shots: u16,
    pub shots_on_target: u16,
    /// Shots on target stopped (goalkeepers).
    pub saves: u16,
    /// Goals the side let in while the player was in the lineup.
    pub conceded: u16,
    /// Matches of 60 minutes or more without conceding.
    pub clean_sheets: u16,
    pub yellows: u16,
    pub reds: u16,
    /// The sum of the match ratings, in tenths (a 7.3 adds 73), over the `rated` matches.
    pub rating_tenths: u32,
    /// Matches he was rated in (10 minutes or more).
    pub rated: u16,
    /// Matches he was the best player of.
    pub player_of_the_match: u16,
    /// Experience points earned (1 to 10 a match played; see `development`).
    pub xp: u32,
}

/// A clean sheet needs at least this many minutes on the pitch.
const CLEAN_SHEET_MINUTES: u16 = 60;

impl PlayerStats {
    fn add(&mut self, o: &PlayerStats) {
        self.appearances += o.appearances;
        self.minutes += o.minutes;
        self.goals += o.goals;
        self.assists += o.assists;
        self.shots += o.shots;
        self.shots_on_target += o.shots_on_target;
        self.saves += o.saves;
        self.conceded += o.conceded;
        self.clean_sheets += o.clean_sheets;
        self.yellows += o.yellows;
        self.reds += o.reds;
        self.rating_tenths += o.rating_tenths;
        self.rated += o.rated;
        self.player_of_the_match += o.player_of_the_match;
        self.xp += o.xp;
    }

    /// His average match rating (3.0 to 10.0), or `None` if he has not been rated yet.
    pub fn average_rating(&self) -> Option<f32> {
        (self.rated > 0).then(|| self.rating_tenths as f32 / self.rated as f32 / 10.0)
    }
}

/// One player's match while it is being worked out.
struct Row {
    id: PlayerId,
    side: Side,
    position: Position,
    /// His rating in the slot (0-99).
    overall: u8,
    /// The minutes he came on and went off (0 and the length of the match for a starter who played it all).
    from: u16,
    to: u16,
    stats: PlayerStats,
}

/// The stats each player earned in one match, from its events, with his match rating from `rating_model` and a
/// player of the match (the best rating on the winning side, or on the pitch if it was a draw). Extra time counts as
/// minutes; a red card or a substitution ends the player's minutes, and a substitute's start at his minute. Goals
/// conceded are those while he was on the pitch.
pub fn match_stats(detail: &MatchDetail, outcome: &Outcome, rating_model: RatingModel) -> Vec<(PlayerId, PlayerStats)> {
    let length = if outcome.extra_time.is_some() { 120 } else { 90 };
    let mut rows: Vec<Row> = vec![];
    for (side, side_index) in [Side::Home, Side::Away].into_iter().zip(0..) {
        for slot in 0..11 {
            let id = detail.lineups[side_index][slot];
            if id != 0 {
                let stats = PlayerStats { appearances: 1, ..PlayerStats::default() };
                rows.push(Row { id, side, position: detail.positions[side_index][slot], overall: detail.slot_ratings[side_index][slot], from: 0, to: length, stats });
            }
        }
    }
    for event in &detail.events {
        if let Event::Substitution { minute, side, off, on, position, rating } = *event {
            if let Some(row) = rows.iter_mut().find(|r| r.id == off) {
                row.to = row.to.min(minute as u16);
            }
            let stats = PlayerStats { appearances: 1, ..PlayerStats::default() };
            rows.push(Row { id: on, side, position, overall: rating, from: minute as u16, to: length, stats });
            continue;
        }
        let mut change = |id: PlayerId, apply: &dyn Fn(&mut PlayerStats)| {
            if let Some(row) = rows.iter_mut().find(|r| r.id == id) {
                apply(&mut row.stats);
            }
        };
        match *event {
            Event::Goal { scorer, assister, .. } => {
                change(scorer, &|s| s.goals += 1);
                if let Some(assister) = assister {
                    change(assister, &|s| s.assists += 1);
                }
            }
            Event::Shot { shooter, on_target, .. } => change(shooter, &|s| {
                s.shots += 1;
                s.shots_on_target += on_target as u16;
            }),
            Event::Save { keeper, .. } => change(keeper, &|s| s.saves += 1),
            Event::Yellow { player, .. } => change(player, &|s| s.yellows += 1),
            Event::Red { player, minute, .. } => {
                if let Some(row) = rows.iter_mut().find(|r| r.id == player) {
                    row.stats.reds += 1;
                    row.to = row.to.min(minute as u16);
                }
            }
            _ => {}
        }
    }
    for row in rows.iter_mut() {
        row.stats.minutes = row.to - row.from;
        let against = if row.side == Side::Home { Side::Away } else { Side::Home };
        let on_pitch = |minute: u8| row.from < minute as u16 && minute as u16 <= row.to;
        row.stats.conceded = detail.events.iter().filter(|e| matches!(e, Event::Goal { side, minute, .. } if *side == against && on_pitch(*minute))).count() as u16;
        row.stats.clean_sheets = (row.stats.conceded == 0 && row.stats.minutes >= CLEAN_SHEET_MINUTES) as u16;
    }
    let winner = outcome.winner();
    let ratings: Vec<Option<f32>> = rows
        .iter()
        .map(|row| {
            let result = match winner {
                None => TeamResult::Draw,
                Some(winner) if winner == row.side => TeamResult::Win,
                Some(_) => TeamResult::Loss,
            };
            rate(rating_model, &RatingInput { position: row.position, overall: row.overall, minutes: row.stats.minutes, stats: &row.stats, result })
        })
        .collect();
    let best = player_of_the_match(&rows, &ratings, winner);
    for (i, (row, rating)) in rows.iter_mut().zip(&ratings).enumerate() {
        if let Some(rating) = rating {
            row.stats.rating_tenths = (rating * 10.0).round() as u32;
            row.stats.rated = 1;
        }
        row.stats.player_of_the_match = (best == Some(i)) as u16;
        let play = MatchPlay {
            minutes: row.stats.minutes,
            rating: *rating,
            neutral_rating: neutral_rating(rating_model, row.position, row.overall),
            overall: row.overall,
            line: row.position.line(),
            player_of_the_match: best == Some(i),
        };
        row.stats.xp = match_xp(DEFAULT_EXPERIENCE_MODEL, &play) as u32;
    }
    rows.into_iter().map(|r| (r.id, r.stats)).collect()
}

/// The index of the best-rated player of the winning side, or of everyone if nobody won. A tie in rating goes to the
/// one with more goals plus assists; a tie in that too, which is rare, to the earlier row.
fn player_of_the_match(rows: &[Row], ratings: &[Option<f32>], winner: Option<Side>) -> Option<usize> {
    let mut best: Option<(usize, f32, u16)> = None;
    for (i, row) in rows.iter().enumerate() {
        let Some(rating) = ratings[i] else { continue };
        if winner.is_some_and(|w| w != row.side) {
            continue;
        }
        let involved = row.stats.goals + row.stats.assists;
        if best.is_none_or(|(_, top, top_involved)| rating > top || (rating == top && involved > top_involved)) {
            best = Some((i, rating, involved));
        }
    }
    best.map(|(i, _, _)| i)
}

/// The stats every player earned in a live match that is over (see `LiveMatch::is_over`), for the screen that shows
/// the match's XP before the match is stored.
pub fn live_match_stats(live: &LiveMatch) -> Vec<(PlayerId, PlayerStats)> {
    let report = live.clone().into_report(&mut Rng::new(0));
    let detail = report.detail.expect("a live match has events");
    match_stats(&detail, &report.outcome, DEFAULT_RATING_MODEL)
}

impl World {
    /// Adds a played match to its players' totals for this season and competition.
    pub fn record_match(&mut self, competition: CompetitionId, detail: &MatchDetail, outcome: &Outcome) {
        for (player, stats) in match_stats(detail, outcome, DEFAULT_RATING_MODEL) {
            self.stats.entry((player, self.year, competition)).or_default().add(&stats);
            self.last_match_xp.insert(player, stats.xp as u8);
            if let Some(p) = self.players.get_mut(&player) {
                p.xp = p.xp.saturating_add(stats.xp as u16);
            }
        }
    }

    /// Everyone's totals as (player, stats), added up over the seasons and competitions asked for: `None` means all
    /// of them, so `(None, None)` is the whole career.
    // shortcut: scans every stored row; index by (year, competition) if a view that asks every frame gets slow.
    pub fn stats_for(&self, season: Option<u16>, competition: Option<CompetitionId>) -> Vec<(PlayerId, PlayerStats)> {
        let mut totals: BTreeMap<PlayerId, PlayerStats> = BTreeMap::new();
        for ((player, year, c), s) in &self.stats {
            if season.is_none_or(|y| y == *year) && competition.is_none_or(|wanted| wanted == *c) {
                totals.entry(*player).or_default().add(s);
            }
        }
        totals.into_iter().collect()
    }

    /// A player's totals this season over every competition.
    pub fn season_stats(&self, player: PlayerId) -> PlayerStats {
        let mut total = PlayerStats::default();
        for (_, s) in self.stats.range((player, self.year, 0)..=(player, self.year, CompetitionId::MAX)) {
            total.add(s);
        }
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Home players have ids 1-11, away players 12-22; the goalkeepers are 1 and 12.
    fn detail(events: Vec<Event>) -> MatchDetail {
        let lineup = |first: u32| std::array::from_fn(|i| first + i as u32);
        let slots = crate::lineup::Formation::FourFourTwo.slots();
        MatchDetail { home_possession: 50, lineups: [lineup(1), lineup(12)], positions: [slots, slots], slot_ratings: [[70; 11]; 2], events }
    }

    fn row(stats: &[(PlayerId, PlayerStats)], id: PlayerId) -> PlayerStats {
        stats.iter().find(|r| r.0 == id).unwrap().1
    }

    #[test]
    fn a_substitute_plays_from_his_minute_and_concedes_only_what_happens_while_he_is_on() {
        let events = vec![
            Event::Goal { minute: 30, side: Side::Away, scorer: 20, assister: None },
            Event::Substitution { minute: 60, side: Side::Home, off: 9, on: 30, position: Position::St, rating: 68 },
            Event::Goal { minute: 70, side: Side::Home, scorer: 30, assister: None },
        ];
        let stats = match_stats(&detail(events), &Outcome::full_time(1, 1), RatingModel::Formula);
        assert_eq!(stats.len(), 23);
        let (off, on) = (row(&stats, 9), row(&stats, 30));
        assert_eq!((off.minutes, off.appearances, off.conceded), (60, 1, 1));
        assert_eq!((on.minutes, on.appearances, on.goals, on.conceded), (30, 1, 1, 0));
        // Played 30 minutes of a match without conceding, but a clean sheet needs 60.
        assert_eq!(on.clean_sheets, 0);
        assert!(on.rated == 1, "30 minutes is enough to be rated");
    }

    #[test]
    fn a_goal_gives_the_scorer_a_goal_the_setter_up_an_assist_and_the_other_side_a_conceded() {
        let events = vec![
            Event::Shot { minute: 10, side: Side::Home, shooter: 9, on_target: true },
            Event::Goal { minute: 10, side: Side::Home, scorer: 9, assister: Some(6) },
        ];
        let stats = match_stats(&detail(events), &Outcome::full_time(1, 0), RatingModel::Formula);
        assert_eq!(stats.len(), 22);
        let scorer = row(&stats, 9);
        assert_eq!((scorer.goals, scorer.shots, scorer.shots_on_target, scorer.appearances, scorer.minutes), (1, 1, 1, 1, 90));
        assert_eq!(row(&stats, 6).assists, 1);
        assert_eq!((row(&stats, 12).conceded, row(&stats, 12).clean_sheets), (1, 0));
        assert_eq!((row(&stats, 1).conceded, row(&stats, 1).clean_sheets), (0, 1));
    }

    #[test]
    fn a_red_card_ends_the_minutes_and_extra_time_adds_thirty() {
        let red = vec![Event::Red { minute: 55, side: Side::Away, player: 15 }];
        let stats = match_stats(&detail(red), &Outcome::full_time(0, 0), RatingModel::Formula);
        assert_eq!((row(&stats, 15).minutes, row(&stats, 15).reds), (55, 1));
        assert_eq!(row(&stats, 14).minutes, 90);
        let aet = Outcome { full_time: (0, 0), extra_time: Some((0, 0)), shootout: Some((4, 3)) };
        assert_eq!(row(&match_stats(&detail(vec![]), &aet, RatingModel::Formula), 3).minutes, 120);
    }

    #[test]
    fn an_empty_slot_is_not_an_appearance_and_totals_add_up_across_matches() {
        let mut d = detail(vec![Event::Save { minute: 30, side: Side::Away, keeper: 12 }]);
        d.lineups[0][10] = 0;
        assert_eq!(match_stats(&d, &Outcome::full_time(0, 0), RatingModel::Formula).len(), 21);

        let mut w = World::new(0, 2014);
        w.record_match(7, &d, &Outcome::full_time(0, 0));
        w.record_match(7, &d, &Outcome::full_time(0, 0));
        w.record_match(8, &d, &Outcome::full_time(0, 0));
        assert_eq!(w.stats_for(Some(2014), Some(7)).iter().find(|r| r.0 == 12).unwrap().1.saves, 2);
        assert_eq!(w.stats_for(None, None).iter().find(|r| r.0 == 12).unwrap().1.saves, 3);
        assert!(w.stats_for(Some(2015), None).is_empty());
        assert_eq!(w.season_stats(12).saves, 3);
        assert_eq!(w.season_stats(12).appearances, 3);
        assert_eq!(w.season_stats(999), PlayerStats::default());
    }

    #[test]
    fn a_match_gives_ratings_to_the_players_and_a_player_of_the_match_on_the_winning_side() {
        let events = vec![
            Event::Shot { minute: 10, side: Side::Home, shooter: 9, on_target: true },
            Event::Goal { minute: 10, side: Side::Home, scorer: 9, assister: Some(6) },
        ];
        let stats = match_stats(&detail(events), &Outcome::full_time(1, 0), RatingModel::Formula);
        assert!(stats.iter().all(|(_, s)| s.rated == 1 && s.average_rating().is_some()));
        assert_eq!(stats.iter().map(|(_, s)| s.player_of_the_match).sum::<u16>(), 1, "one player of the match");
        assert_eq!(row(&stats, 9).player_of_the_match, 1, "the scorer of the only goal wins it");
        assert!(row(&stats, 9).average_rating() > row(&stats, 12).average_rating(), "the scorer beats the beaten side");
        // A draw: the player of the match can come from either side.
        let draw = match_stats(&detail(vec![]), &Outcome::full_time(0, 0), RatingModel::Formula);
        assert_eq!(draw.iter().map(|(_, s)| s.player_of_the_match).sum::<u16>(), 1);
    }

    #[test]
    fn a_sent_off_player_under_ten_minutes_is_not_rated() {
        let red = vec![Event::Red { minute: 5, side: Side::Home, player: 4 }];
        let stats = match_stats(&detail(red), &Outcome::full_time(0, 0), RatingModel::Formula);
        assert_eq!((row(&stats, 4).rated, row(&stats, 4).average_rating()), (0, None));
        assert_eq!(row(&stats, 5).rated, 1);
    }

    #[test]
    fn simulated_ratings_look_like_real_ones() {
        // Eight equal clubs play a season; real ratings of starters average about 6.5, most lie between 5.5 and 7.5,
        // and forwards do not beat defenders by much (research/game-features/match-ratings.md, section 4).
        let mut w = World::new(0, 2014);
        for id in 1..=8 {
            crate::testing::add_club_with_eleven(&mut w, id, 1, 70);
        }
        let clubs = (1..=8).collect();
        let rules = crate::competition::Rules::double_round_robin();
        w.competitions.insert(1, crate::competition::Competition { id: 1, name: "L".into(), country: "X".into(), rules, level: None, clubs });
        let mut rng = crate::Rng::new(3);
        w.start_season(&mut rng);
        w.play_season_to_end(&mut rng);
        let mut by_line: std::collections::BTreeMap<&str, Vec<f32>> = Default::default();
        for (id, s) in w.stats_for(None, Some(1)) {
            let line = match w.players[&id].positions[0].line() {
                crate::Line::Goalkeeper => "GK",
                crate::Line::Defence => "DEF",
                crate::Line::Midfield => "MID",
                crate::Line::Attack => "ATT",
            };
            by_line.entry(line).or_default().push(s.average_rating().unwrap());
        }
        let mean = |v: &[f32]| v.iter().sum::<f32>() / v.len() as f32;
        let all: Vec<f32> = by_line.values().flatten().copied().collect();
        assert!((6.3..6.7).contains(&mean(&all)), "mean {:.2}", mean(&all));
        assert!(all.iter().all(|r| (5.5..7.5).contains(r)), "season averages stay between 5.5 and 7.5: {all:?}");
        let means: Vec<String> = by_line.iter().map(|(line, v)| format!("{line} {:.2}", mean(v))).collect();
        assert!((mean(&by_line["ATT"]) - mean(&by_line["DEF"])).abs() < 0.3, "by line: {means:?}");
    }

    #[test]
    fn equal_ratings_for_the_player_of_the_match_go_to_the_one_with_more_goals_and_assists() {
        let row = |id, side, goals, assists| Row { id, side, position: Position::St, overall: 70, from: 0, to: 90, stats: PlayerStats { goals, assists, ..PlayerStats::default() } };
        let rows = [row(1, Side::Home, 0, 0), row(2, Side::Home, 1, 1), row(3, Side::Home, 1, 0)];
        let equal = [Some(7.0), Some(7.0), Some(7.0)];
        assert_eq!(player_of_the_match(&rows, &equal, Some(Side::Home)), Some(1), "two goal contributions beat one and none");
        let identical = [row(1, Side::Home, 0, 0), row(2, Side::Away, 0, 0)];
        assert_eq!(player_of_the_match(&identical, &equal[..2], None), Some(0), "a complete tie goes to the earlier row");
        assert_eq!(player_of_the_match(&identical, &[Some(6.0), Some(8.0)], None), Some(1), "a better rating wins whatever the goals");
    }
}
