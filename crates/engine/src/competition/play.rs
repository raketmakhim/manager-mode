//! Playing fixtures: one match, one week, or everything up to a club's next match.
use crate::model::{ClubId, World};
use crate::rng::Rng;
use crate::sim::{simulate, MatchInput, MatchTeam};

impl World {
    /// Plays the fixture at `index` in `fixtures` with its competition's match model and stores the result. Cup ties
    /// always get a winner.
    // shortcut: recomputes both lineups on every match (a whole-world season of about 10,000 matches takes 0.5 s in a
    // release build, 10 s in debug); cache per club, cleared when ratings or squads change, when the simtool runs
    // thousands of seasons. The event detail of a full-model match is dropped here; keep it for the matches that
    // matter (the user's club, the top competitions) once something shows it.
    pub fn play_fixture(&mut self, index: usize, rng: &mut Rng) {
        let f = &self.fixtures[index];
        let competition = &self.competitions[&f.competition];
        let input = self.match_input(f.home, f.away, competition.is_cup());
        let report = simulate(competition.rules.model, &input, rng);
        self.fixtures[index].result = Some(report.outcome);
    }

    /// Plays every unplayed match of `week`, then draws any cup round that this completes.
    pub fn play_week(&mut self, week: u8, rng: &mut Rng) {
        for index in 0..self.fixtures.len() {
            if self.fixtures[index].week == week && self.fixtures[index].result.is_none() {
                self.play_fixture(index, rng);
            }
        }
        self.advance_cups(rng);
    }

    /// Plays every match that is left, week by week, so cup rounds still get drawn in order.
    pub fn play_season_to_end(&mut self, rng: &mut Rng) {
        while let Some(week) = self.next_unplayed_week() {
            self.play_week(week, rng);
        }
    }

    /// Plays on until the club's next unplayed match has been played: the earlier weeks for everyone, then that
    /// match and the other clubs' matches of its week. If the club has a second match that week it stays unplayed
    /// for the next call. Returns the played fixture's index, or `None` if the club has nothing left to play.
    pub fn play_next_match(&mut self, club: ClubId, rng: &mut Rng) -> Option<usize> {
        let next = (0..self.fixtures.len())
            .filter(|&i| self.fixtures[i].result.is_none() && self.fixtures[i].involves(club))
            .min_by_key(|&i| (self.fixtures[i].week, self.fixtures[i].day))?;
        let week = self.fixtures[next].week;
        // Weeks one at a time, so a cup round drawn after week 3 is placed after week 3 and still gets played in order.
        while let Some(w) = self.next_unplayed_week().filter(|&w| w < week) {
            self.play_week(w, rng);
        }
        for i in 0..self.fixtures.len() {
            let f = &self.fixtures[i];
            if f.week == week && f.result.is_none() && (i == next || !f.involves(club)) {
                self.play_fixture(i, rng);
            }
        }
        self.advance_cups(rng);
        Some(next)
    }

    /// What a match model needs to know about the two clubs: their lineups, each player rated in his slot.
    fn match_input(&self, home: ClubId, away: ClubId, must_have_winner: bool) -> MatchInput {
        MatchInput {
            home: MatchTeam { ratings: self.lineup_ratings(home) },
            away: MatchTeam { ratings: self.lineup_ratings(away) },
            must_have_winner,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::competition::{Competition, Rules};
    use crate::testing::add_club_with_eleven;

    /// A world with one league of `n` clubs, club `i` rated 50 + i.
    fn world(n: u32) -> World {
        let mut w = World::new(0, 0);
        for id in 1..=n {
            add_club_with_eleven(&mut w, id, 1, 50 + id as u8);
        }
        let clubs = (1..=n).collect();
        w.competitions.insert(1, Competition { id: 1, name: "L".into(), country: "X".into(), rules: Rules::double_round_robin(), level: None, clubs });
        w.start_season(&mut Rng::new(1));
        w
    }

    #[test]
    fn playing_the_next_match_plays_everything_before_it_and_nothing_after() {
        let mut w = world(8);
        let mut rng = Rng::new(2);
        let index = w.play_next_match(1, &mut rng).unwrap();
        let week = w.fixtures[index].week;
        assert!(w.fixtures[index].result.is_some());
        for f in &w.fixtures {
            if f.week < week { assert!(f.result.is_some(), "week {} must be played", f.week); }
            if f.week > week { assert!(f.result.is_none(), "week {} must not be played", f.week); }
        }
        assert!(w.calendar_problems().is_empty());
    }

    #[test]
    fn a_club_can_play_its_whole_season_one_match_at_a_time() {
        let mut w = world(8);
        let mut rng = Rng::new(2);
        let mut played = 0;
        while w.play_next_match(1, &mut rng).is_some() {
            played += 1;
        }
        let mine = w.fixtures.iter().filter(|f| f.involves(1)).count();
        assert_eq!(played, mine, "one call per match of the club");
        assert!(w.fixtures.iter().filter(|f| f.involves(1)).all(|f| f.result.is_some()));
    }

    #[test]
    fn a_finished_league_table_adds_up() {
        let mut w = world(8);
        let mut rng = Rng::new(2);
        for week in 1..=14 {
            w.play_week(week, &mut rng);
        }
        let table = w.table(1);
        assert_eq!(table.len(), 8);
        assert!(table.iter().all(|r| r.played == 14 && r.won + r.drawn + r.lost == 14));
        assert_eq!(table.iter().map(|r| r.won).sum::<u8>(), table.iter().map(|r| r.lost).sum::<u8>());
        assert_eq!(table.iter().map(|r| r.goals_for as u32).sum::<u32>(), table.iter().map(|r| r.goals_against as u32).sum::<u32>());
        assert!(table.windows(2).all(|p| p[0].points >= p[1].points), "best first");
    }

    #[test]
    fn cup_rounds_are_drawn_as_the_season_is_played() {
        let mut w = world(8);
        let cup = w.competitions.values().find(|c| c.is_cup()).unwrap().id;
        let mut rng = Rng::new(2);
        while w.play_next_match(1, &mut rng).is_some() {}
        w.play_season_to_end(&mut rng);
        assert_eq!(w.fixtures.iter().filter(|f| f.competition == cup).count(), 7, "8 clubs: 7 cup matches in 3 rounds");
        assert!(w.fixtures.iter().all(|f| f.result.is_some()));
        assert!(w.calendar_problems().is_empty());
    }
}
