//! Honours: the titles and season awards players have won, kept as a list so a player's page can show them. A title is
//! recorded the moment a competition is decided (`World::award_titles`); the season's awards are recorded when the season
//! ends (`World::record_honours`). They are kept, not worked out later, because by then competitions have been rebuilt
//! and players have moved clubs.
use crate::awards::Contender;
use crate::competition::{Competition, CHAMPIONS_LEAGUE_GROUP_ROUNDS};
use crate::model::{ClubId, CompetitionId, Fixture, Line, PlayerId, World};
use std::collections::{BTreeMap, BTreeSet};

const LINES: [Line; 4] = [Line::Goalkeeper, Line::Defence, Line::Midfield, Line::Attack];

/// What the honour is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HonourKind {
    /// His club won the competition (a first division, a cup or the Champions League).
    Title,
    /// Best player of the season over every competition.
    BallonDor,
    PlayerOfTheSeason,
    YoungPlayer,
    /// Best goalkeeper, defender, midfielder or attacker of a competition.
    BestOfLine(Line),
    BestEleven,
    TopScorer,
    TopAssists,
}

/// One honour: a player, a season and a competition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Honour {
    /// The year the season started in.
    pub season: u16,
    /// `None` for the Ballon d'Or, which covers every competition.
    pub competition: Option<CompetitionId>,
    /// The competition's name as it was then ("Premier League (England)"): competition ids change between seasons.
    pub competition_name: String,
    pub kind: HonourKind,
    pub player: PlayerId,
}

impl World {
    /// Every honour the player has won, oldest first.
    pub fn honours_of(&self, player: PlayerId) -> Vec<&Honour> {
        self.honours.iter().filter(|h| h.player == player).collect()
    }

    /// The titles of one season, as the number each (player, competition) has: what the awards add to a score.
    pub(crate) fn titles_won_in(&self, season: u16) -> BTreeMap<(PlayerId, CompetitionId), u16> {
        let mut titles = BTreeMap::new();
        for h in self.honours.iter().filter(|h| h.season == season && h.kind == HonourKind::Title) {
            if let Some(competition) = h.competition {
                *titles.entry((h.player, competition)).or_default() += 1;
            }
        }
        titles
    }

    /// Gives a title to the players of the club that has won a first division, a cup or the Champions League of this
    /// season, once, as soon as the competition is decided. Only players with stats in the competition get it. Call it
    /// after results (`advance_competitions` does).
    pub fn award_titles(&mut self) {
        let year = self.year;
        let decided: Vec<(CompetitionId, ClubId)> =
            self.competitions.values().filter(|c| !self.titles_awarded.contains(&(year, c.id))).filter_map(|c| Some((c.id, self.champion(c)?))).collect();
        for (competition, club) in decided {
            self.titles_awarded.insert((year, competition));
            let name = self.competition_name(competition);
            let winners: Vec<PlayerId> =
                self.players.values().filter(|p| p.club == Some(club) && self.stats.contains_key(&(p.id, year, competition))).map(|p| p.id).collect();
            self.honours.extend(winners.into_iter().map(|player| Honour { season: year, competition: Some(competition), competition_name: name.clone(), kind: HonourKind::Title, player }));
        }
    }

    /// "Premier League (England)".
    fn competition_name(&self, competition: CompetitionId) -> String {
        self.competitions.get(&competition).map_or_else(String::new, |c| format!("{} ({})", c.name, c.country))
    }

    /// The winner of a first division (the top of the table once every match is played), of a cup or of the Champions
    /// League (the winner of the final); `None` while it is undecided or for any other competition.
    fn champion(&self, competition: &Competition) -> Option<ClubId> {
        let own: Vec<&Fixture> = self.fixtures.iter().filter(|f| f.competition == competition.id).collect();
        if own.is_empty() || own.iter().any(|f| f.result.is_none()) {
            return None;
        }
        if competition.is_league() && competition.level.is_none_or(|level| level == 1) {
            return self.table(competition.id).first().map(|row| row.club);
        }
        if competition.is_champions_league() || competition.is_cup() {
            let last = own.iter().map(|f| f.round).max()?;
            let mut final_matches = own.iter().filter(|f| f.round == last);
            let (final_match, only_one) = (final_matches.next()?, final_matches.next().is_none());
            let after_groups = !competition.is_champions_league() || last > CHAMPIONS_LEAGUE_GROUP_ROUNDS;
            return (after_groups && only_one).then(|| final_match.winner()).flatten();
        }
        None
    }

