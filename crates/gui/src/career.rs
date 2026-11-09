//! Career tab: start a new career (pick a year, a league and a club), then see that club's fixtures and dates, play its
//! matches at once or watch them a minute at a time (`live`).
use crate::leagues::{league_label, sorted_leagues};
use crate::squad::{self, History};
use crate::lineup;
use crate::{competitions, fixtures, live, stats, transfers, RED};
use eframe::egui;
use engine::{advance_ratings, CareerStatus, edition_label, editions, load_edition, plan_transfers, season_label, season_name, ClubId, CompetitionId, Connection, Rng, Side, Window, World};

/// How many levelled-up players the line next to the buttons names.
const MAX_LEVELLED_NAMED: usize = 3;

pub struct Career {
    /// Editions in the database, oldest first.
    editions: Vec<i64>,
    /// The year picked in the drop-down; `world` is loaded for it.
    edition: i64,
    /// The loaded edition, so the league and club lists can be filled. Gets its fixtures when the career starts.
    world: Option<World>,
    league: Option<CompetitionId>,
    club: Option<ClubId>,
    /// False: the new-career form. True: the club's season.
    started: bool,
    view: View,
    /// The competition shown in the Competitions view; `None` means your own league.
    competition: Option<CompetitionId>,
    /// Your squad's overall at the start of each season so far, and its ratings at the start of this one, for the squad view.
    history: Option<History>,
    squad: squad::State,
    lineup: lineup::State,
    stats: stats::State,
    /// Your match while it is being watched; the rest of the screen waits.
    live: Option<live::State>,
    /// The summer's transfers while you decide on the ones that involve your club; the season waits.
    window: Option<Window>,
    /// Seeded when the career starts; every simulated match draws from it.
    rng: Rng,
    /// The result of the last match you played, for the line next to the button.
    last_played: Option<String>,
    error: Option<String>,
}

impl Career {
    pub fn new(conn: &Connection) -> Self {
        let editions = editions(conn).unwrap_or_default();
        Career { edition: editions.first().copied().unwrap_or(15), editions, world: None, league: None, club: None, started: false, view: View::Fixtures, competition: None, history: None, squad: squad::State::default(), lineup: lineup::State::default(), stats: stats::State::default(), live: None, window: None, rng: Rng::new(0), last_played: None, error: None }
    }

