//! The club's fixture list: every league match and cup tie with its date, in date order.
use crate::{GREEN, RED};
use eframe::egui::{self, Color32, RichText};
use engine::{match_date, ClubId, Day, Side, World};

/// Shows the fixtures of `club_id`.
pub fn ui(ui: &mut egui::Ui, world: &World, club_id: ClubId) {
    // One row per match, plus a row for each cup the club is in but does not play in the first round (a bye).
    let mut rows: Vec<Row> = world
        .fixtures
        .iter()
        .filter(|f| f.home == club_id || f.away == club_id)
        .map(|f| {
            let home = f.home == club_id;
            Row {
                week: f.week,
                day: f.day,
                competition: world.competitions[&f.competition].name.clone(),
                round: f.round.to_string(),
                venue: if home { "H" } else { "A" },
                opponent: world.clubs[if home { &f.away } else { &f.home }].name.clone(),
                result: f.result.map_or("-".to_string(), |o| o.to_string()),
                colour: f.result.and_then(|o| o.winner()).map(|winner| if (winner == Side::Home) == home { GREEN } else { RED }),
            }
        })
        .collect();
    let mut knocked_out = vec![];
    for cup in world.competitions.values().filter(|c| c.is_cup() && c.clubs.contains(&club_id)) {
        let drawn = || world.fixtures.iter().filter(|f| f.competition == cup.id);
        let mine = || drawn().filter(|f| f.home == club_id || f.away == club_id);
        if !mine().any(|f| f.round == 1) {
            let (week, day) = drawn().map(|f| (f.week, f.day)).min().unwrap_or((1, Day::Wednesday));
            rows.push(Row { week, day, competition: cup.name.clone(), round: "1".into(), venue: "", opponent: "bye: into round 2".into(), result: "-".into(), colour: None });
        }
        if let Some(last) = mine().max_by_key(|f| f.round) {
            let loser = if last.result.and_then(|o| o.winner()) == Some(Side::Home) { last.away } else { last.home };
            if last.result.is_some() && loser == club_id {
                knocked_out.push(format!("Knocked out of the {} in round {}.", cup.name, last.round));
            }
        }
    }
    rows.sort_by_key(|r| (r.week, r.day));
    ui.label(format!("{} fixtures. Cup rounds after the first are drawn as results come in.", rows.len()));
    for text in &knocked_out {
        ui.label(RichText::new(text).strong());
    }
    ui.add_space(6.0);
    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("career_fixtures").striped(true).min_col_width(100.0).show(ui, |ui| {
            for h in ["Date", "Competition", "Round", "", "Opponent", "Result"] {
                ui.label(RichText::new(h).strong());
            }
            ui.end_row();
            for r in rows {
                ui.label(match_date(world.year, r.week, r.day));
                ui.label(r.competition);
                ui.label(r.round);
                ui.label(r.venue);
                ui.label(r.opponent);
                let result = RichText::new(r.result);
                ui.label(r.colour.map_or(result.clone(), |c| result.color(c)));
                ui.end_row();
            }
        });
    });
}

/// One line of the fixture table.
struct Row {
    /// Season week, 1 = first.
    week: u8,
    day: Day,
    competition: String,
    round: String,
    /// "H", "A", or empty for a bye.
    venue: &'static str,
    opponent: String,
    result: String,
    /// Green for a win, red for a loss, none for a draw or an unplayed match.
    colour: Option<Color32>,
}
