//! Helpers for listing and naming leagues in drop-downs.
use engine::{Competition, World};

/// Leagues that have fixtures (two or more clubs), A to Z by name then country.
pub fn sorted_leagues(world: &World) -> Vec<&Competition> {
    let mut v: Vec<&Competition> =
        world.competitions.values().filter(|c| c.is_league() && c.clubs.len() >= 2).collect();
    v.sort_by(|a, b| (&a.name, &a.country).cmp(&(&b.name, &b.country)));
    v
}

/// "Premier League (England)": several countries have a league with the same name.
pub fn league_label(l: &Competition) -> String {
    format!("{} ({})", l.name, l.country)
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::Rules;

    fn competition(id: u32, name: &str, country: &str, rules: Rules, clubs: usize) -> Competition {
        Competition { id, name: name.into(), country: country.into(), rules, level: None, clubs: (0..clubs as u32).collect() }
    }

    #[test]
    fn leagues_are_sorted_a_to_z_and_skip_cups_and_one_club_leagues() {
        let mut world = World::new(0, 2014);
        let league = |id, name, country, clubs| competition(id, name, country, Rules::double_round_robin(), clubs);
        world.competitions.insert(1, league(1, "Serie A", "Italy", 20));
        world.competitions.insert(2, league(2, "Premiership", "Scotland", 12));
        world.competitions.insert(3, league(3, "Premiership", "England", 20));
        world.competitions.insert(4, league(4, "Lonely League", "Malta", 1));
        world.competitions.insert(5, competition(5, "Italy Cup", "Italy", Rules::default_cup(), 20));
        let names: Vec<String> = sorted_leagues(&world).into_iter().map(league_label).collect();
        assert_eq!(names, ["Premiership (England)", "Premiership (Scotland)", "Serie A (Italy)"]);
    }
}
