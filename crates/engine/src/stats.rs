//! Player stats: what each player did, added up from the events of the matches played. A match gives each player in
//! its lineup one `PlayerStats` (a pure function of the match events); `World` adds them to the player's season
//! totals for the competition. Only matches whose model returns events (`MatchDetail`) count.
use crate::model::{CompetitionId, PlayerId, World};
use crate::outcome::{Outcome, Side};
use crate::sim::{Event, MatchDetail};
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
    }
}

/// The stats each starter earned in one match, from its events. Extra time counts as minutes; a red card ends the
/// player's minutes.
// shortcut: no substitutions yet, so everyone in the lineup plays the whole match unless sent off; goals conceded
// count for the whole lineup even after a red card. Revisit with substitutions.
pub fn match_stats(detail: &MatchDetail, outcome: &Outcome) -> Vec<(PlayerId, PlayerStats)> {
    let length = if outcome.extra_time.is_some() { 120 } else { 90 };
    let mut rows: Vec<(PlayerId, Side, PlayerStats)> = vec![];
    for (lineup, side) in detail.lineups.iter().zip([Side::Home, Side::Away]) {
        for &id in lineup.iter().filter(|&&id| id != 0) {
            rows.push((id, side, PlayerStats { appearances: 1, minutes: length, ..PlayerStats::default() }));
        }
    }
    for event in &detail.events {
        let mut change = |id: PlayerId, apply: &dyn Fn(&mut PlayerStats)| {
            if let Some(row) = rows.iter_mut().find(|r| r.0 == id) {
                apply(&mut row.2);
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
            Event::Red { player, minute, .. } => change(player, &|s| {
                s.reds += 1;
                s.minutes = s.minutes.min(minute as u16);
            }),
            _ => {}
        }
    }
    let goals_by = |side: Side| detail.events.iter().filter(|e| matches!(e, Event::Goal { side: s, .. } if *s == side)).count() as u16;
    for (_, side, s) in rows.iter_mut() {
        s.conceded = goals_by(if *side == Side::Home { Side::Away } else { Side::Home });
        s.clean_sheets = (s.conceded == 0 && s.minutes >= CLEAN_SHEET_MINUTES) as u16;
    }
    rows.into_iter().map(|(id, _, s)| (id, s)).collect()
}

impl World {
    /// Adds a played match to its players' totals for this season and competition.
    pub fn record_match(&mut self, competition: CompetitionId, detail: &MatchDetail, outcome: &Outcome) {
        for (player, stats) in match_stats(detail, outcome) {
            self.stats.entry((player, self.year, competition)).or_default().add(&stats);
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
        MatchDetail { home_possession: 50, lineups: [lineup(1), lineup(12)], events }
    }

    fn row(stats: &[(PlayerId, PlayerStats)], id: PlayerId) -> PlayerStats {
        stats.iter().find(|r| r.0 == id).unwrap().1
    }

    #[test]
    fn a_goal_gives_the_scorer_a_goal_the_setter_up_an_assist_and_the_other_side_a_conceded() {
        let events = vec![
            Event::Shot { minute: 10, side: Side::Home, shooter: 9, on_target: true },
            Event::Goal { minute: 10, side: Side::Home, scorer: 9, assister: Some(6) },
        ];
        let stats = match_stats(&detail(events), &Outcome::full_time(1, 0));
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
        let stats = match_stats(&detail(red), &Outcome::full_time(0, 0));
        assert_eq!((row(&stats, 15).minutes, row(&stats, 15).reds), (55, 1));
        assert_eq!(row(&stats, 14).minutes, 90);
        let aet = Outcome { full_time: (0, 0), extra_time: Some((0, 0)), shootout: Some((4, 3)) };
        assert_eq!(row(&match_stats(&detail(vec![]), &aet), 3).minutes, 120);
    }

    #[test]
    fn an_empty_slot_is_not_an_appearance_and_totals_add_up_across_matches() {
        let mut d = detail(vec![Event::Save { minute: 30, side: Side::Away, keeper: 12 }]);
        d.lineups[0][10] = 0;
        assert_eq!(match_stats(&d, &Outcome::full_time(0, 0)).len(), 21);

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
}
