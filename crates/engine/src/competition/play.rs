//! Playing fixtures: one match, one week, or everything up to a club's next match.
use crate::attrs::Attr;
use super::Competition;
use crate::fitness::FitnessSettings;
use crate::lineup::{rating_at, Selection};
use crate::model::{ClubId, Player, Position, World};
use crate::rng::Rng;
use crate::sim::{simulate, LiveMatch, MatchInput, MatchModel, MatchPlayer, MatchReport, MatchTeam, AVERAGE_AGGRESSION, DEFAULT_SHOT_BASED_PARTS};

impl World {
    /// Plays the fixture at `index` in `fixtures` with its competition's match model and stores the result. Cup ties
    /// always get a winner.
    // shortcut: recomputes both lineups on every match (a whole-world season of about 10,000 matches takes 0.5 s in a
    // release build, 10 s in debug); cache per club, cleared when ratings or squads change, when the simtool runs
    // thousands of seasons. The events of a match only feed the player stats; they are not kept. Keep them for the
    // matches that matter (the user's club, finals) once something shows them.
    pub fn play_fixture(&mut self, index: usize, rng: &mut Rng) {
        self.rest_for(index);
        let f = &self.fixtures[index];
        let competition = &self.competitions[&f.competition];
        let input = self.match_input(f.home, f.away, competition, None);
        let report = simulate(competition.rules.model, &input, rng);
        self.store_report(index, report);
    }

    /// Stores a played match's result and adds its events to the player stats.
    fn store_report(&mut self, index: usize, report: MatchReport) {
        if let Some(detail) = &report.detail {
            let f = &self.fixtures[index];
            self.record_match(f.competition, f.day_number(), detail, &report.outcome);
        }
        self.fixtures[index].result = Some(report.outcome);
    }

    /// Plays every unplayed match of `week`, then draws any cup round that this completes.
    pub fn play_week(&mut self, week: u8, rng: &mut Rng) {
        // In order of day, so a player's rest between two matches of the week counts the right days.
        let mut todo: Vec<usize> = (0..self.fixtures.len()).filter(|&i| self.fixtures[i].week == week && self.fixtures[i].result.is_none()).collect();
        todo.sort_by_key(|&i| self.fixtures[i].day);
        for index in todo {
            self.play_fixture(index, rng);
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
        let next = self.play_up_to_next_match(club, rng)?;
        self.play_fixture(next, rng);
        self.advance_cups(rng);
        Some(next)
    }

    /// Plays everything before the club's next unplayed match, the other clubs' matches of its week included, but not
    /// the match itself, which the caller plays with `play_fixture` or live (`start_live_match`). Returns its index,
    /// or `None` if the club has nothing left to play. Cup rounds are drawn when the caller calls `advance_cups`.
    pub fn play_up_to_next_match(&mut self, club: ClubId, rng: &mut Rng) -> Option<usize> {
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
            if f.week == week && f.result.is_none() && !f.involves(club) {
                self.play_fixture(i, rng);
            }
        }
        self.rest_for(next); // so the lineup shown and the match watched start from today's fitness
        Some(next)
    }

    /// The fixture at `index` ready to be played a minute at a time, with `club`'s changes made by the user. `None`
    /// if the competition's match model cannot be played that way.
    pub fn start_live_match(&self, index: usize, club: ClubId, rng: &mut Rng) -> Option<LiveMatch> {
        let f = &self.fixtures[index];
        let competition = &self.competitions[&f.competition];
        (competition.rules.model == MatchModel::ShotBased).then(|| LiveMatch::new(&self.match_input(f.home, f.away, competition, Some(club)), DEFAULT_SHOT_BASED_PARTS, [f.home == club, f.away == club], rng))
    }

    /// Plays out whatever is left of a live match, stores its result and draws any cup round that this completes.
    pub fn finish_live_match(&mut self, index: usize, live: LiveMatch, rng: &mut Rng) {
        let report = live.into_report(rng);
        self.store_report(index, report);
        self.advance_cups(rng);
    }

