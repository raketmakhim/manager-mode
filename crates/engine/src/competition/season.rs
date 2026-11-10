//! The end of a season: promotion and relegation, then a fresh set of fixtures for the next one.
use crate::competition::standings::TableRow;
use crate::model::{ClubId, CompetitionId, World};
use crate::rng::Rng;

/// A club that changed division at the end of a season.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClubMove {
    pub club: ClubId,
    pub from: CompetitionId,
    pub to: CompetitionId,
}

/// Who goes down and who comes up between two neighbouring divisions: the bottom `places` of the upper table and
/// the top `places` of the lower one. Returns (relegated, promoted). `places` is capped at half of either
/// division, so nobody can move twice.
// shortcut: no play-offs; a league with promotion play-offs (the Championship) just takes its top places.
pub fn promotion_and_relegation(upper: &[TableRow], lower: &[TableRow], places: usize) -> (Vec<ClubId>, Vec<ClubId>) {
    let places = places.min(upper.len() / 2).min(lower.len() / 2);
    let relegated = upper[upper.len() - places..].iter().map(|r| r.club).collect();
    let promoted = lower[..places].iter().map(|r| r.club).collect();
    (relegated, promoted)
}

impl World {
    /// True once every fixture has a result (and at least one exists).
    pub fn season_finished(&self) -> bool {
        !self.fixtures.is_empty() && self.fixtures.iter().all(|f| f.result.is_some())
    }

    /// Moves clubs between divisions: in each country, neighbouring levels (1 and 2, 2 and 3...) swap their
    /// bottom and top clubs. Returns every move.
    fn promote_and_relegate(&mut self) -> Vec<ClubMove> {
        let mut levels: Vec<(&str, u8, CompetitionId)> = self
            .competitions
            .values()
            .filter(|c| c.is_league() && c.clubs.len() >= 2)
            .filter_map(|c| Some((c.country.as_str(), c.level?, c.id)))
            .collect();
        levels.sort();
        let mut moves = vec![];
        for pair in levels.windows(2) {
            let ((country, level, upper), (other_country, other_level, lower)) = (pair[0], pair[1]);
            if country != other_country || other_level != level + 1 {
                continue; // no division directly below
            }
            let places = self.competitions[&upper].rules.relegated as usize;
            let (down, up) = promotion_and_relegation(&self.table(upper), &self.table(lower), places);
            moves.extend(down.into_iter().map(|club| ClubMove { club, from: upper, to: lower }));
            moves.extend(up.into_iter().map(|club| ClubMove { club, from: lower, to: upper }));
        }
        for m in &moves {
            self.competitions.get_mut(&m.from).unwrap().clubs.retain(|&c| c != m.club);
            self.competitions.get_mut(&m.to).unwrap().clubs.push(m.club);
            self.clubs.get_mut(&m.club).unwrap().league = m.to;
        }
        moves
    }

