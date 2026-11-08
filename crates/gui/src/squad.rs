//! The squad view: your players with how their ratings changed since the start of this season, and each one's overall over the seasons.
use crate::{GREEN, RED};
use eframe::egui::{self, Color32, RichText};
use engine::{Attrs, ClubId, Player, PlayerId, Position, World, KEEPER, OUTFIELD, XP_PER_LEVEL};
use std::collections::HashMap;

/// A player's ratings at one moment.
#[derive(Clone, Copy)]
struct Rating {
    overall: u8,
    potential: u8,
    attrs: Attrs,
}

/// What the squad view remembers about your players across seasons. Over seasons it keeps only each player's overall;
/// the attributes are kept only for the current season, because that is all the page compares.
pub struct History {
    /// Each season so far, oldest first: its name ("2014/15") and every squad player's overall at its start.
    overalls: Vec<(String, HashMap<PlayerId, u8>)>,
    /// Overall, potential and attributes of every squad player at the start of the current season.
    season_start: HashMap<PlayerId, Rating>,
}

impl History {
    /// A history that starts with the club's current squad as the start of season `season` ("2014/15").
    pub fn new(world: &World, club: ClubId, season: String) -> History {
        let mut history = History { overalls: vec![], season_start: HashMap::new() };
        history.start_season(world, club, season);
        history
    }

    /// Records the squad as it is now as the start of the next season. The old season's attributes are dropped.
    pub fn start_season(&mut self, world: &World, club: ClubId, season: String) {
        let squad = world.clubs[&club].squad.iter().map(|id| &world.players[id]);
        self.season_start = squad.clone().map(|p| (p.id, Rating { overall: p.overall, potential: p.potential, attrs: p.attrs })).collect();
        self.overalls.push((season, squad.map(|p| (p.id, p.overall)).collect()));
    }

    /// The current season's name.
    fn season(&self) -> &str {
        self.overalls.last().map_or("", |(name, _)| name)
    }

    /// A player's overall at the start of each season he was in the squad, oldest first.
    fn overall_by_season(&self, player: PlayerId) -> Vec<(&str, u8)> {
        self.overalls.iter().filter_map(|(season, overalls)| Some((season.as_str(), *overalls.get(&player)?))).collect()
    }
}

/// What the view remembers between frames.
#[derive(Default)]
pub struct State {
    open: Option<PlayerId>,
}

/// Shows the squad of `club`, best first, with the changes since the start of this season. Clicking a player opens his page.
pub fn ui(ui: &mut egui::Ui, world: &World, club: ClubId, history: &History, state: &mut State) {
    if let Some(p) = state.open.and_then(|id| world.players.get(&id)) {
        if player_page(ui, world, p, history) {
            state.open = None;
        }
        return;
    }
    ui.label(format!("Changes are since the start of {}. Open a player for his overall over the seasons.", history.season()));
    ui.label(format!("XP: every match gives 1 to 10 (less for a high overall). At {XP_PER_LEVEL} XP a player levels up and the rest stays in his bar. Last: XP of his last match."));
    ui.add_space(6.0);
    let then = &history.season_start;

    let mut players: Vec<_> = world.clubs[&club].squad.iter().map(|id| &world.players[id]).collect();
    players.sort_by_key(|p| std::cmp::Reverse(p.overall));
    egui::ScrollArea::both().id_salt("squad_table").show(ui, |ui| {
        egui::Grid::new("squad").striped(true).min_col_width(40.0).show(ui, |ui| {
            for h in ["Player", "Pos", "Age", "Overall", "", "Potential", "", "Apps", "Goals", "Assists", "Rating", "XP", "Last"] {
                ui.label(RichText::new(h).strong());
            }
            ui.end_row();
            for p in players {
                let was = then.get(&p.id);
                let position = p.positions.first().map_or(String::new(), |pos| pos.code().to_string());
                if ui.selectable_label(false, &p.name).clicked() {
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
                ui.label(stats.average_rating().map_or("-".to_string(), |r| format!("{r:.1}")));
                xp_bar(ui, p.xp);
                ui.label(world.last_match_xp.get(&p.id).map_or("-".to_string(), |xp| xp.to_string()));
                ui.end_row();
            }
        });
    });
}

/// The XP bar: how far to the next level.
fn xp_bar(ui: &mut egui::Ui, xp: u16) {
    ui.add(egui::ProgressBar::new(xp as f32 / XP_PER_LEVEL as f32).desired_width(80.0).text(format!("{xp}/{XP_PER_LEVEL}")));
}

/// A player's page: who he is, his overall and XP, what he did this season and every attribute with how it changed
/// since the start of this season, and his overall over the seasons. True when Back is pressed.
fn player_page(ui: &mut egui::Ui, world: &World, p: &Player, history: &History) -> bool {
    let then = history.season_start.get(&p.id);
    let back = ui.button("Back to the squad").clicked();
    ui.heading(&p.name);
    let positions: Vec<&str> = p.positions.iter().map(|pos| pos.code()).collect();
    ui.label(format!("{}   age {}", positions.join(", "), age(&p.birth_date, world.year).map_or("?".to_string(), |a| a.to_string())));
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label("Overall");
        ui.label(RichText::new(p.overall.to_string()).heading());
        change_label(ui, then.map(|w| p.overall as i32 - w.overall as i32));
        ui.add_space(16.0);
        ui.label("Potential");
        ui.label(RichText::new(p.potential.to_string()).heading());
        change_label(ui, then.map(|w| p.potential as i32 - w.potential as i32));
        ui.add_space(16.0);
        ui.label("XP");
        xp_bar(ui, p.xp);
        ui.label(format!("(last match {})", world.last_match_xp.get(&p.id).map_or("-".to_string(), |xp| xp.to_string())));
    });
    let stats = world.season_stats(p.id);
    ui.label(format!(
        "This season: {} apps, {} goals, {} assists, rating {}, {} XP",
        stats.appearances,
        stats.goals,
        stats.assists,
        stats.average_rating().map_or("-".to_string(), |r| format!("{r:.1}")),
        stats.xp
    ));
    ui.label(overall_track_text(&history.overall_by_season(p.id)));
    ui.label(format!("Attribute changes are since the start of {}.", history.season()));
    ui.separator();
    let Some(then) = then.map(|w| &w.attrs) else {
        ui.label("He was not in the squad at the start of the season, so there is nothing to compare with.");
        attribute_groups(ui, p, None);
        return back;
    };
    attribute_groups(ui, p, Some(then));
    back
}

