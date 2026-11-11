//! Season awards: who was the best player, the best young player, the best of each line, the best eleven and the top
//! scorers, for all competitions together (the Ballon d'Or) or for one league, cup or the Champions League. Worked out
//! from the stats of a season when asked, so they exist for the current season and every past one.
//!
//! A player's score adds up what he did, line by line: how far his match ratings were above an ordinary performance,
//! goals (worth more to a defender than a striker, who is expected to score), assists, clean sheets and player of the
//! match; Champions League matches count more. The numbers are our guesses; the tests state the rules.
use crate::model::{CompetitionId, Line, PlayerId, World};
use crate::stats::PlayerStats;
use std::collections::BTreeMap;

/// The rating of an ordinary performance; each rated match adds its rating above this to the score.
const ORDINARY_RATING: f64 = 6.5;
const ASSIST_VALUE: f64 = 1.0;
const PLAYER_OF_THE_MATCH_VALUE: f64 = 1.0;
/// How much more a Champions League match counts than a domestic one.
const CHAMPIONS_LEAGUE_WEIGHT: f64 = 1.5;
/// What winning a competition adds to the score of each player of the winning club: the Champions League is worth
/// the most, a first division title less, a domestic cup (or league cup) least. Not scaled by how much he played; the
/// minimum minutes keep out those who hardly did.
const CHAMPIONS_LEAGUE_TITLE_VALUE: f64 = 15.0;
const LEAGUE_TITLE_VALUE: f64 = 10.0;
const CUP_TITLE_VALUE: f64 = 5.0;
/// The oldest age, in years at the start of the season, that counts as young.
const YOUNG_AGE: u8 = 21;
/// A player must have played this share of the minutes of whoever played most to be ranked, so a few brilliant minutes
/// do not win the award.
const MIN_SHARE_OF_MOST_MINUTES: f64 = 0.25;
/// The best eleven: how many of each line, a 4-3-3.
const BEST_ELEVEN: [(Line, usize); 4] = [(Line::Goalkeeper, 1), (Line::Defence, 4), (Line::Midfield, 3), (Line::Attack, 3)];

/// What one goal is worth to a player of this line: more for those not expected to score.
fn goal_value(line: Line) -> f64 {
    match line {
        Line::Goalkeeper => 0.0,
        Line::Defence => 2.5,
        Line::Midfield => 1.5,
        Line::Attack => 1.0,
    }
}

/// What one clean sheet is worth to a player of this line: the back of the team keeps them.
fn clean_sheet_value(line: Line) -> f64 {
    match line {
        Line::Goalkeeper => 2.0,
        Line::Defence => 1.5,
        Line::Midfield | Line::Attack => 0.0,
    }
}

/// How well a player of `line` did in a competition, from his stats there. Not weighted by competition.
fn score(line: Line, s: &PlayerStats) -> f64 {
    let rating_surplus = s.rating_tenths as f64 / 10.0 - ORDINARY_RATING * s.rated as f64;
    rating_surplus
        + goal_value(line) * s.goals as f64
        + ASSIST_VALUE * s.assists as f64
        + clean_sheet_value(line) * s.clean_sheets as f64
        + PLAYER_OF_THE_MATCH_VALUE * s.player_of_the_match as f64
}

/// One player's season in the competitions an award covers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contender {
    pub player: PlayerId,
    /// The part of the pitch his main position is on.
    pub line: Line,
    /// Years at the start of the season.
    pub age: u8,
    pub minutes: u16,
    pub goals: u16,
    pub assists: u16,
    /// His season's score (higher is better); see the module doc.
    pub score: f64,
}

/// The awards of one set of competitions in one season.
pub struct Awards {
    /// Everyone who played enough, best first.
    ranked: Vec<Contender>,
    /// Everyone who scored, most goals first; a tie goes to fewer minutes.
    scorers: Vec<Contender>,
    /// Everyone with an assist, most first; a tie goes to fewer minutes.
    assisters: Vec<Contender>,
}