    /// Ends a finished season and starts the next: promotion and relegation, the year goes up by one, and every
    /// league and cup gets new fixtures. Returns the moves, or `None` if the season is not finished yet.
    // This moves the competitions on only: the summer's transfers, ratings and ageing are applied by the caller
    // (`gui::Career`), because they need the database and the user's club.
    pub fn start_next_season(&mut self, rng: &mut Rng) -> Option<Vec<ClubMove>> {
        if !self.season_finished() {
            return None;
        }
        let moves = self.promote_and_relegate();
        self.year += 1;
        self.fixtures.clear();
        self.competitions.retain(|_, c| !c.is_cup()); // cups are rebuilt from the new league lists
        self.start_season(rng);
        Some(moves)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::competition::{Competition, Rules};
    use crate::testing::add_club_with_eleven;

    /// Two divisions of `size` clubs in one country: league 1 holds clubs 1..=size, league 2 the next `size`.
    /// A club is rated lower the higher its id, so the better divisions hold the lower ids.
    fn two_divisions(size: u32) -> World {
        let mut w = World::new(0, 2014);
        for (id, first) in [(1, 1), (2, size + 1)] {
            let clubs: Vec<ClubId> = (first..first + size).collect();
            for &c in &clubs {
                add_club_with_eleven(&mut w, c, id, 99 - c as u8);
            }
            let rules = Rules::double_round_robin();
            w.competitions.insert(id, Competition { id, name: format!("D{id}"), country: "X".into(), rules, level: Some(id as u8), clubs });
        }
        w
    }

    fn rows(clubs: &[ClubId]) -> Vec<TableRow> {
        let row = |&club| TableRow { club, played: 0, won: 0, drawn: 0, lost: 0, goals_for: 0, goals_against: 0, points: 0 };
        clubs.iter().map(row).collect()
    }

    #[test]
    fn the_bottom_of_the_upper_table_swaps_with_the_top_of_the_lower() {
        let (down, up) = promotion_and_relegation(&rows(&[1, 2, 3, 4, 5, 6]), &rows(&[7, 8, 9, 10, 11, 12]), 2);
        assert_eq!((down, up), (vec![5, 6], vec![7, 8]));
        // Never more than half a division, even when asked for more.
        let (down, up) = promotion_and_relegation(&rows(&[1, 2, 3, 4]), &rows(&[7, 8, 9, 10]), 9);
        assert_eq!((down.len(), up.len()), (2, 2));
    }

    #[test]
    fn a_season_cannot_end_before_it_is_finished() {
        let mut w = two_divisions(8);
        w.start_season(&mut Rng::new(1));
        assert!(!w.season_finished());
        assert!(w.start_next_season(&mut Rng::new(1)).is_none());
        assert_eq!(w.year, 2014);
    }

    #[test]
    fn the_next_season_has_new_fixtures_new_divisions_and_the_same_number_of_clubs() {
        let mut w = two_divisions(8);
        let mut rng = Rng::new(1);
        w.start_season(&mut rng);
        w.play_season_to_end(&mut rng);
        assert!(w.season_finished());
        let expected_down: Vec<ClubId> = w.table(1).iter().rev().take(3).map(|r| r.club).collect();
        let expected_up: Vec<ClubId> = w.table(2).iter().take(3).map(|r| r.club).collect();

        let moves = w.start_next_season(&mut rng).unwrap();
        assert_eq!(moves.len(), 6);
        assert_eq!(w.year, 2015);
        assert!(expected_down.iter().all(|c| w.clubs[c].league == 2));
        assert!(expected_up.iter().all(|c| w.clubs[c].league == 1));
        assert_eq!((w.competitions[&1].clubs.len(), w.competitions[&2].clubs.len()), (8, 8));
        // The links agree both ways, and the new season starts clean.
        for c in w.clubs.values() {
            assert!(w.competitions[&c.league].clubs.contains(&c.id));
        }
        assert!(w.fixtures.iter().all(|f| f.result.is_none()) && !w.fixtures.is_empty());
        assert!(!w.season_finished());
        assert!(w.calendar_problems().is_empty());
        assert_eq!(w.competitions.values().filter(|c| c.is_cup()).count(), 1, "one cup per country, rebuilt");
    }

    #[test]
    fn many_seasons_in_a_row_keep_the_divisions_whole() {
        let mut w = two_divisions(10);
        let mut rng = Rng::new(9);
        w.start_season(&mut rng);
        for _ in 0..5 {
            w.play_season_to_end(&mut rng);
            w.start_next_season(&mut rng).unwrap();
            assert_eq!((w.competitions[&1].clubs.len(), w.competitions[&2].clubs.len()), (10, 10));
        }
        assert_eq!(w.year, 2019);
    }

    #[test]
    fn real_fifa_15_divisions_keep_their_size_through_a_season_change() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../history.db");
        if !std::path::Path::new(path).exists() { return; } // needs `cargo run -p import` first
        let mut w = crate::source::load_edition(&rusqlite::Connection::open(path).unwrap(), 15).unwrap();
        let mut rng = Rng::new(1);
        w.start_season(&mut rng);
        let sizes = |w: &World| w.competitions.values().filter(|c| c.is_league()).map(|c| (c.id, c.clubs.len())).collect::<Vec<_>>();
        let before = sizes(&w);
        w.play_season_to_end(&mut rng);
        let moves = w.start_next_season(&mut rng).unwrap();
        assert!(!moves.is_empty());
        assert_eq!(sizes(&w), before, "every division keeps its number of clubs");
        assert!(w.calendar_problems().is_empty());
        let premier_league = w.competitions.values().find(|c| c.name == "Premier League" && c.country == "England").unwrap();
        assert_eq!(moves.iter().filter(|m| m.to == premier_league.id).count(), 3);
    }
}