/// Every attribute the player has, in blocks by category side by side, with its value and, if there is something to
/// compare with, how it changed. Attributes that changed are coloured; the rest stay grey.
fn attribute_groups(ui: &mut egui::Ui, player: &Player, then: Option<&Attrs>) {
    let list = if player.positions.first() == Some(&Position::Gk) { KEEPER } else { OUTFIELD };
    egui::ScrollArea::vertical().id_salt("player_page").show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            for group in list.chunk_by(|a, b| a.category() == b.category()) {
                ui.vertical(|ui| {
                    ui.set_min_width(210.0);
                    ui.label(RichText::new(capitalised(group[0].category())).strong());
                    egui::Grid::new(("attribute_group", group[0].category())).striped(true).min_col_width(50.0).show(ui, |ui| {
                        for &attr in group {
                            ui.label(attr.label());
                            match player.attrs[attr] {
                                Some(now) => {
                                    ui.label(RichText::new(now.to_string()).strong());
                                    change_label(ui, then.and_then(|t| t[attr]).map(|was| now as i32 - was as i32));
                                }
                                None => {
                                    ui.label("-");
                                    ui.label("");
                                }
                            }
                            ui.end_row();
                        }
                    });
                });
                ui.add_space(12.0);
            }
        });
    });
}

/// Age in the season that starts in `season_start_year`, from a birth date like "1985-10-24".
fn age(birth_date: &str, season_start_year: u16) -> Option<u16> {
    let born: u16 = birth_date.get(..4)?.parse().ok()?;
    season_start_year.checked_sub(born)
}

/// "Overall by season: 2014/15 74, 2015/16 81 (+7)": each season's starting overall and how it changed from the one
/// before.
fn overall_track_text(track: &[(&str, u8)]) -> String {
    let parts: Vec<String> = track
        .iter()
        .enumerate()
        .map(|(i, (season, overall))| match i.checked_sub(1).map(|before| *overall as i32 - track[before].1 as i32) {
            Some(change) => format!("{season} {overall} ({change:+})"),
            None => format!("{season} {overall}"),
        })
        .collect();
    format!("Overall by season: {}", parts.join(", "))
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
    fn the_overall_over_the_seasons_shows_each_change() {
        assert_eq!(overall_track_text(&[("2014/15", 74), ("2015/16", 81), ("2016/17", 80)]), "Overall by season: 2014/15 74, 2015/16 81 (+7), 2016/17 80 (-1)");
    }

    #[test]
    fn a_new_season_keeps_only_overalls_of_the_old_ones_and_the_attributes_of_the_new() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../history.db");
        if !std::path::Path::new(path).exists() { return; } // needs `cargo run -p import` first
        let conn = engine::Connection::open(path).unwrap();
        let mut world = engine::load_edition(&conn, 15).unwrap();
        let club = *world.clubs.keys().next().unwrap();
        let mut history = History::new(&world, club, "2014/15".into());
        assert!(engine::advance_ratings(&conn, &mut world).unwrap());
        let changed = world.clubs[&club].squad.iter().filter(|id| world.players[*id].overall != history.season_start[*id].overall).count();
        assert!(changed > 0, "a whole squad's ratings did not move in a year");
        history.start_season(&world, club, "2015/16".into());
        assert_eq!(history.season(), "2015/16");
        let someone = world.clubs[&club].squad[0];
        let track = history.overall_by_season(someone);
        assert_eq!(track.len(), 2);
        assert_eq!(track[1].1, world.players[&someone].overall);
        assert_eq!(history.season_start[&someone].overall, world.players[&someone].overall, "this season's changes start from now");
    }
}
