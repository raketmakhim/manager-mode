//! Career tab: start a new career (pick a year, a league and a club), then see that club's fixtures and dates.
use crate::season::{league_label, sorted_leagues};
use crate::RED;
use eframe::egui::{self, RichText};
use engine::{edition_label, editions, load_edition, season_label, week_date, ClubId, CompetitionId, Connection, Rng, World};

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
    error: Option<String>,
}

impl Career {
    pub fn new(conn: &Connection) -> Self {
        let editions = editions(conn).unwrap_or_default();
        Career { edition: editions.first().copied().unwrap_or(15), editions, world: None, league: None, club: None, started: false, error: None }
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
        if let Some(world) = &mut self.world {
            world.start_season(&mut Rng::new(self.edition as u64));
            self.started = true;
        }
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
        let (Some(world), Some(club_id)) = (&self.world, self.club) else { return };
        let club = &world.clubs[&club_id];
        let league = &world.competitions[&club.league];
        ui.horizontal(|ui| {
            ui.heading(&club.name);
            ui.label(format!("{} · season {}", league_label(league), season_label(world.edition)));
        });
        if ui.button("New career").clicked() {
            self.started = false;
            self.load(conn); // start_season changed the world, so load a clean one
            return;
        }
        ui.separator();

        // One row per match, plus a row for each cup the club is in but does not play in the first round (a bye).
        let mut rows: Vec<Row> = world
            .fixtures
            .iter()
            .filter(|f| f.home == club_id || f.away == club_id)
            .map(|f| {
                let home = f.home == club_id;
                Row {
                    week: f.week,
                    competition: world.competitions[&f.competition].name.clone(),
                    round: f.round.to_string(),
                    venue: if home { "H" } else { "A" },
                    opponent: world.clubs[if home { &f.away } else { &f.home }].name.clone(),
                    result: f.result.map_or("-".to_string(), |o| o.to_string()),
                }
            })
            .collect();
        for cup in world.competitions.values().filter(|c| c.is_cup() && c.clubs.contains(&club_id)) {
            let drawn = || world.fixtures.iter().filter(|f| f.competition == cup.id);
            if !drawn().any(|f| f.home == club_id || f.away == club_id) {
                let week = drawn().map(|f| f.week).min().unwrap_or(1);
                rows.push(Row { week, competition: cup.name.clone(), round: "1".into(), venue: "", opponent: "bye: into round 2".into(), result: "-".into() });
            }
        }
        rows.sort_by(|a, b| (a.week, &a.competition).cmp(&(b.week, &b.competition)));
        ui.label(format!("{} fixtures. Cup rounds after the first are drawn as results come in.", rows.len()));
        ui.add_space(6.0);
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("career_fixtures").striped(true).min_col_width(100.0).show(ui, |ui| {
                for h in ["Date", "Competition", "Round", "", "Opponent", "Result"] {
                    ui.label(RichText::new(h).strong());
                }
                ui.end_row();
                for r in rows {
                    ui.label(week_date(world.year, r.week));
                    ui.label(r.competition);
                    ui.label(r.round);
                    ui.label(r.venue);
                    ui.label(r.opponent);
                    ui.label(r.result);
                    ui.end_row();
                }
            });
        });
    }
}

/// One line of the fixture table.
struct Row {
    /// Season week, 1 = first.
    week: u8,
    competition: String,
    round: String,
    /// "H", "A", or empty for a bye.
    venue: &'static str,
    opponent: String,
    result: String,
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
    }
}