impl Awards {
    /// Builds the awards from every player's season. Pure: the same contenders always give the same awards.
    pub fn new(contenders: Vec<Contender>) -> Awards {
        let most_minutes = contenders.iter().map(|c| c.minutes).max().unwrap_or(0) as f64;
        let enough = |c: &&Contender| c.minutes as f64 >= most_minutes * MIN_SHARE_OF_MOST_MINUTES;
        let mut ranked: Vec<Contender> = contenders.iter().filter(enough).copied().collect();
        ranked.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.player.cmp(&b.player)));
        let contenders_with_assists: Vec<Contender> = contenders.iter().filter(|c| c.assists > 0).copied().collect();
        let mut scorers: Vec<Contender> = contenders.into_iter().filter(|c| c.goals > 0).collect();
        scorers.sort_by_key(|c| (std::cmp::Reverse(c.goals), c.minutes, c.player));
        let mut assisters: Vec<Contender> = contenders_with_assists;
        assisters.sort_by_key(|c| (std::cmp::Reverse(c.assists), c.minutes, c.player));
        Awards { ranked, scorers, assisters }
    }

    /// Everyone who played enough, best first: the Ballon d'Or ranking when the awards cover every competition.
    pub fn ranking(&self) -> &[Contender] {
        &self.ranked
    }

    /// The best player.
    pub fn best_player(&self) -> Option<&Contender> {
        self.ranked.first()
    }

    /// The best player who was `YOUNG_AGE` or younger.
    pub fn best_young_player(&self) -> Option<&Contender> {
        self.ranked.iter().find(|c| c.age <= YOUNG_AGE)
    }

    /// The best goalkeeper, defender, midfielder or attacker.
    pub fn best_of_line(&self, line: Line) -> Option<&Contender> {
        self.ranked.iter().find(|c| c.line == line)
    }

    /// The best eleven: a goalkeeper, four defenders, three midfielders and three attackers, goalkeeper first. Short if
    /// a line does not have enough players.
    pub fn best_eleven(&self) -> Vec<&Contender> {
        BEST_ELEVEN.iter().flat_map(|&(line, count)| self.ranked.iter().filter(move |c| c.line == line).take(count)).collect()
    }

    /// The players with goals, most first.
    pub fn top_scorers(&self) -> &[Contender] {
        &self.scorers
    }

    /// The players with assists, most first.
    pub fn top_assists(&self) -> &[Contender] {
        &self.assisters
    }
}