    /// Records the season's awards as honours: the Ballon d'Or; in every league and the Champions League the player of
    /// the season, the best young player, the best of each line and the best eleven; in every competition the top scorer
    /// and the assist leader. Call it once when the season is over, before the next one is built. Does nothing a second time.
    pub fn record_honours(&mut self) {
        let season = self.year;
        if self.honours.iter().any(|h| h.season == season && h.kind != HonourKind::Title) {
            return;
        }
        let played: BTreeSet<CompetitionId> = self.stats.keys().filter(|key| key.1 == season).map(|key| key.2).collect();
        let mut new = vec![];
        for &id in &played {
            let Some(competition) = self.competitions.get(&id) else { continue };
            let awards = self.awards(season, &[id]);
            let name = self.competition_name(id);
            let mut give = |kind, winner: Option<&Contender>| {
                new.extend(winner.map(|c| Honour { season, competition: Some(id), competition_name: name.clone(), kind, player: c.player }));
            };
            give(HonourKind::TopScorer, awards.top_scorers().first());
            give(HonourKind::TopAssists, awards.top_assists().first());
            if competition.is_league() || competition.is_champions_league() {
                give(HonourKind::PlayerOfTheSeason, awards.best_player());
                give(HonourKind::YoungPlayer, awards.best_young_player());
                for line in LINES {
                    give(HonourKind::BestOfLine(line), awards.best_of_line(line));
                }
                for member in awards.best_eleven() {
                    give(HonourKind::BestEleven, Some(member));
                }
            }
        }
        let everything: Vec<CompetitionId> = played.into_iter().collect();
        if let Some(winner) = self.awards(season, &everything).best_player() {
            new.push(Honour { season, competition: None, competition_name: "All competitions".into(), kind: HonourKind::BallonDor, player: winner.player });
        }
        self.honours.extend(new);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::competition::Rules;
    use crate::model::{Day, Position};
    use crate::outcome::Outcome;
    use crate::stats::PlayerStats;
    use crate::testing::player;

    /// A world with a first division of clubs 1 and 2 (competition 1, every match played, club 1 on top) and one striker
    /// in each club (ids 1 and 2) with stats there: player 1 has more goals, player 2 more assists.
    fn finished_league() -> World {
        let mut w = World::new(0, 2014);
        for id in 1..=2 {
            let mut p = player(id, 80, &[Position::St]);
            p.club = Some(id);
            w.players.insert(id, p);
        }
        let stats = |goals, assists| PlayerStats { minutes: 900, appearances: 10, goals, assists, ..PlayerStats::default() };
        w.stats.insert((1, 2014, 1), stats(5, 1));
        w.stats.insert((2, 2014, 1), stats(2, 3));
        let league = Competition { id: 1, name: "L".into(), country: "X".into(), rules: Rules::double_round_robin(), level: Some(1), clubs: vec![1, 2] };
        w.competitions.insert(1, league);
        w.fixtures = vec![played(1, 1, 2, (2, 0), false), played(1, 2, 1, (1, 1), false)];
        w
    }

    fn played(competition: CompetitionId, home: ClubId, away: ClubId, goals: (u8, u8), must_have_winner: bool) -> Fixture {
        Fixture { competition, round: 1, week: 1, day: Day::Saturday, home, away, result: Some(Outcome::full_time(goals.0, goals.1)), must_have_winner }
    }

    fn kinds(w: &World, player: PlayerId) -> Vec<HonourKind> {
        w.honours_of(player).iter().map(|h| h.kind).collect()
    }

    #[test]
    fn the_players_of_the_club_that_wins_a_first_division_get_the_title_once() {
        let mut w = finished_league();
        w.award_titles();
        w.award_titles();
        assert_eq!((kinds(&w, 1), kinds(&w, 2)), (vec![HonourKind::Title], vec![]), "once, and only for the winners");
        assert_eq!(w.honours[0].competition_name, "L (X)");
    }

    #[test]
    fn a_league_still_being_played_or_below_the_first_division_gives_no_title() {
        let mut w = finished_league();
        w.fixtures[1].result = None;
        w.award_titles();
        assert!(w.honours.is_empty(), "not decided yet");
        w.fixtures[1].result = Some(Outcome::full_time(1, 1));
        w.competitions.get_mut(&1).unwrap().level = Some(2);
        w.award_titles();
        assert!(w.honours.is_empty(), "a second division");
    }

    #[test]
    fn the_winner_of_a_cup_final_gets_a_cup_title() {
        let mut w = finished_league();
        let cup = Competition { id: 5, name: "Cup".into(), country: "X".into(), rules: Rules::default_cup(), level: None, clubs: vec![1, 2] };
        w.competitions.insert(5, cup);
        for id in 1..=2 {
            w.stats.insert((id, 2014, 5), PlayerStats { minutes: 90, appearances: 1, ..PlayerStats::default() });
        }
        let mut final_match = played(5, 2, 1, (1, 0), true);
        final_match.result = None;
        w.fixtures.push(final_match);
        w.titles_awarded.insert((2014, 1)); // leave the league out of this
        w.award_titles();
        assert!(w.honours.is_empty(), "the final is not played yet");
        w.fixtures.last_mut().unwrap().result = Some(Outcome::full_time(1, 0));
        w.award_titles();
        assert_eq!((kinds(&w, 2), kinds(&w, 1)), (vec![HonourKind::Title], vec![]), "club 2 won the cup final at home");
    }

    #[test]
    fn a_title_raises_the_score_and_the_champions_league_is_worth_more_than_the_league() {
        let mut w = finished_league();
        let cl = Competition { id: 9, name: "CL".into(), country: "Europe".into(), rules: Rules::champions_league(), level: None, clubs: vec![] };
        w.competitions.insert(9, cl);
        let base = PlayerStats { minutes: 900, appearances: 10, ..PlayerStats::default() };
        w.stats.insert((1, 2014, 1), base);
        w.stats.insert((2, 2014, 9), base);
        let title = |competition, player| Honour { season: 2014, competition: Some(competition), competition_name: String::new(), kind: HonourKind::Title, player };
        w.honours = vec![title(1, 1), title(9, 2)]; // player 1 won the league, player 2 the Champions League
        let awards = w.awards(2014, &[1, 9]);
        let score_of = |id| awards.ranking().iter().find(|c| c.player == id).unwrap().score;
        assert!(score_of(1) > 0.0, "the league title counts");
        assert!(score_of(2) > score_of(1), "the Champions League counts for most");
    }

    #[test]
    fn a_leagues_awards_are_recorded_once_with_the_ballon_dor_and_the_scorer_and_assist_leader() {
        let mut w = finished_league();
        w.record_honours();
        w.record_honours();
        let count = |kind| w.honours.iter().filter(|h| h.kind == kind).count();
        assert_eq!((count(HonourKind::BallonDor), count(HonourKind::TopScorer), count(HonourKind::BestEleven)), (1, 1, 2), "recorded once; both strikers are in the eleven");
        assert!(kinds(&w, 1).contains(&HonourKind::TopScorer) && kinds(&w, 1).contains(&HonourKind::BallonDor) && kinds(&w, 1).contains(&HonourKind::PlayerOfTheSeason));
        assert!(kinds(&w, 2).contains(&HonourKind::TopAssists) && !kinds(&w, 2).contains(&HonourKind::BallonDor));
        assert!(w.honours.iter().any(|h| h.kind == HonourKind::BallonDor && h.competition.is_none()));
    }

    #[test]
    fn a_cup_gives_only_a_top_scorer_and_an_assist_leader() {
        let mut w = finished_league();
        let cup = Competition { id: 5, name: "Cup".into(), country: "X".into(), rules: Rules::default_cup(), level: None, clubs: vec![1, 2] };
        w.competitions.insert(5, cup);
        w.stats.clear();
        w.stats.insert((1, 2014, 5), PlayerStats { minutes: 90, appearances: 1, goals: 2, assists: 1, ..PlayerStats::default() });
        w.record_honours();
        let cup_honours: Vec<HonourKind> = w.honours.iter().filter(|h| h.competition == Some(5)).map(|h| h.kind).collect();
        assert_eq!(cup_honours, [HonourKind::TopScorer, HonourKind::TopAssists]);
    }
}
