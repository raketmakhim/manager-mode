//! The Awards view: for a season and one competition (or all of them, the Ballon d'Or), the best players, the best
//! young player, the best of each line, the best eleven, the top scorers and the assist leaders. A cup only has the
//! scorers and the assist leaders.
use crate::leagues::{competition_list, league_label};
use eframe::egui::{self, RichText};
use engine::{season_name, ClubId, CompetitionId, Contender, Line, World};
use std::collections::BTreeSet;

/// Rows of the ranking: the Ballon d'Or lists more than a league's player of the season.
const RANKING_SHOWN_ALL: usize = 30;
const RANKING_SHOWN: usize = 10;
const LEADERS_SHOWN: usize = 10;

#[derive(Default)]
pub struct State {
    /// Start year of the season shown; `None` is the latest one with stats.
    season: Option<u16>,
    /// `None` is every competition, the Ballon d'Or.
    competition: Option<CompetitionId>,
}

fn line_label(line: Line) -> &'static str {
    match line {
        Line::Goalkeeper => "Goalkeeper",
        Line::Defence => "Defender",
        Line::Midfield => "Midfielder",
        Line::Attack => "Attacker",
    }
}

pub fn ui(ui: &mut egui::Ui, world: &World, my_club: ClubId, state: &mut State) {
    let seasons: BTreeSet<u16> = world.stats.keys().map(|&(_, year, _)| year).collect();
    let Some(&latest) = seasons.iter().next_back() else {
        ui.label("Nothing to show yet: play some matches.");
        return;
    };
    let season = state.season.filter(|s| seasons.contains(s)).unwrap_or(latest);
    let competitions: BTreeSet<CompetitionId> = world.stats.keys().filter(|&&(_, year, _)| year == season).map(|&(_, _, competition)| competition).collect();
    ui.horizontal(|ui| {
        ui.label("Season:");
        egui::ComboBox::from_id_salt("awards_season").selected_text(season_name(season)).show_ui(ui, |ui| {
            for &year in seasons.iter().rev() {
                ui.selectable_value(&mut state.season, Some(year), season_name(year));
            }
        });
        ui.label("Competition:");
        let competition_text = state.competition.and_then(|c| world.competitions.get(&c)).map_or("All competitions (Ballon d'Or)".to_string(), league_label);
        egui::ComboBox::from_id_salt("awards_competition").width(280.0).selected_text(competition_text).show_ui(ui, |ui| {
            ui.selectable_value(&mut state.competition, None, "All competitions (Ballon d'Or)");
            let listed: Vec<&engine::Competition> = competitions.iter().filter_map(|c| world.competitions.get(c)).collect();
            competition_list(ui, &mut state.competition, &listed);
        });
    });
    ui.add_space(6.0);

    let covered: Vec<CompetitionId> = state.competition.filter(|c| competitions.contains(c)).map_or(competitions.iter().copied().collect(), |c| vec![c]);
    let awards = world.awards(season, &covered);
    let cup = state.competition.and_then(|c| world.competitions.get(&c)).is_some_and(|c| c.is_cup());
    if awards.top_scorers().is_empty() && awards.top_assists().is_empty() && awards.ranking().is_empty() {
        ui.label("Nothing to show yet: play some matches.");
        return;
    }
    let all = state.competition.is_none();
    egui::ScrollArea::vertical().show(ui, |ui| {
        if !cup {
            ui.label(RichText::new(if all { "Ballon d'Or" } else { "Player of the season" }).heading());
            winners_ui(ui, world, my_club, &awards);
            ui.add_space(10.0);
            ui.label(RichText::new(if all { "Ballon d'Or ranking" } else { "Ranking" }).strong());
            players_ui(ui, world, my_club, "awards_ranking", awards.ranking().iter().take(if all { RANKING_SHOWN_ALL } else { RANKING_SHOWN }));
            ui.add_space(10.0);
            ui.label(RichText::new("Best eleven").strong());
            players_ui(ui, world, my_club, "awards_eleven", awards.best_eleven().into_iter());
            ui.add_space(10.0);
        }
        ui.label(RichText::new("Top scorers").strong());
        players_ui(ui, world, my_club, "awards_scorers", awards.top_scorers().iter().take(LEADERS_SHOWN));
        ui.add_space(10.0);
        ui.label(RichText::new("Assist leaders").strong());
        players_ui(ui, world, my_club, "awards_assists", awards.top_assists().iter().take(LEADERS_SHOWN));
    });
}

/// The single winners: best player, best young player and the best of each line.
fn winners_ui(ui: &mut egui::Ui, world: &World, my_club: ClubId, awards: &engine::Awards) {
    let lines = [Line::Goalkeeper, Line::Defence, Line::Midfield, Line::Attack];
    let rows = [("Best player", awards.best_player()), ("Best young player (21 or younger)", awards.best_young_player())]
        .into_iter()
        .chain(lines.map(|line| (match line { Line::Goalkeeper => "Best goalkeeper", Line::Defence => "Best defender", Line::Midfield => "Best midfielder", Line::Attack => "Best attacker" }, awards.best_of_line(line))));
    egui::Grid::new("awards_winners").striped(true).min_col_width(60.0).show(ui, |ui| {
        for (title, winner) in rows {
            ui.label(RichText::new(title).strong());
            match winner {
                Some(c) => {
                    ui.label(player_text(world, c, my_club));
                    ui.label(club_name(world, c));
                }
                None => {
                    ui.label("-");
                    ui.label("");
                }
            }
            ui.end_row();
        }
    });
}

/// A table of players: place, name, club, line, age, goals and score.
fn players_ui<'a>(ui: &mut egui::Ui, world: &World, my_club: ClubId, id: &str, players: impl Iterator<Item = &'a Contender>) {
    egui::Grid::new(id).striped(true).min_col_width(40.0).show(ui, |ui| {
        for h in ["#", "Player", "Club", "Position", "Age", "Goals", "Assists", "Score"] {
            ui.label(RichText::new(h).strong());
        }
        ui.end_row();
        for (place, c) in players.enumerate() {
            ui.label((place + 1).to_string());
            ui.label(player_text(world, c, my_club));
            ui.label(club_name(world, c));
            ui.label(line_label(c.line));
            ui.label(c.age.to_string());
            ui.label(c.goals.to_string());
            ui.label(c.assists.to_string());
            ui.label(format!("{:.1}", c.score));
            ui.end_row();
        }
    });
}

/// The player's name, in bold if he plays for your club.
fn player_text(world: &World, c: &Contender, my_club: ClubId) -> RichText {
    let player = &world.players[&c.player];
    let text = RichText::new(&player.name);
    if player.club == Some(my_club) { text.strong() } else { text }
}

/// The club he plays for now, which is not always where he won the award.
fn club_name(world: &World, c: &Contender) -> String {
    world.players[&c.player].club.map_or(String::new(), |club| world.clubs[&club].name.clone())
}