    /// Loads the picked edition and selects the first league and club.
    fn load(&mut self, conn: &Connection) {
        match load_edition(conn, self.edition) {
            Ok(world) => {
                self.league = sorted_leagues(&world).first().map(|l| l.id);
                self.world = Some(world);
                self.pick_first_club();
                self.error = None;
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    /// Selects the club that comes first A to Z in the picked league.
    fn pick_first_club(&mut self) {
        let Some(world) = &self.world else { return };
        let league = self.league.and_then(|id| world.competitions.get(&id));
        self.club = league.and_then(|l| l.clubs.iter().min_by_key(|id| &world.clubs[id].name).copied());
    }

    /// Draws the season's fixtures; the seed makes the cup draw the same for the same edition.
    fn start(&mut self) {
        let (Some(world), Some(club)) = (&mut self.world, self.club) else { return };
        self.rng = Rng::new(self.edition as u64);
        self.last_played = None;
        self.competition = None;
        self.view = View::Fixtures;
        world.start_season(&mut self.rng);
        world.plan_retirements(&mut self.rng);
        self.history = Some(History::new(world, club, season_name(world.year)));
        self.squad = squad::State::default();
        self.lineup = lineup::State::default();
        self.live = None;
        self.window = None;
        self.started = true;
    }

    /// Plays the club's next match (and everything before it in the league and cups) and remembers its result.
    fn play_next_match(&mut self, club: ClubId) {
        let Some(world) = &mut self.world else { return };
        self.last_played = Some(match world.play_next_match(club, &mut self.rng) {
            None => "Nothing left to play.".to_string(),
            Some(index) => Self::result_text(world, index),
        });
        self.add_level_ups(club);
    }

    /// Levels up your players whose XP bar is full and adds who to the line next to the buttons.
    fn add_level_ups(&mut self, club: ClubId) {
        let Some(world) = &mut self.world else { return };
        let levelled = world.level_up_club(club, &mut self.rng);
        if levelled.is_empty() {
            return;
        }
        // After a whole season there can be a dozen: name the first few.
        let names: Vec<String> = levelled.iter().take(MAX_LEVELLED_NAMED).map(|d| format!("{} {} -> {}", world.players[&d.player].name, d.overall_before, d.overall_after)).collect();
        let more = levelled.len().saturating_sub(MAX_LEVELLED_NAMED);
        if let Some(text) = &mut self.last_played {
            text.push_str(&format!(" Levelled up: {}{}.", names.join(", "), if more > 0 { format!(" and {more} more") } else { String::new() }));
        }
    }

    /// Plays everything before the club's next match and opens that match to watch a minute at a time. A match
    /// that cannot be watched (a competition on the cheap model) is just played.
    fn watch_next_match(&mut self, club: ClubId) {
        let Some(world) = &mut self.world else { return };
        let Some(index) = world.play_up_to_next_match(club, &mut self.rng) else {
            self.last_played = Some("Nothing left to play.".to_string());
            return;
        };
        match world.start_live_match(index, club, &mut self.rng) {
            Some(live) => {
                let side = if world.fixtures[index].home == club { Side::Home } else { Side::Away };
                self.live = Some(live::State::new(index, live, side, world.clubs[&club].formation));
            }
            None => {
                world.play_fixture(index, &mut self.rng);
                world.advance_cups(&mut self.rng);
                self.last_played = Some(Self::result_text(world, index));
            }
        }
    }

    /// The match you watched is over: store its result and go back to the season.
    fn finish_watching(&mut self) {
        let (Some(world), Some(state)) = (&mut self.world, self.live.take()) else { return };
        let (index, live) = state.into_parts();
        world.finish_live_match(index, live, &mut self.rng);
        self.last_played = Some(Self::result_text(world, index));
        if let Some(club) = self.club {
            self.add_level_ups(club);
        }
    }

    /// "Home 2-1 Away" for a played fixture.
    fn result_text(world: &World, index: usize) -> String {
        let f = &world.fixtures[index];
        let result = f.result.map_or(String::new(), |o| o.to_string());
        format!("{} {} {}", world.clubs[&f.home].name, result, world.clubs[&f.away].name)
    }

    /// Simulates every match that is left in the season, for all clubs.
    fn finish_season(&mut self) {
        let Some(world) = &mut self.world else { return };
        world.play_season_to_end(&mut self.rng);
        self.last_played = Some("Season finished.".to_string());
        if let Some(club) = self.club {
            self.add_level_ups(club);
        }
    }

    /// The end of the season: plans the summer's transfers and, if some involve your club, waits for your answers
    /// (`transfers::ui`) before starting the next season.
    fn begin_next_season(&mut self, club: ClubId, conn: &Connection) {
        let Some(world) = &self.world else { return };
        match plan_transfers(conn, world, Some(club)) {
            Ok(window) if !window.questions_for(club).is_empty() => self.window = Some(window),
            Ok(window) => self.start_next_season(club, window, conn),
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    /// Retires the players who planned to, makes the transfers, then promotion and relegation and the next season's
    /// fixtures, and lets everyone decide on retiring this season. Says what happened to your club. Nothing in the
    /// world changes until the point where nothing can fail any more except reading the new ratings.
    fn start_next_season(&mut self, club: ClubId, window: Window, conn: &Connection) {
        let Some(world) = &mut self.world else { return };
        let Some(moves) = world.start_next_season(&mut self.rng) else { return };
        let leaving: Vec<String> = world.clubs[&club].squad.iter().map(|id| &world.players[id]).filter(|p| p.career == CareerStatus::Retiring).map(|p| p.name.clone()).collect();
        let retired = world.retire_planned();
        let deals = (window.transfers().len(), window.questions_for(club).iter().filter(|&&i| !window.is_declined(i)).count());
        world.make_transfers(window);
        let ratings = match advance_ratings(conn, world) {
            Ok(true) => "Player ratings updated to the real new season.",
            Ok(false) => "No newer data: player ratings are unchanged.",
            Err(_) => "Could not read the new ratings.",
        };
        // After the transfers and new ratings, so the newcomers decide too and the chance uses the new overall.
        let planned = world.plan_retirements(&mut self.rng);
        let planning: Vec<String> = planned.iter().filter(|(c, _)| *c == Some(club)).map(|(_, id)| world.players[id].name.clone()).collect();
        if let Some(history) = &mut self.history {
            history.start_season(world, club, season_name(world.year));
        }
        let mine = moves.iter().find(|m| m.club == club);
        let text = match mine {
            Some(m) => {
                let verb = if world.competitions[&m.to].level < world.competitions[&m.from].level { "promoted to" } else { "relegated to" };
                format!("Your club was {verb} the {}.", world.competitions[&m.to].name)
            }
            _ => "Your club stays in its division.".to_string(),
        };
        self.last_played = Some(format!("{} clubs changed division. {text} {} transfers, {} of them yours. {} players retired{}, {} plan to retire at the end of this season{}. {ratings}", moves.len(), deals.0, deals.1, retired.len(), yours(&leaving), planned.len(), yours(&planning)));
        self.competition = None;
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, conn: &Connection) {
        if self.world.is_none() {
            self.load(conn);
        }
        egui::CentralPanel::default_margins().show(ui, |ui| {
            if let Some(e) = &self.error {
                ui.colored_label(RED, e);
            }
            if self.started {
                self.season_ui(ui, conn);
            } else {
                self.new_career_ui(ui, conn);
            }
        });
    }

    fn new_career_ui(&mut self, ui: &mut egui::Ui, conn: &Connection) {
        ui.heading("New career");
        ui.add_space(6.0);
        let before = self.edition;
        ui.horizontal(|ui| {
            ui.label("Start in:");
            let label = |e: i64| format!("{} ({})", edition_label(e), season_label(e));
            egui::ComboBox::from_id_salt("career_edition").selected_text(label(self.edition)).show_ui(ui, |ui| {
                for &e in &self.editions {
                    ui.selectable_value(&mut self.edition, e, label(e));
                }
            });
        });
        if self.edition != before {
            self.load(conn); // the clubs differ per edition
        }
        let Some(world) = &self.world else { return };

        let leagues = sorted_leagues(world);
        let Some(league) = self.league.and_then(|id| world.competitions.get(&id)) else { return };
        let league_before = self.league;
        ui.horizontal(|ui| {
            ui.label("League:");
            egui::ComboBox::from_id_salt("career_league").selected_text(league_label(league)).show_ui(ui, |ui| {
                for l in &leagues {
                    ui.selectable_value(&mut self.league, Some(l.id), league_label(l));
                }
            });
        });
        if self.league != league_before {
            self.pick_first_club();
            return;
        }

        let mut clubs: Vec<_> = league.clubs.iter().map(|id| &world.clubs[id]).collect();
        clubs.sort_by(|a, b| a.name.cmp(&b.name));
        let selected = self.club.map_or(String::new(), |id| world.clubs[&id].name.clone());
        ui.horizontal(|ui| {
            ui.label("Club:");
            egui::ComboBox::from_id_salt("career_club").selected_text(selected).show_ui(ui, |ui| {
                for c in &clubs {
                    ui.selectable_value(&mut self.club, Some(c.id), &c.name);
                }
            });
        });
        ui.add_space(8.0);
        if ui.add_enabled(self.club.is_some(), egui::Button::new("Start career")).clicked() {
            self.start();
        }
    }

    fn season_ui(&mut self, ui: &mut egui::Ui, conn: &Connection) {
        let (Some(club_id), Some(world)) = (self.club, &self.world) else { return };
        if let Some(state) = &mut self.live {
            if live::ui(ui, world, state, &mut self.rng) {
                self.finish_watching();
            }
            return;
        }
        let (club, league) = (&world.clubs[&club_id], &world.competitions[&world.clubs[&club_id].league]);
        let title = format!("{} · season {}", league_label(league), season_name(world.year));
        ui.horizontal(|ui| {
            ui.heading(&club.name);
            ui.label(title);
        });
        ui.horizontal(|ui| {
            let finished = self.world.as_ref().is_some_and(|w| w.season_finished());
            if ui.add_enabled(!finished, egui::Button::new("Watch next match")).clicked() {
                self.watch_next_match(club_id);
            }
            if ui.add_enabled(!finished, egui::Button::new("Play next match")).clicked() {
                self.play_next_match(club_id);
            }
            if ui.add_enabled(!finished, egui::Button::new("Finish season")).clicked() {
                self.finish_season();
            }
            if ui.add_enabled(finished, egui::Button::new("Start next season")).clicked() {
                self.begin_next_season(club_id, conn);
            }
            if ui.button("New career").clicked() {
                self.started = false;
                self.load(conn); // the season changed the world, so load a clean one
            }
            if let Some(text) = &self.last_played {
                ui.label(text);
            }
        });
        let Some(world) = &self.world else { return };
        ui.separator();
        if let Some(window) = &mut self.window {
            if transfers::ui(ui, world, club_id, window) {
                if let Some(window) = self.window.take() {
                    self.start_next_season(club_id, window, conn);
                }
            }
            return;
        }

        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.view, View::Fixtures, "My fixtures");
            ui.selectable_value(&mut self.view, View::Squad, "My squad");
            ui.selectable_value(&mut self.view, View::Lineup, "Lineup");
            ui.selectable_value(&mut self.view, View::Competitions, "Competitions");
            ui.selectable_value(&mut self.view, View::Stats, "Stats");
        });
        ui.add_space(6.0);
        match self.view {
            View::Fixtures => fixtures::ui(ui, world, club_id),
            View::Squad => {
                if let Some(history) = &self.history {
                    squad::ui(ui, world, club_id, history, &mut self.squad);
                }
            }
            View::Lineup => {
                if let Some(world) = &mut self.world {
                    lineup::ui(ui, world, club_id, &mut self.lineup);
                }
            }
            View::Competitions => competitions::ui(ui, world, club_id, &mut self.competition),
            View::Stats => stats::ui(ui, world, club_id, &mut self.stats),
        }
    }
}

/// " (yours: A, B)" naming your players in a list, or nothing when none of them is.
fn yours(names: &[String]) -> String {
    if names.is_empty() {
        String::new()
    } else {
        format!(" (yours: {})", names.join(", "))
    }
}

/// What the season screen shows.
#[derive(PartialEq)]
enum View {
    Fixtures,
    Competitions,
    Stats,
    Squad,
    Lineup,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_career_shows_the_clubs_league_and_cup_fixtures() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../history.db");
        if !std::path::Path::new(path).exists() { return; } // needs `cargo run -p import` first
        let conn = Connection::open(path).unwrap();
        let mut c = Career::new(&conn);
        c.load(&conn);
        assert!(c.club.is_some(), "the form starts with a club selected");
        c.start();
        let world = c.world.as_ref().unwrap();
        let club = c.club.unwrap();
        let league = world.clubs[&club].league;
        let mine = |id: CompetitionId| world.fixtures.iter().filter(|f| f.competition == id && f.involves(club)).count();
        let n = world.competitions[&league].clubs.len();
        assert_eq!(mine(league), 2 * (n - 1));
        assert!(world.calendar_problems().is_empty());
        let name = world.clubs[&club].name.clone();
        c.play_next_match(club);
        assert!(c.last_played.as_deref().is_some_and(|t| t.contains(&name)));
    }

    #[test]
    fn the_next_season_waits_for_your_answers_on_deals_and_a_declined_deal_keeps_the_player() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../history.db");
        if !std::path::Path::new(path).exists() { return; }
        let conn = Connection::open(path).unwrap();
        let mut c = Career::new(&conn);
        c.load(&conn);
        c.start();
        let club = c.club.unwrap();
        c.finish_season();
        c.begin_next_season(club, &conn);
        let mut window = c.window.take().expect("the first club has deals in its first summer");
        let asked = window.questions_for(club);
        let kept = window.transfers()[asked[0]].player;
        let (sells, from) = (window.transfers()[asked[0]].from == Some(club), window.transfers()[asked[0]].from);
        window.set_declined(asked[0], true);
        c.start_next_season(club, window, &conn);
        let world = c.world.as_ref().unwrap();
        assert_eq!(world.edition, 16);
        if sells {
            assert_eq!(world.players[&kept].club, from, "the player you refused to sell is still yours");
        } else {
            assert_ne!(world.players[&kept].club, Some(club), "the player you refused to sign did not arrive");
        }
        assert!(world.calendar_problems().is_empty());
        assert!(world.players.values().filter(|p| p.career == CareerStatus::Retired).all(|p| p.club.is_none()), "a retired player has no club");
        assert!(world.players.values().any(|p| p.career == CareerStatus::Retiring), "the new season starts with some planning to retire");
    }

    #[test]
    fn a_match_can_be_watched_and_its_result_is_stored_when_it_is_over() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../history.db");
        if !std::path::Path::new(path).exists() { return; }
        let conn = Connection::open(path).unwrap();
        let mut c = Career::new(&conn);
        c.load(&conn);
        c.start();
        let club = c.club.unwrap();
        c.watch_next_match(club);
        assert!(c.live.is_some(), "the next match opens to be watched");
        c.finish_watching();
        assert!(c.live.is_none());
        let name = c.world.as_ref().unwrap().clubs[&club].name.clone();
        assert!(c.last_played.as_deref().is_some_and(|t| t.contains(&name)));
        assert!(c.world.as_ref().unwrap().fixtures.iter().any(|f| f.involves(club) && f.result.is_some()));
    }
}