    /// What a match model needs to know about the two clubs: their lineups, each player rated in his slot.
    ///
    /// `user_club` is the club whose lineup the user may rearrange during the match, if any: its starters carry their rating
    /// in every position (about 20% of a season's simulation time if everyone's did, and only the user changes positions).
    pub fn match_input(&self, home: ClubId, away: ClubId, competition: &Competition, user_club: Option<ClubId>) -> MatchInput {
        MatchInput { home: self.match_team(home, user_club == Some(home)), away: self.match_team(away, user_club == Some(away)), must_have_winner: competition.is_cup(), max_substitutions: competition.rules.substitutions }
    }

    /// The club's lineup and bench as the match models see them. An empty place is a player with id 0 and rating 0.
    fn match_team(&self, club: ClubId, every_position: bool) -> MatchTeam {
        let slots = self.clubs[&club].formation.slots();
        let Selection { lineup, bench } = self.selection_of(club);
        let players = std::array::from_fn(|i| match lineup[i] {
            Some(id) => match_player(&self.players[&id], slots[i], every_position, &self.fitness_settings),
            None => MatchPlayer::empty(slots[i]),
        });
        // A substitute is rated in his own main position and in every other, for the slot of whoever he replaces.
        let bench = bench.map(|place| match place {
            Some(id) => match_player(&self.players[&id], self.players[&id].positions[0], true, &self.fitness_settings),
            None => MatchPlayer::empty(Position::Cm),
        });
        MatchTeam { players, bench }
    }
}

/// One lineup player for a match model. An attribute the data lacks (FIFA 15) counts as his rating in the slot. With
/// `every_position` false only the rating in his slot is worked out and `ratings` repeats it.
fn match_player(player: &Player, slot: Position, every_position: bool, fitness: &FitnessSettings) -> MatchPlayer {
    let rating = rating_at(player, slot);
    let attr = |a: Attr| player.attrs[a].unwrap_or(rating);
    MatchPlayer {
        id: player.id,
        position: slot,
        rating,
        ratings: if every_position { std::array::from_fn(|i| rating_at(player, Position::ALL[i])) } else { [rating; Position::COUNT] },
        finishing: attr(Attr::Finishing),
        creativity: creativity(attr(Attr::Vision), attr(Attr::ShortPassing), attr(Attr::LongPassing), attr(Attr::Crossing), attr(Attr::Dribbling)),
        aggression: player.attrs[Attr::Aggression].unwrap_or(AVERAGE_AGGRESSION),
        stamina: attr(Attr::Stamina),
        start_tiredness: fitness.start_tiredness(player.fitness),
    }
}

