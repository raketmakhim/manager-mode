//! The stats section of a player's page: one line per season over every competition, and the career total underneath.
use eframe::egui::{self, RichText};
use engine::{season_name, PlayerId, PlayerStats, World};

const HEADINGS: [&str; 10] = ["Season", "Apps", "Minutes", "Goals", "Assists", "Clean sheets", "Yellow", "Red", "Rating", "MotM"];

/// One table line: the label and the player's numbers. A rating needs rated matches, so it can be a dash.
fn cells(label: String, s: &PlayerStats) -> [String; 10] {
    [
        label,
        s.appearances.to_string(),
        s.minutes.to_string(),
        s.goals.to_string(),
        s.assists.to_string(),
        s.clean_sheets.to_string(),
        s.yellows.to_string(),
        s.reds.to_string(),
        s.average_rating().map_or("-".to_string(), |r| format!("{r:.2}")),
        s.player_of_the_match.to_string(),
    ]
}

/// The table's lines: a season each, oldest first, then the total of all of them.
fn lines(seasons: &[(u16, PlayerStats)]) -> Vec<[String; 10]> {
    let mut total = PlayerStats::default();
    let mut lines: Vec<[String; 10]> = seasons
        .iter()
        .map(|(year, s)| {
            total.add(s);
            cells(season_name(*year), s)
        })
        .collect();
    lines.push(cells("Total".to_string(), &total));
    lines
}

/// Shows the player's stats by season with the total as the last line.
pub fn ui(ui: &mut egui::Ui, world: &World, player: PlayerId) {
    ui.label(RichText::new("Stats").strong());
    let seasons = world.season_totals(player);
    if seasons.is_empty() {
        ui.label("No matches played yet.");
        return;
    }
    let lines = lines(&seasons);
    egui::Grid::new(("player_stats", player)).striped(true).min_col_width(48.0).show(ui, |ui| {
        for heading in HEADINGS {
            ui.label(RichText::new(heading).strong());
        }
        ui.end_row();
        let last = lines.len() - 1;
        for (i, line) in lines.into_iter().enumerate() {
            for text in line {
                ui.label(if i == last { RichText::new(text).strong() } else { RichText::new(text) });
            }
            ui.end_row();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn season(goals: u16, appearances: u16, rating_tenths: u32, rated: u16) -> PlayerStats {
        PlayerStats { goals, appearances, rating_tenths, rated, ..PlayerStats::default() }
    }

    #[test]
    fn the_table_has_a_line_a_season_and_a_total_that_adds_them_up() {
        let lines = lines(&[(2014, season(7, 41, 2400, 40)), (2015, season(3, 20, 1500, 20))]); // rated 6.0 and 7.5
        assert_eq!(lines.len(), 3);
        assert_eq!((lines[0][0].as_str(), lines[1][0].as_str(), lines[2][0].as_str()), ("2014/15", "2015/16", "Total"));
        assert_eq!((lines[2][1].as_str(), lines[2][3].as_str()), ("61", "10"), "appearances and goals add up");
        assert_eq!(lines[2][8], "6.50", "the total rating is over all rated matches (3900 tenths in 60), not the mean of 6.0 and 7.5");
    }

    #[test]
    fn a_player_with_no_rated_matches_has_a_dash_for_rating() {
        assert_eq!(lines(&[(2014, season(0, 1, 0, 0))])[0][8], "-");
    }

    #[test]
    fn xp_is_not_in_the_table() {
        assert!(!HEADINGS.contains(&"XP"));
    }
}
