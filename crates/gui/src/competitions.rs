//! The Competitions view: pick a league or cup and see its table or its rounds.
use crate::leagues::league_label;
use eframe::egui::{self, RichText};
use engine::{match_date, ClubId, Competition, CompetitionId, Fixture, World};

/// Shows one competition. `selected` is the pick; `None` means the league of `my_club`.
pub fn ui(ui: &mut egui::Ui, world: &World, my_club: ClubId, selected: &mut Option<CompetitionId>) {
    let current = selected.unwrap_or(world.clubs[&my_club].league);
    let Some(competition) = world.competitions.get(&current) else { return };

    let mut all: Vec<&Competition> = world.competitions.values().filter(|c| c.clubs.len() >= 2).collect();
    all.sort_by(|a, b| (&a.country, a.is_cup(), &a.name).cmp(&(&b.country, b.is_cup(), &b.name)));
    ui.horizontal(|ui| {
        ui.label("Competition:");
        egui::ComboBox::from_id_salt("competition").width(320.0).selected_text(league_label(competition)).show_ui(ui, |ui| {
            for c in all {
                ui.selectable_value(selected, Some(c.id), league_label(c));
            }
        });
    });
    ui.add_space(6.0);

    if competition.is_cup() {
        cup_ui(ui, world, competition, my_club);
    } else {
        table_ui(ui, world, competition, my_club);
    }
}

/// A club's name, in bold if it is the player's club.
fn club_text(world: &World, club: ClubId, my_club: ClubId) -> RichText {
    let text = RichText::new(&world.clubs[&club].name);
    if club == my_club { text.strong() } else { text }
}

fn table_ui(ui: &mut egui::Ui, world: &World, league: &Competition, my_club: ClubId) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("league_table").striped(true).min_col_width(34.0).show(ui, |ui| {
            for h in ["#", "Club", "P", "W", "D", "L", "GF", "GA", "GD", "Pts"] {
                ui.label(RichText::new(h).strong());
            }
            ui.end_row();
            for (place, row) in world.table(league.id).iter().enumerate() {
                ui.label((place + 1).to_string());
                ui.label(club_text(world, row.club, my_club));
                for n in [row.played as i32, row.won as i32, row.drawn as i32, row.lost as i32, row.goals_for as i32, row.goals_against as i32, row.goal_difference(), row.points as i32] {
                    ui.label(n.to_string());
                }
                ui.end_row();
            }
        });
    });
}

fn cup_ui(ui: &mut egui::Ui, world: &World, cup: &Competition, my_club: ClubId) {
    let fixtures: Vec<&Fixture> = world.fixtures.iter().filter(|f| f.competition == cup.id).collect();
    if fixtures.is_empty() {
        ui.label("No round has been drawn yet.");
        return;
    }
    let rounds = fixtures.iter().map(|f| f.round).max().unwrap_or(0);
    egui::ScrollArea::vertical().show(ui, |ui| {
        for round in 1..=rounds {
            let ties: Vec<&&Fixture> = fixtures.iter().filter(|f| f.round == round).collect();
            let Some(first) = ties.first() else { continue };
            ui.label(RichText::new(format!("Round {round} · {}", match_date(world.year, first.week, first.day))).strong());
            egui::Grid::new(("cup_round", round)).striped(true).min_col_width(160.0).show(ui, |ui| {
                for f in ties {
                    ui.label(club_text(world, f.home, my_club));
                    ui.label(f.result_text());
                    ui.label(club_text(world, f.away, my_club));
                    ui.end_row();
                }
            });
            ui.add_space(8.0);
        }
    });
}