/// How well a player sets up goals (0-99): vision 50%, short passing 20%, long passing 11%, crossing 11%, dribbling 8%
/// (our design: vision counts most, then short passing, then the longer deliveries, then dribbling; no real data to fit).
fn creativity(vision: u8, short_passing: u8, long_passing: u8, crossing: u8, dribbling: u8) -> u8 {
    let weighted = 50 * vision as u16 + 20 * short_passing as u16 + 11 * long_passing as u16 + 11 * crossing as u16 + 8 * dribbling as u16;
    (weighted / 100) as u8
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
    fn playing_a_match_tires_the_outfield_players_of_both_clubs_and_not_the_others() {
        let mut w = world(4);
        let index = w.play_next_match(1, &mut Rng::new(2)).unwrap();
        let (home, away) = (w.fixtures[index].home, w.fixtures[index].away);
        let outfield_fitness = |club: ClubId| w.clubs[&club].squad.iter().map(|id| &w.players[id]).filter(|p| p.positions[0] != Position::Gk).map(|p| p.fitness).collect::<Vec<_>>();
        for club in [home, away] {
            assert!(outfield_fitness(club).iter().all(|&f| f < 100), "club {club} played");
        }
        let keeper = w.clubs[&home].squad.iter().map(|id| &w.players[id]).find(|p| p.positions[0] == Position::Gk).unwrap();
        assert_eq!(keeper.fitness, 100, "a goalkeeper does not tire");
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

    #[test]
    fn the_players_goals_add_up_to_the_table_and_the_lineups_and_minutes_add_up() {
        let mut w = world(8);
        w.play_season_to_end(&mut Rng::new(2));
        let stats = w.stats_for(Some(w.year), Some(1));
        let table = w.table(1);
        assert_eq!(stats.iter().map(|(_, s)| s.goals as u32).sum::<u32>(), table.iter().map(|r| r.goals_for as u32).sum::<u32>());
        // Each goal against is conceded by the eleven on the pitch at the time (fewer after a red card), whoever they are.
        assert!(stats.iter().map(|(_, s)| s.conceded as u32).sum::<u32>() <= 11 * table.iter().map(|r| r.goals_against as u32).sum::<u32>());
        // Eleven start each of the 14 matches of 8 clubs and a substitute adds an appearance; nobody plays a match twice.
        assert!(stats.iter().map(|(_, s)| s.appearances as u32).sum::<u32>() >= 11 * 14 * 8);
        assert!(stats.iter().all(|(_, s)| s.appearances <= 14 && s.minutes <= 14 * 90));
        // The minutes of a club's players cannot exceed eleven players for the whole of every match.
        assert!(stats.iter().map(|(_, s)| s.minutes as u32).sum::<u32>() <= 11 * 90 * 14 * 8);
    }

    #[test]
    fn a_live_match_can_be_changed_by_the_user_and_is_stored_with_its_stats() {
        use crate::outcome::Side;
        use crate::testing::player;
        let mut w = world(8);
        // Three extra players in club 1: a back-up keeper, a midfielder and a striker for the bench.
        for (i, position) in [Position::Gk, Position::Cm, Position::St].into_iter().enumerate() {
            let mut p = player(111 + i as u32, 60, &[position]);
            p.club = Some(1);
            w.clubs.get_mut(&1).unwrap().squad.push(p.id);
            w.players.insert(p.id, p);
        }
        let mut rng = Rng::new(3);
        let index = w.play_up_to_next_match(1, &mut rng).unwrap();
        assert!(w.fixtures[index].result.is_none(), "the match itself is left for the caller");
        let mut live = w.start_live_match(index, 1, &mut rng).unwrap();
        for _ in 0..60 {
            live.advance(&mut rng);
        }
        let side = if w.fixtures[index].home == 1 { Side::Home } else { Side::Away };
        let off = live.team(side).players[10].id;
        let on = live.team(side).bench.iter().find(|p| p.position == Position::St).expect("the striker is on the bench").id;
        live.substitute(side, off, on).unwrap();
        w.finish_live_match(index, live, &mut rng);
        assert!(w.fixtures[index].result.is_some());
        let stats = w.stats_for(Some(w.year), Some(1));
        let of = |id| stats.iter().find(|(p, _)| *p == id).map(|(_, s)| *s).unwrap();
        assert_eq!((of(on).appearances, of(on).minutes), (1, 30), "the substitute played from the 60th minute");
        assert_eq!(of(off).minutes, 60);
    }

    #[test]
    fn a_match_of_the_cheap_model_cannot_be_played_live() {
        let mut w = world(8);
        w.competitions.get_mut(&1).unwrap().rules.model = MatchModel::Cheap;
        let mut rng = Rng::new(1);
        let index = w.play_up_to_next_match(1, &mut rng).unwrap();
        assert!(w.start_live_match(index, 1, &mut rng).is_none());
    }

    #[test]
    fn creativity_weights_vision_then_short_passing_then_long_passing_and_crossing_then_dribbling() {
        assert_eq!(creativity(80, 80, 80, 80, 80), 80);
        assert_eq!(creativity(0, 0, 0, 0, 0), 0);
        let only = |v, sp, lp, cr, dr| creativity(v, sp, lp, cr, dr);
        // 100 in one attribute and 0 elsewhere gives that attribute's share.
        assert_eq!([only(100, 0, 0, 0, 0), only(0, 100, 0, 0, 0), only(0, 0, 100, 0, 0), only(0, 0, 0, 100, 0), only(0, 0, 0, 0, 100)], [50, 20, 11, 11, 8]);
    }
}
