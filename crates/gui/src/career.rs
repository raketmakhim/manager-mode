//! Career tab: start a new career (pick a year, a league and a club), then see that club's fixtures and dates.
use crate::season::{league_label, sorted_leagues};
use crate::squad::{self, Checkpoint};
use crate::{competitions, fixtures, RED};
use eframe::egui;
use engine::{advance_ratings, edition_label, editions, load_edition, season_label, season_name, ClubId, CompetitionId, Connection, Rng, World};

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
    /// Your squad's ratings at the start of each season so far, for the squad view.
    checkpoints: Vec<Checkpoint>,
    squad: squad::State,
    /// Seeded when the career starts; every simulated match draws from it.
    rng: Rng,
    /// The result of the last match you played, for the line next to the button.
    last_played: Option<String>,
    error: Option<String>,
}

impl Career {
    pub fn new(conn: &Connection) -> Self {
        let editions = editions(conn).unwrap_or_default();
        Career { edition: editions.first().copied().unwrap_or(15), editions, world: None, league: None, club: None, started: false, view: View::Fixtures, competition: None, checkpoints: vec![], squad: squad::State::default(), rng: Rng::new(0), last_played: None, error: None }
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
        self.checkpoints = vec![Checkpoint::of_squad(world, club, season_name(world.year))];
        self.squad = squad::State::default();
        self.started = true;
    }

    /// Plays the club's next match (and everything before it in the league and cups) and remembers its result.
    fn play_next_match(&mut self, club: ClubId) {
        let Some(world) = &mut self.world else { return };
        self.last_played = Some(match world.play_next_match(club, &mut self.rng) {
            None => "Nothing left to play.".to_string(),
            Some(index) => {
                let f = &world.fixtures[index];
                let result = f.result.map_or(String::new(), |o| o.to_string());
                format!("{} {} {}", world.clubs[&f.home].name, result, world.clubs[&f.away].name)
            }
        });
    }

    /// Simulates every match that is left in the season, for all clubs.
    fn finish_season(&mut self) {
        let Some(world) = &mut self.world else { return };
        world.play_season_to_end(&mut self.rng);
        self.last_played = Some("Season finished.".to_string());
    }

    /// Promotion and relegation, then the next season's fixtures. Says what happened to your club.
    fn start_next_season(&mut self, club: ClubId, conn: &Connection) {
        let Some(world) = &mut self.world else { return };
        let Some(moves) = world.start_next_season(&mut self.rng) else { return };
        let ratings = match advance_ratings(conn, world) {
            Ok(true) => "Player ratings updated to the real new season.",
            Ok(false) => "No newer data: player ratings are unchanged.",
            Err(_) => "Could not read the new ratings.",
        };
        self.checkpoints.push(Checkpoint::of_squad(world, club, season_name(world.year)));
        let mine = moves.iter().find(|m| m.club == club);
        let text = match mine {
            Some(m) => {
                let verb = if world.competitions[&m.to].level < world.competitions[&m.from].level { "promoted to" } else { "relegated to" };
                format!("Your club was {verb} the {}.", world.competitions[&m.to].name)
            }
            _ => "Your club stays in its division.".to_string(),
        };
        self.last_played = Some(format!("{} clubs changed division. {text} {ratings}", moves.len()));
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
        let (club, league) = (&world.clubs[&club_id], &world.competitions[&world.clubs[&club_id].league]);
        let title = format!("{} · season {}", league_label(league), season_name(world.year));
        ui.horizontal(|ui| {
            ui.heading(&club.name);
            ui.label(title);
        });
        ui.horizontal(|ui| {
            let finished = self.world.as_ref().is_some_and(|w| w.season_finished());
            if ui.add_enabled(!finished, egui::Button::new("Play next match")).clicked() {
                self.play_next_match(club_id);
            }
            if ui.add_enabled(!finished, egui::Button::new("Finish season")).clicked() {
                self.finish_season();
            }
            if ui.add_enabled(finished, egui::Button::new("Start next season")).clicked() {
                self.start_next_season(club_id, conn);
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

        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.view, View::Fixtures, "My fixtures");
            ui.selectable_value(&mut self.view, View::Squad, "My squad");
            ui.selectable_value(&mut self.view, View::Competitions, "Competitions");
        });
        ui.add_space(6.0);
        match self.view {
            View::Fixtures => fixtures::ui(ui, world, club_id),
            View::Squad => squad::ui(ui, world, club_id, &self.checkpoints, &mut self.squad),
            View::Competitions => competitions::ui(ui, world, club_id, &mut self.competition),
        }
    }
}

/// What the season screen shows.
#[derive(PartialEq)]
enum View {
    Fixtures,
    Competitions,
    Squad,
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
        let mine = |id: CompetitionId| world.fixtures.iter().filter(|f| f.competition == id && (f.home == club || f.away == club)).count();
        let n = world.competitions[&league].clubs.len();
        assert_eq!(mine(league), 2 * (n - 1));
        assert!(world.calendar_problems().is_empty());
        let name = world.clubs[&club].name.clone();
        c.play_next_match(club);
        assert!(c.last_played.as_deref().is_some_and(|t| t.contains(&name)));
    }
}
