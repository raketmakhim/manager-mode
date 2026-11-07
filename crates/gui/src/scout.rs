//! Player scout tab: search a player, then see how their stats really changed in later editions.
use crate::{GREEN, RED};
use eframe::egui::{self, RichText};
use engine::{career, edition_label, player_name, search, Connection, Hit, Snapshot, KEEPER, OUTFIELD};

#[derive(Default)]
pub struct Scout {
    query: String,
    hits: Vec<Hit>,
    open_id: Option<i64>,
    name: String,
    career: Vec<Snapshot>,
    /// Selected edition (index into `career`).
    col: usize,
    error: Option<String>,
}

impl Scout {
    fn refresh_search(&mut self, conn: &Connection) {
        self.hits.clear();
        if self.query.trim().len() >= 2 {
            match search(conn, &self.query, 40) {
                Ok(h) => self.hits = h,
                Err(e) => self.error = Some(e.to_string()),
            }
        }
    }

    fn open_player(&mut self, conn: &Connection, id: i64) {
        match (player_name(conn, id), career(conn, id)) {
            (Ok(n), Ok(c)) => {
                self.name = n;
                self.career = c;
                self.col = 0;
                self.open_id = Some(id);
            }
            (Err(e), _) | (_, Err(e)) => self.error = Some(e.to_string()),
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, conn: &Connection) {
        egui::Panel::left("search").default_size(300.0).show(ui, |ui| {
            ui.heading("Find a player");
            let input = ui.add(egui::TextEdit::singleline(&mut self.query).hint_text("type a name").desired_width(f32::INFINITY));
            if input.changed() {
                self.refresh_search(conn);
            }
            if input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if let Some(h) = self.hits.first() {
                    let id = h.player_id;
                    self.open_player(conn, id);
                }
            }
            if let Some(e) = &self.error {
                ui.colored_label(RED, e);
            }
            ui.separator();
            let mut clicked = None;
            egui::ScrollArea::vertical().show(ui, |ui| {
                for h in &self.hits {
                    let text = format!("{}\n{} to {}, best {}", h.name, edition_label(h.first), edition_label(h.last), h.best);
                    if ui.selectable_label(self.open_id == Some(h.player_id), text).clicked() {
                        clicked = Some(h.player_id);
                    }
                }
            });
            if let Some(id) = clicked {
                self.open_player(conn, id);
            }
        });

        egui::CentralPanel::default_margins().show(ui, |ui| {
            if self.career.is_empty() {
                ui.label("Search for a player on the left.");
                return;
            }
            let c = &self.career;
            let last = &c[c.len() - 1];
            ui.heading(&self.name);
            ui.label(format!("{}   {} to {}", last.positions, edition_label(c[0].version), edition_label(last.version)));
            ui.add_space(8.0);

            // Pick a season: the line underneath shows what happened next.
            ui.horizontal(|ui| {
                ui.label("Season:");
                for (i, s) in c.iter().enumerate() {
                    if ui.selectable_label(self.col == i, edition_label(s.version)).clicked() {
                        self.col = i;
                    }
                }
            });
            let sel = &c[self.col];
            ui.label(match c.get(self.col + 1) {
                Some(n) => format!("From {}: overall {} -> {} next edition, potential {} -> {}", edition_label(sel.version),
                    sel.overall.unwrap_or(0), n.overall.unwrap_or(0), sel.potential.unwrap_or(0), n.potential.unwrap_or(0)),
                None => format!("{} is the last edition we have data for", edition_label(sel.version)),
            });
            ui.separator();

            let prev = |i: usize| i.checked_sub(1).map(|p| &c[p]);
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Header row helper: underlines the selected season.
                let header = |ui: &mut egui::Ui, title: &str| {
                    ui.label(RichText::new(title).strong());
                    for (i, s) in c.iter().enumerate() {
                        let t = RichText::new(edition_label(s.version)).strong();
                        ui.label(if i == self.col { t.underline() } else { t });
                    }
                    ui.end_row();
                };

                egui::Grid::new("career").striped(true).min_col_width(70.0).show(ui, |ui| {
                    header(ui, "Career");
                    ui.label("Age");
                    for s in c { ui.label(s.age.map_or("-".into(), |a| a.to_string())); }
                    ui.end_row();
                    ui.label("Overall");
                    for (i, s) in c.iter().enumerate() { ui.label(delta_label(s.overall, prev(i).and_then(|p| p.overall))); }
                    ui.end_row();
                    ui.label("Potential");
                    for (i, s) in c.iter().enumerate() { ui.label(delta_label(s.potential, prev(i).and_then(|p| p.potential))); }
                    ui.end_row();
                    ui.label("Club");
                    for s in c { ui.label(s.club.clone().unwrap_or("-".into())); }
                    ui.end_row();
                    ui.label("Value");
                    for s in c { ui.label(money(s.value, "M", 1e6)); }
                    ui.end_row();
                    ui.label("Wage");
                    for s in c { ui.label(money(s.wage, "k", 1e3)); }
                    ui.end_row();
                });
                ui.add_space(12.0);

                // Goalkeepers show keeper stats, everyone else the outfield ones.
                let is_gk = last.positions.split(',').any(|p| p.trim() == "GK");
                let list = if is_gk { KEEPER } else { OUTFIELD };
                egui::Grid::new("attrs").striped(true).min_col_width(70.0).show(ui, |ui| {
                    header(ui, "Attributes");
                    for &a in list {
                        ui.label(a.label());
                        for (i, s) in c.iter().enumerate() {
                            ui.label(delta_label(s.attrs[a].map(i64::from), prev(i).and_then(|p| p.attrs[a].map(i64::from))));
                        }
                        ui.end_row();
                    }
                });
            });
        });
    }
}

/// "82" or "82  +2" / "82  -1" compared with the previous edition.
fn delta_label(cur: Option<i64>, prev: Option<i64>) -> RichText {
    let Some(v) = cur else { return RichText::new("-") };
    match prev.map(|p| v - p) {
        Some(d) if d > 0 => RichText::new(format!("{v}  +{d}")).color(GREEN),
        Some(d) if d < 0 => RichText::new(format!("{v}  {d}")).color(RED),
        _ => RichText::new(v.to_string()),
    }
}

fn money(v: Option<i64>, suffix: &str, div: f64) -> String {
    v.map_or("-".into(), |v| format!("€{:.1}{suffix}", v as f64 / div))
}
