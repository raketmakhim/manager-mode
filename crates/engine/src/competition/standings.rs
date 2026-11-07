//! League tables. A pure function from results to rows, so any format that needs a table (a league, a group,
//! a league phase) can share it.
use crate::model::{ClubId, CompetitionId, Fixture, World};
use crate::outcome::Side;
use std::collections::BTreeMap;

const POINTS_FOR_A_WIN: u16 = 3;
const POINTS_FOR_A_DRAW: u16 = 1;

/// One club's line in a table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableRow {
    pub club: ClubId,
    /// Matches played.
    pub played: u8,
    pub won: u8,
    pub drawn: u8,
    pub lost: u8,
    pub goals_for: u16,
    pub goals_against: u16,
    pub points: u16,
}

impl TableRow {
    fn new(club: ClubId) -> TableRow {
        TableRow { club, played: 0, won: 0, drawn: 0, lost: 0, goals_for: 0, goals_against: 0, points: 0 }
    }

    /// Goals scored minus goals conceded.
    pub fn goal_difference(&self) -> i32 {
        self.goals_for as i32 - self.goals_against as i32
    }

    /// Adds one played match from this club's side.
    fn add(&mut self, scored: u8, conceded: u8, won: Option<bool>) {
        self.played += 1;
        self.goals_for += scored as u16;
        self.goals_against += conceded as u16;
        match won {
            Some(true) => (self.won, self.points) = (self.won + 1, self.points + POINTS_FOR_A_WIN),
            Some(false) => self.lost += 1,
            None => (self.drawn, self.points) = (self.drawn + 1, self.points + POINTS_FOR_A_DRAW),
        }
    }
}

/// The table for `clubs` from the played matches among `fixtures`, best first: points, then goal difference, then
/// goals scored, then club id (so the order is always the same). Unplayed matches and matches against clubs outside
/// `clubs` are ignored.
// shortcut: one tie-break order for every league; Spain and others use head-to-head first. Upgrade when the
// order becomes a field of `Rules`.
pub fn standings(clubs: &[ClubId], fixtures: &[&Fixture]) -> Vec<TableRow> {
    let mut rows: BTreeMap<ClubId, TableRow> = clubs.iter().map(|&c| (c, TableRow::new(c))).collect();
    for f in fixtures {
        let Some(outcome) = f.result else { continue };
        if !rows.contains_key(&f.home) || !rows.contains_key(&f.away) {
            continue;
        }
        let (home_goals, away_goals) = outcome.goals();
        let winner = outcome.winner();
        rows.get_mut(&f.home).unwrap().add(home_goals, away_goals, winner.map(|w| w == Side::Home));
        rows.get_mut(&f.away).unwrap().add(away_goals, home_goals, winner.map(|w| w == Side::Away));
    }
    let mut table: Vec<TableRow> = rows.into_values().collect();
    table.sort_by_key(|r| (std::cmp::Reverse(r.points), std::cmp::Reverse(r.goal_difference()), std::cmp::Reverse(r.goals_for), r.club));
    table
}

impl World {
    /// The current table of a competition, from the matches played so far. Empty for an unknown competition.
    pub fn table(&self, competition: CompetitionId) -> Vec<TableRow> {
        let Some(c) = self.competitions.get(&competition) else { return vec![] };
        let fixtures: Vec<&Fixture> = self.fixtures.iter().filter(|f| f.competition == competition).collect();
        standings(&c.clubs, &fixtures)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Day;
    use crate::outcome::Outcome;

    fn played(home: ClubId, away: ClubId, goals: (u8, u8)) -> Fixture {
        Fixture { competition: 1, round: 1, week: 1, day: Day::Saturday, home, away, result: Some(Outcome::full_time(goals.0, goals.1)) }
    }

    fn table(fixtures: &[Fixture]) -> Vec<TableRow> {
        standings(&[1, 2, 3], &fixtures.iter().collect::<Vec<_>>())
    }

    #[test]
    fn a_win_is_3_points_a_draw_1_and_a_loss_0() {
        let t = table(&[played(1, 2, (2, 0)), played(3, 1, (1, 1))]);
        let of = |club| t.iter().find(|r| r.club == club).unwrap().clone();
        assert_eq!((of(1).points, of(1).won, of(1).drawn, of(1).played), (4, 1, 1, 2));
        assert_eq!((of(2).points, of(2).lost, of(2).goals_against), (0, 1, 2));
        assert_eq!((of(3).points, of(3).goal_difference()), (1, 0));
    }

    #[test]
    fn ties_break_on_goal_difference_then_goals_scored_then_club_id() {
        // Everyone ends on 3 points: 1 beat 2, 2 beat 3, 3 beat 1.
        let t = table(&[played(1, 2, (4, 0)), played(2, 3, (1, 0)), played(3, 1, (1, 0))]);
        // Goal differences: club 1 = +3, club 2 = -3, club 3 = 0 ... so 1, 3, 2.
        assert_eq!(t.iter().map(|r| r.club).collect::<Vec<_>>(), vec![1, 3, 2]);
        // Identical records: the lower club id comes first.
        let t = table(&[played(1, 2, (1, 1))]);
        assert_eq!(t.iter().map(|r| r.club).collect::<Vec<_>>(), vec![1, 2, 3]);
    }

    #[test]
    fn unplayed_matches_and_outside_clubs_are_ignored() {
        let mut unplayed = played(1, 2, (0, 0));
        unplayed.result = None;
        let t = table(&[unplayed, played(1, 99, (5, 0))]);
        assert!(t.iter().all(|r| r.played == 0));
        assert_eq!(t.len(), 3, "every club has a row, even before a match");
    }

    #[test]
    fn a_shootout_win_counts_as_a_win_and_its_goals_do_not_count() {
        let o = Outcome { full_time: (1, 1), extra_time: Some((0, 0)), shootout: Some((4, 3)) };
        let mut f = played(1, 2, (0, 0));
        f.result = Some(o);
        let t = table(&[f]);
        let one = t.iter().find(|r| r.club == 1).unwrap();
        assert_eq!((one.won, one.goals_for), (1, 1));
    }
}
