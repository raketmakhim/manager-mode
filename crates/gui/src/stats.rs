//! The Stats view: the leading players of the career so far, all time or one season, in every competition or one.
use crate::leagues::league_label;
use eframe::egui::{self, RichText};
use engine::{season_name, ClubId, CompetitionId, PlayerId, PlayerStats, World};
use std::collections::BTreeSet;

/// How many players the Stats view shows.
const SHOWN: usize = 25;

/// What the table is ranked by.
#[derive(Clone, Copy, PartialEq)]
enum Rank {
    Goals,
    Assists,
    Appearances,
    CleanSheets,
    /// Average match rating; only players with `MIN_RATED_FOR_LEADERS` rated matches count.
    Rating,
    PlayerOfTheMatch,
}

/// Rated matches a player needs to be ranked by average rating (one lucky game would top the list otherwise).
const MIN_RATED_FOR_LEADERS: u16 = 10;

impl Rank {
    const ALL: [Rank; 6] = [Rank::Goals, Rank::Assists, Rank::Appearances, Rank::CleanSheets, Rank::Rating, Rank::PlayerOfTheMatch];

    fn label(self) -> &'static str {
        match self {
            Rank::Goals => "Goals",
            Rank::Assists => "Assists",
            Rank::Appearances => "Appearances",
            Rank::CleanSheets => "Clean sheets",
            Rank::Rating => "Average rating (10+ matches)",
            Rank::PlayerOfTheMatch => "Player of the match",
        }
    }

    /// True for a rate-like stat where doing it in fewer minutes is better.
    fn fewer_minutes_wins(self) -> bool {
        matches!(self, Rank::Goals | Rank::Assists)
    }

    fn count(self, s: &PlayerStats) -> u16 {
        match self {
            Rank::Goals => s.goals,
            Rank::Assists => s.assists,
            Rank::Appearances => s.appearances,
            Rank::CleanSheets => s.clean_sheets,
            // Hundredths of a rating point, so the ranking keeps the second decimal; 0 drops him from the table.
            Rank::Rating if s.rated >= MIN_RATED_FOR_LEADERS => s.average_rating().map_or(0, |r| (r * 100.0).round() as u16),
            Rank::Rating => 0,
            Rank::PlayerOfTheMatch => s.player_of_the_match,
        }
    }
}

pub struct State {
    /// Start year of the season shown; `None` is all time.
    season: Option<u16>,
    /// `None` is every competition.
    competition: Option<CompetitionId>,
    rank: Rank,
}

impl Default for State {
    fn default() -> State {
        State { season: None, competition: None, rank: Rank::Goals }
    }
}

/// The players with the most of one stat, most first. Nobody on zero. Ties go to whoever needed fewer minutes when
/// `fewer_minutes_wins` (goals and assists: he was more efficient), otherwise to whoever played more (appearances,
/// clean sheets).
pub fn leaders(stats: &[(PlayerId, PlayerStats)], count: impl Fn(&PlayerStats) -> u16, fewer_minutes_wins: bool, limit: usize) -> Vec<(PlayerId, PlayerStats)> {
    let mut best: Vec<(PlayerId, PlayerStats)> = stats.iter().filter(|(_, s)| count(s) > 0).copied().collect();
    let minutes = |s: &PlayerStats| if fewer_minutes_wins { s.minutes as i32 } else { -(s.minutes as i32) };
    best.sort_by_key(|(id, s)| (std::cmp::Reverse(count(s)), minutes(s), *id));
    best.truncate(limit);
    best
}

pub fn ui(ui: &mut egui::Ui, world: &World, my_club: ClubId, state: &mut State) {
    let seasons: BTreeSet<u16> = world.stats.keys().map(|&(_, year, _)| year).collect();
    let competitions: BTreeSet<CompetitionId> = world.stats.keys().map(|&(_, _, competition)| competition).collect();
    ui.horizontal(|ui| {
        ui.label("Season:");
        let season_text = state.season.map_or("All time".to_string(), season_name);
        egui::ComboBox::from_id_salt("stats_season").selected_text(season_text).show_ui(ui, |ui| {
            ui.selectable_value(&mut state.season, None, "All time");
            for &year in seasons.iter().rev() {
                ui.selectable_value(&mut state.season, Some(year), season_name(year));
            }
        });
        ui.label("Competition:");
        let competition_text = state.competition.and_then(|c| world.competitions.get(&c)).map_or("All competitions".to_string(), league_label);
        egui::ComboBox::from_id_salt("stats_competition").width(260.0).selected_text(competition_text).show_ui(ui, |ui| {
            ui.selectable_value(&mut state.competition, None, "All competitions");
            for c in competitions.iter().filter_map(|c| world.competitions.get(c)) {
                ui.selectable_value(&mut state.competition, Some(c.id), league_label(c));
            }
        });
        ui.label("Ranked by:");
        egui::ComboBox::from_id_salt("stats_rank").selected_text(state.rank.label()).show_ui(ui, |ui| {
            for rank in Rank::ALL {
                ui.selectable_value(&mut state.rank, rank, rank.label());
            }
        });
    });
    ui.add_space(6.0);

    let rows = leaders(&world.stats_for(state.season, state.competition), |s| state.rank.count(s), state.rank.fewer_minutes_wins(), SHOWN);
    if rows.is_empty() {
        ui.label("Nothing to show yet: play some matches.");
        return;
    }
    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("stats_table").striped(true).min_col_width(40.0).show(ui, |ui| {
            for h in ["#", "Player", "Club", "Apps", "Goals", "Assists", "Clean sheets", "Yellow", "Red", "Rating", "MotM"] {
                ui.label(RichText::new(h).strong());
            }
            ui.end_row();
            for (place, (id, s)) in rows.iter().enumerate() {
                let player = &world.players[id];
                ui.label((place + 1).to_string());
                ui.label(&player.name);
                // Where he plays now, which is not always where he scored.
                let club = player.club.map(|c| &world.clubs[&c]);
                let name = club.map_or(String::new(), |c| c.name.clone());
                ui.label(if club.map(|c| c.id) == Some(my_club) { RichText::new(name).strong() } else { RichText::new(name) });
                for n in [s.appearances, s.goals, s.assists, s.clean_sheets, s.yellows, s.reds] {
                    ui.label(n.to_string());
                }
                ui.label(s.average_rating().map_or("-".to_string(), |r| format!("{r:.2}")));
                ui.label(s.player_of_the_match.to_string());
                ui.end_row();
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaders_are_most_first_with_fewer_minutes_winning_a_tie_and_nobody_on_zero() {
        let row = |id, goals, minutes| (id, PlayerStats { goals, minutes, ..PlayerStats::default() });
        let stats = [row(1, 3, 900), row(2, 5, 1200), row(3, 3, 800), row(4, 0, 90)];
        let ids = |limit| leaders(&stats, |s| s.goals, true, limit).iter().map(|r| r.0).collect::<Vec<_>>();
        assert_eq!(ids(10), vec![2, 3, 1]);
        assert_eq!(ids(2), vec![2, 3]);
        // Appearances and clean sheets: a tie goes to whoever played more.
        let more = leaders(&stats, |s| s.goals, false, 10).iter().map(|r| r.0).collect::<Vec<_>>();
        assert_eq!(more, vec![2, 1, 3]);
    }
}