impl World {
    /// The awards for `season` (the year it starts in) over `competitions`; pass every competition for the Ballon d'Or.
    pub fn awards(&self, season: u16, competitions: &[CompetitionId]) -> Awards {
        let mut totals: BTreeMap<PlayerId, Contender> = BTreeMap::new();
        let titles = self.titles_won_in(season);
        for (&(player, year, competition), stats) in &self.stats {
            let (Some(p), true) = (self.players.get(&player), year == season && competitions.contains(&competition)) else { continue };
            let line = p.positions.first().map_or(Line::Midfield, |pos| pos.line());
            let kind = self.competitions.get(&competition);
            let (weight, title_value) = match kind {
                Some(c) if c.is_champions_league() => (CHAMPIONS_LEAGUE_WEIGHT, CHAMPIONS_LEAGUE_TITLE_VALUE),
                Some(c) if c.is_cup() => (1.0, CUP_TITLE_VALUE),
                _ => (1.0, LEAGUE_TITLE_VALUE),
            };
            let total = totals.entry(player).or_insert(Contender { player, line, age: p.age(season), minutes: 0, goals: 0, assists: 0, score: 0.0 });
            total.minutes += stats.minutes;
            total.goals += stats.goals;
            total.assists += stats.assists;
            total.score += weight * score(line, stats) + title_value * titles.get(&(player, competition)).copied().unwrap_or(0) as f64;
        }
        Awards::new(totals.into_values().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contender(player: PlayerId, line: Line, age: u8, minutes: u16, goals: u16, score: f64) -> Contender {
        Contender { player, line, age, minutes, goals, assists: goals / 2, score }
    }

    /// A squad of 25: ids 1 to 3 keepers, 4 to 11 defenders, 12 to 19 midfielders, 20 to 25 attackers, the lower id the
    /// better player in each line, all of them 25 years old and playing a full season.
    fn squad() -> Vec<Contender> {
        (1..=25)
            .map(|id| {
                let line = match id {
                    1..=3 => Line::Goalkeeper,
                    4..=11 => Line::Defence,
                    12..=19 => Line::Midfield,
                    _ => Line::Attack,
                };
                contender(id, line, 25, 3000, id as u16 % 7, 100.0 - id as f64)
            })
            .collect()
    }

    #[test]
    fn the_best_eleven_has_one_keeper_four_defenders_three_midfielders_and_three_attackers() {
        let eleven = Awards::new(squad()).best_eleven().iter().map(|c| (c.line, c.player)).collect::<Vec<_>>();
        let count = |line| eleven.iter().filter(|(l, _)| *l == line).count();
        assert_eq!((count(Line::Goalkeeper), count(Line::Defence), count(Line::Midfield), count(Line::Attack)), (1, 4, 3, 3));
        assert_eq!(eleven[0], (Line::Goalkeeper, 1), "the best of each line, goalkeeper first");
        assert!(eleven.contains(&(Line::Defence, 7)) && !eleven.contains(&(Line::Defence, 8)));
    }

    #[test]
    fn nobody_is_in_the_best_eleven_twice_and_a_thin_squad_gives_a_short_eleven() {
        let awards = Awards::new(squad().into_iter().filter(|c| c.line != Line::Attack).collect());
        let ids: Vec<PlayerId> = awards.best_eleven().iter().map(|c| c.player).collect();
        assert_eq!(ids.len(), 8);
        assert!(ids.iter().enumerate().all(|(i, id)| !ids[..i].contains(id)));
    }

    #[test]
    fn the_ranking_is_best_first_and_the_best_player_heads_it() {
        let awards = Awards::new(squad());
        assert!(awards.ranking().windows(2).all(|w| w[0].score >= w[1].score));
        assert_eq!(awards.best_player().map(|c| c.player), Some(1));
    }

    #[test]
    fn the_best_young_player_is_the_best_of_those_young_enough() {
        let mut players = squad();
        players[5].age = YOUNG_AGE;
        players[10].age = YOUNG_AGE;
        players[2].age = YOUNG_AGE + 1;
        let young = Awards::new(players).best_young_player().map(|c| c.player);
        assert_eq!(young, Some(6), "player 3 is a year too old; 6 is the best of the young");
    }

    #[test]
    fn the_best_of_a_line_comes_from_that_line() {
        let awards = Awards::new(squad());
        for (line, best) in [(Line::Goalkeeper, 1), (Line::Defence, 4), (Line::Midfield, 12), (Line::Attack, 20)] {
            assert_eq!(awards.best_of_line(line).map(|c| c.player), Some(best));
        }
    }

    #[test]
    fn a_few_brilliant_minutes_cannot_win() {
        let mut players = squad();
        players.push(contender(99, Line::Attack, 19, 200, 9, 1000.0));
        let awards = Awards::new(players);
        assert_eq!(awards.best_player().map(|c| c.player), Some(1));
        assert!(awards.ranking().iter().all(|c| c.player != 99));
        assert_eq!(awards.top_scorers().first().map(|c| c.player), Some(99), "he can still top the scorers: goals are goals");
    }

    #[test]
    fn the_top_scorer_has_the_most_goals_and_a_tie_goes_to_fewer_minutes() {
        let players = vec![contender(1, Line::Attack, 25, 3000, 20, 0.0), contender(2, Line::Attack, 25, 2500, 20, 0.0), contender(3, Line::Attack, 25, 3000, 25, 0.0), contender(4, Line::Defence, 25, 3000, 0, 0.0)];
        let scorers: Vec<PlayerId> = Awards::new(players).top_scorers().iter().map(|c| c.player).collect();
        assert_eq!(scorers, [3, 2, 1], "nobody on zero goals");
    }

    #[test]
    fn no_contenders_give_no_awards() {
        let awards = Awards::new(vec![]);
        assert!(awards.best_player().is_none() && awards.best_eleven().is_empty() && awards.top_scorers().is_empty());
    }

    #[test]
    fn more_of_anything_that_counts_never_lowers_the_score() {
        let base = PlayerStats { rating_tenths: 700, rated: 10, goals: 3, assists: 2, clean_sheets: 1, player_of_the_match: 1, ..PlayerStats::default() };
        for line in [Line::Goalkeeper, Line::Defence, Line::Midfield, Line::Attack] {
            for better in [PlayerStats { goals: 4, ..base }, PlayerStats { assists: 3, ..base }, PlayerStats { clean_sheets: 2, ..base }, PlayerStats { player_of_the_match: 2, ..base }, PlayerStats { rating_tenths: 710, ..base }] {
                assert!(score(line, &better) >= score(line, &base), "{line:?}");
            }
        }
    }

    #[test]
    fn a_goal_is_worth_more_to_a_defender_than_to_an_attacker_and_a_keeper_scores_nothing_for_goals() {
        let goal = PlayerStats { goals: 1, ..PlayerStats::default() };
        assert!(score(Line::Defence, &goal) > score(Line::Midfield, &goal) && score(Line::Midfield, &goal) > score(Line::Attack, &goal));
        assert_eq!(score(Line::Goalkeeper, &goal), 0.0);
    }

    #[test]
    fn a_performance_below_the_ordinary_rating_lowers_the_score() {
        let poor = PlayerStats { rating_tenths: 600, rated: 10, ..PlayerStats::default() };
        assert!(score(Line::Midfield, &poor) < 0.0);
    }

    #[test]
    fn the_awards_of_a_world_cover_only_the_season_and_competitions_asked_for_and_weigh_the_champions_league_up() {
        use crate::competition::{Competition, Rules};
        use crate::model::Position;
        use crate::testing::player;
        let mut w = World::new(0, 2014);
        for id in 1..=2 {
            w.players.insert(id, player(id, 80, &[Position::St]));
        }
        let rules = Rules::champions_league();
        w.competitions.insert(9, Competition { id: 9, name: "CL".into(), country: "Europe".into(), rules, level: None, clubs: vec![] });
        let season = PlayerStats { minutes: 900, goals: 5, rating_tenths: 700, rated: 10, ..PlayerStats::default() };
        w.stats.insert((1, 2014, 1), season); // player 1: league
        w.stats.insert((2, 2014, 9), season); // player 2: the same in the Champions League
        w.stats.insert((1, 2013, 1), PlayerStats { goals: 40, minutes: 900, ..season }); // last season does not count
        let both = w.awards(2014, &[1, 9]);
        assert_eq!(both.best_player().map(|c| c.player), Some(2), "the same numbers are worth more in the Champions League");
        assert_eq!(both.top_scorers().iter().map(|c| c.goals).collect::<Vec<_>>(), [5, 5], "last season's 40 goals are left out");
        let league = w.awards(2014, &[1]);
        assert_eq!(league.ranking().len(), 1, "player 2 did not play in the league");
    }

    #[test]
    fn the_assist_leader_has_the_most_assists_and_a_tie_goes_to_fewer_minutes() {
        let mut players = vec![contender(1, Line::Midfield, 25, 3000, 0, 0.0), contender(2, Line::Midfield, 25, 2500, 0, 0.0), contender(3, Line::Midfield, 25, 3000, 0, 0.0), contender(4, Line::Attack, 25, 3000, 0, 0.0)];
        for (player, assists) in players.iter_mut().zip([12, 12, 15, 0]) {
            player.assists = assists;
        }
        let leaders: Vec<PlayerId> = Awards::new(players).top_assists().iter().map(|c| c.player).collect();
        assert_eq!(leaders, [3, 2, 1], "nobody on zero assists");
    }
}
