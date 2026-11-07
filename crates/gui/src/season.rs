//! Season tab: pick an edition, start a season, browse the fixtures.
use crate::RED;
use eframe::egui::{self, RichText};
use engine::{edition_label, editions, load_edition, season_label, ClubId, Competition, CompetitionId, Connection, World};

pub struct Season {
    /// Editions in the database, oldest first.
    editions: Vec<i64>,
    /// The edition picked in the drop-down (not started until you press the button).
    edition: i64,
    world: Option<World>,
    league: Option<CompetitionId>,
    /// `None`: show one round of the whole league. `Some`: show that club's whole season.
    club: Option<ClubId>,
    /// 1 = first round.
    round: u8,
    error: Option<String>,
}

impl Season {
    pub fn new(conn: &Connection) -> Self {
        let editions = editions(conn).unwrap_or_default();
        Season { edition: editions.first().copied().unwrap_or(15), editions, world: None, league: None, club: None, round: 1, error: None }
    }

    /// Loads the picked edition and makes its fixtures.
    fn start(&mut self, conn: &Connection) {
        match load_edition(conn, self.edition) {
            Ok(mut world) => {
                world.generate_fixtures();
                self.league = sorted_leagues(&world).first().map(|l| l.id);
                self.world = Some(world);
                self.club = None;
                self.round = 1;
                self.error = None;
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, conn: &Connection) {
        egui::CentralPanel::default_margins().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Start a season in:");
                let label = |e: i64| format!("{} ({})", edition_label(e), season_label(e));
                egui::ComboBox::from_id_salt("edition").selected_text(label(self.edition)).show_ui(ui, |ui| {
                    for &e in &self.editions {
                        ui.selectable_value(&mut self.edition, e, label(e));
                    }
                });
                if ui.button("Start season").clicked() {
                    self.start(conn);
                }
            });
            if let Some(e) = &self.error {
                ui.colored_label(RED, e);
            }
            let Some(world) = &self.world else {
                ui.label("Pick an edition and press Start season.");
                return;
            };
            ui.separator();

            // League and club pickers.
            let leagues = sorted_leagues(world);
            let Some(league) = self.league.and_then(|id| world.competitions.get(&id)) else { return };
            let mut clubs: Vec<_> = league.clubs.iter().map(|id| &world.clubs[id]).collect();
            clubs.sort_by(|a, b| a.name.cmp(&b.name));
            ui.horizontal(|ui| {
                ui.label("League:");
                let before = self.league;
                egui::ComboBox::from_id_salt("league").selected_text(league_label(league)).show_ui(ui, |ui| {
                    for l in &leagues {
                        ui.selectable_value(&mut self.league, Some(l.id), league_label(l));
                    }
                });
                if self.league != before {
                    self.club = None;
                    self.round = 1;
                }
                ui.label("Show:");
                let selected = self.club.map_or("one round at a time".to_string(), |id| world.clubs[&id].name.clone());
                egui::ComboBox::from_id_salt("club").selected_text(selected).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.club, None, "one round at a time");
                    for c in &clubs {
                        ui.selectable_value(&mut self.club, Some(c.id), &c.name);
                    }
                });
            });

            let fixtures: Vec<_> = world.fixtures.iter().filter(|f| f.competition == league.id).collect();
            let rounds = fixtures.iter().map(|f| f.round).max().unwrap_or(0);
            ui.label(format!("{}: {} clubs, {} rounds, {} matches. Season {}.", league_label(league), league.clubs.len(), rounds, fixtures.len(), season_label(world.edition)));
            ui.add_space(6.0);

            let name = |id: ClubId| world.clubs[&id].name.as_str();
            let result = |f: &engine::Fixture| f.result.map_or("-".to_string(), |o| o.to_string());
            match self.club {
                None => {
                    ui.horizontal(|ui| {
                        if ui.add_enabled(self.round > 1, egui::Button::new("◀")).clicked() {
                            self.round -= 1;
                        }
                        ui.label(RichText::new(format!("Round {} of {}", self.round, rounds)).strong());
                        if ui.add_enabled(self.round < rounds, egui::Button::new("▶")).clicked() {
                            self.round += 1;
                        }
                    });
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        egui::Grid::new("round").striped(true).min_col_width(120.0).show(ui, |ui| {
                            for f in fixtures.iter().filter(|f| f.round == self.round) {
                                ui.label(name(f.home));
                                ui.label(result(f));
                                ui.label(name(f.away));
                                ui.end_row();
                            }
                        });
                    });
                }
                Some(club) => {
                    let mut mine: Vec<_> = fixtures.iter().filter(|f| f.home == club || f.away == club).collect();
                    mine.sort_by_key(|f| f.round);
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        egui::Grid::new("club").striped(true).min_col_width(120.0).show(ui, |ui| {
                            for h in ["Round", "Home", "", "Away"] {
                                ui.label(RichText::new(h).strong());
                            }
                            ui.end_row();
                            for f in mine {
                                ui.label(f.round.to_string());
                                ui.label(if f.home == club { RichText::new(name(f.home)).strong() } else { RichText::new(name(f.home)) });
                                ui.label(result(f));
                                ui.label(if f.away == club { RichText::new(name(f.away)).strong() } else { RichText::new(name(f.away)) });
                                ui.end_row();
                            }
                        });
                    });
                }
            }
        });
    }
}

/// Leagues that have fixtures (two or more clubs), A to Z by name then country.
fn sorted_leagues(world: &World) -> Vec<&Competition> {
    let mut v: Vec<&Competition> =
        world.competitions.values().filter(|c| c.is_league() && c.clubs.len() >= 2).collect();
    v.sort_by(|a, b| (&a.name, &a.country).cmp(&(&b.name, &b.country)));
    v
}

fn league_label(l: &Competition) -> String {
    format!("{} ({})", l.name, l.country)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starting_a_season_gives_a_league_with_fixtures() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../history.db");
        if !std::path::Path::new(path).exists() { return; } // needs `cargo run -p import` first
        let conn = Connection::open(path).unwrap();
        let mut s = Season::new(&conn);
        assert_eq!(s.editions.len(), 10);
        s.start(&conn);
        let world = s.world.as_ref().unwrap();
        let league = &world.competitions[&s.league.unwrap()];
        assert!(league.clubs.len() >= 2);
        assert!(world.fixtures.iter().any(|f| f.competition == league.id));
        assert_eq!((s.round, s.club), (1, None));
    }
}
