//! The squad view: your players with how their ratings changed since the start of an earlier season.
use crate::{GREEN, RED};
use eframe::egui::{self, Color32, RichText};
use engine::{Attrs, ClubId, Player, PlayerId, Position, World, KEEPER, OUTFIELD};
use std::collections::HashMap;

/// A player's ratings at one moment.
#[derive(Clone, Copy)]
struct Rating {
    overall: u8,
    potential: u8,
    attrs: Attrs,
}

/// The ratings of a club's players at the start of a season, to compare later ones with.
pub struct Checkpoint {
    /// "2014/15".
    season: String,
    ratings: HashMap<PlayerId, Rating>,
}

impl Checkpoint {
    /// Records the club's current squad. `season` is its name, like "2014/15".
    pub fn of_squad(world: &World, club: ClubId, season: String) -> Checkpoint {
        let ratings = world.clubs[&club]
            .squad
            .iter()
            .map(|id| {
                let p = &world.players[id];
                (*id, Rating { overall: p.overall, potential: p.potential, attrs: p.attrs })
            })
            .collect();
        Checkpoint { season, ratings }
    }
}

/// What the view remembers between frames.
#[derive(Default)]
pub struct State {
    /// Index of the checkpoint being compared with; `None` means the one before the latest.
    compare: Option<usize>,
    open: Option<PlayerId>,
}

/// Shows the squad of `club`, best first, with changes since the chosen checkpoint.
pub fn ui(ui: &mut egui::Ui, world: &World, club: ClubId, checkpoints: &[Checkpoint], state: &mut State) {
    let Some(last) = checkpoints.len().checked_sub(1) else { return };
    let chosen = state.compare.unwrap_or(last.saturating_sub(1)).min(last);
    ui.horizontal(|ui| {
        ui.label("Change since the start of:");
        egui::ComboBox::from_id_salt("squad_compare").selected_text(&checkpoints[chosen].season).show_ui(ui, |ui| {
            for (i, c) in checkpoints.iter().enumerate() {
                ui.selectable_value(&mut state.compare, Some(i), &c.season);
            }
        });
    });
    ui.add_space(6.0);
    let then = &checkpoints[chosen].ratings;

    let mut players: Vec<_> = world.clubs[&club].squad.iter().map(|id| &world.players[id]).collect();
    players.sort_by_key(|p| std::cmp::Reverse(p.overall));
    ui.columns(2, |columns| {
        egui::ScrollArea::vertical().id_salt("squad_table").show(&mut columns[0], |ui| {
            egui::Grid::new("squad").striped(true).min_col_width(40.0).show(ui, |ui| {
                for h in ["Player", "Pos", "Age", "Overall", "", "Potential", "", "Apps", "Goals", "Assists"] {
                    ui.label(RichText::new(h).strong());
                }
                ui.end_row();
                for p in players {
                    let was = then.get(&p.id);
                    let position = p.positions.first().map_or(String::new(), |pos| pos.code().to_string());
                    if ui.selectable_label(state.open == Some(p.id), &p.name).clicked() {
                        state.open = Some(p.id);
                    }
                    ui.label(position);
                    ui.label(age(&p.birth_date, world.year).map_or(String::new(), |a| a.to_string()));
                    ui.label(p.overall.to_string());
                    change_label(ui, was.map(|w| p.overall as i32 - w.overall as i32));
                    ui.label(p.potential.to_string());
                    change_label(ui, was.map(|w| p.potential as i32 - w.potential as i32));
                    let stats = world.season_stats(p.id);
                    for n in [stats.appearances, stats.goals, stats.assists] {
                        ui.label(n.to_string());
                    }
                    ui.end_row();
                }
            });
        });
        match state.open.and_then(|id| world.players.get(&id)) {
            Some(p) => attribute_changes(&mut columns[1], p, then.get(&p.id).map(|w| &w.attrs)),
            None => {
                columns[1].label("Click a player to see how each attribute changed.");
            }
        }
    });
}

/// Age in the season that starts in `season_start_year`, from a birth date like "1985-10-24".
fn age(birth_date: &str, season_start_year: u16) -> Option<u16> {
    let born: u16 = birth_date.get(..4)?.parse().ok()?;
    season_start_year.checked_sub(born)
}

/// "+3" in green, "-2" in red, "0" in grey, or nothing when there is nothing to compare with.
fn change_label(ui: &mut egui::Ui, change: Option<i32>) {
    let Some(change) = change else {
        ui.label("");
        return;
    };
    let (text, colour) = match change {
        0 => ("0".to_string(), Color32::GRAY),
        c if c > 0 => (format!("+{c}"), GREEN),
        c => (c.to_string(), RED),
    };
    ui.label(RichText::new(text).color(colour));
}

/// Every attribute the player has, grouped by category, with what it was and what it is now.
/// Attributes that changed are coloured; the rest stay grey.
fn attribute_changes(ui: &mut egui::Ui, player: &Player, then: Option<&Attrs>) {
    ui.label(RichText::new(&player.name).strong());
    let Some(then) = then else {
        ui.label("Not in the squad at that time.");
        return;
    };
    let list = if player.positions.first() == Some(&Position::Gk) { KEEPER } else { OUTFIELD };
    egui::ScrollArea::vertical().id_salt("attribute_changes").show(ui, |ui| {
        for group in list.chunk_by(|a, b| a.category() == b.category()) {
            ui.add_space(6.0);
            ui.label(RichText::new(capitalised(group[0].category())).strong());
            egui::Grid::new(("attribute_group", group[0].category())).striped(true).min_col_width(60.0).show(ui, |ui| {
                for &attr in group {
                    ui.label(attr.label());
                    match (then[attr], player.attrs[attr]) {
                        (Some(was), Some(now)) => {
                            ui.label(format!("{was} -> {now}"));
                            change_label(ui, Some(now as i32 - was as i32));
                        }
                        (None, Some(now)) => {
                            ui.label(now.to_string()); // the older edition did not rate this
                            ui.label("");
                        }
                        _ => {
                            ui.label("-");
                            ui.label("");
                        }
                    }
                    ui.end_row();
                }
            });
        }
    });
}

/// "attacking" becomes "Attacking".
fn capitalised(word: &str) -> String {
    let mut chars = word.chars();
    chars.next().map_or(String::new(), |first| first.to_uppercase().chain(chars).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_word_is_capitalised() {
        assert_eq!(capitalised("attacking"), "Attacking");
        assert_eq!(capitalised(""), "");
    }

    #[test]
    fn age_counts_from_the_birth_year() {
        assert_eq!(age("1985-10-24", 2014), Some(29));
        assert_eq!(age("", 2014), None);
        assert_eq!(age("2030-01-01", 2014), None);
    }

    #[test]
    fn a_checkpoint_taken_before_a_new_season_shows_the_real_changes() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../history.db");
        if !std::path::Path::new(path).exists() { return; } // needs `cargo run -p import` first
        let conn = engine::Connection::open(path).unwrap();
        let mut world = engine::load_edition(&conn, 15).unwrap();
        let club = *world.clubs.keys().next().unwrap();
        let before = Checkpoint::of_squad(&world, club, "2014/15".into());
        assert!(engine::advance_ratings(&conn, &mut world).unwrap());
        let changed = world.clubs[&club].squad.iter().filter(|id| world.players[*id].overall != before.ratings[*id].overall).count();
        assert!(changed > 0, "a whole squad's ratings did not move in a year");
        assert_eq!(before.ratings.len(), world.clubs[&club].squad.len());
    }
}
