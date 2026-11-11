//! The honours section of a player's page: the titles and awards he has won, grouped by kind.
use eframe::egui::{self, RichText};
use engine::{season_name, Honour, HonourKind, Line, PlayerId, World};

/// The name of an honour and its place in the list: titles first, then the biggest awards.
fn kind_label(kind: HonourKind) -> (u8, &'static str) {
    match kind {
        HonourKind::Title => (0, "Titles won"),
        HonourKind::BallonDor => (1, "Ballon d'Or"),
        HonourKind::PlayerOfTheSeason => (2, "Player of the season"),
        HonourKind::YoungPlayer => (3, "Best young player"),
        HonourKind::BestOfLine(Line::Goalkeeper) => (4, "Best goalkeeper"),
        HonourKind::BestOfLine(Line::Defence) => (5, "Best defender"),
        HonourKind::BestOfLine(Line::Midfield) => (6, "Best midfielder"),
        HonourKind::BestOfLine(Line::Attack) => (7, "Best attacker"),
        HonourKind::BestEleven => (8, "Best eleven"),
        HonourKind::TopScorer => (9, "Top scorer"),
        HonourKind::TopAssists => (10, "Assist leader"),
    }
}

/// One line per kind of honour the player has: its name and where and when each one was won ("Premier League
/// (England) 2014/15"), in the order of `kind_label`.
fn summary(honours: &[&Honour]) -> Vec<(&'static str, Vec<String>)> {
    let mut lines: Vec<(u8, &'static str, Vec<String>)> = vec![];
    for h in honours {
        let (rank, label) = kind_label(h.kind);
        let season = season_name(h.season);
        let detail = if h.competition.is_some() { format!("{} {season}", h.competition_name) } else { season };
        match lines.iter_mut().find(|(r, _, _)| *r == rank) {
            Some((_, _, details)) => details.push(detail),
            None => lines.push((rank, label, vec![detail])),
        }
    }
    lines.sort_by_key(|(rank, _, _)| *rank);
    lines.into_iter().map(|(_, label, details)| (label, details)).collect()
}

/// Lists the player's honours, each kind with how many times and, opened, where and when.
pub fn ui(ui: &mut egui::Ui, world: &World, player: PlayerId) {
    let honours = world.honours_of(player);
    ui.label(RichText::new("Honours").strong());
    if honours.is_empty() {
        ui.label("None yet. Titles are given as competitions are decided and awards when a season ends.");
        return;
    }
    for (label, details) in summary(&honours) {
        ui.collapsing(format!("{label} x{}", details.len()), |ui| {
            for detail in details {
                ui.label(detail);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn honour(season: u16, competition: Option<u32>, kind: HonourKind) -> Honour {
        Honour { season, competition, competition_name: "Premier League (England)".into(), kind, player: 1 }
    }

    #[test]
    fn honours_are_grouped_by_kind_counted_and_listed_titles_first() {
        let all = [honour(2015, Some(1), HonourKind::BestEleven), honour(2014, Some(1), HonourKind::Title), honour(2016, Some(1), HonourKind::BestEleven), honour(2016, None, HonourKind::BallonDor), honour(2016, Some(1), HonourKind::Title)];
        let refs: Vec<&Honour> = all.iter().collect();
        let lines = summary(&refs);
        let labels: Vec<&str> = lines.iter().map(|(label, _)| *label).collect();
        assert_eq!(labels, ["Titles won", "Ballon d'Or", "Best eleven"]);
        assert_eq!(lines.iter().map(|(_, details)| details.len()).collect::<Vec<_>>(), [2, 1, 2]);
        assert_eq!(lines[0].1[0], "Premier League (England) 2014/15");
        assert_eq!(lines[1].1[0], "2016/17", "the Ballon d'Or covers every competition, so it names only the season");
    }

    #[test]
    fn no_honours_make_no_lines() {
        assert!(summary(&[]).is_empty());
    }
}
