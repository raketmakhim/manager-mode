//! Building a `World` from history.db: "start a career in FIFA 15".
// shortcut: one plain function, not a `DataSource` trait. Make it a trait when a second source
// (e.g. generated fictional leagues) exists to put behind it.
use crate::attrs::{Attr, Attrs};
use crate::competition::{Competition, Rules};
use crate::model::*;
use rusqlite::{Connection, Result};

/// The editions in the database, oldest first (15 = FIFA 15 ... 24 = FC 24).
pub fn editions(conn: &Connection) -> Result<Vec<i64>> {
    conn.prepare("SELECT fifa_version FROM editions ORDER BY fifa_version")?.query_map([], |r| r.get(0))?.collect()
}

/// Loads every club and player of one edition (15 = FIFA 15 ... 24 = FC 24).
/// National teams ("Friendly International") are skipped: they have no club players.
/// Errors if the edition is not in the database.
pub fn load_edition(conn: &Connection, version: i64) -> Result<World> {
    conn.query_row("SELECT 1 FROM editions WHERE fifa_version = ?1", [version], |_| Ok(()))?;

    // FIFA 15 shipped in autumn 2014: the 2014/15 season
    let mut world = World::new(version, (1999 + version) as u16);

    let mut st = conn.prepare(
        "SELECT t.team_id AS id, t.team_name AS name, te.league_id AS league_id, l.league_name AS league_name,
                l.nationality_name AS country, te.transfer_budget_eur AS budget
         FROM team_editions te
         JOIN teams t USING (team_id)
         JOIN leagues l USING (league_id)
         WHERE te.fifa_version = ?1
           AND EXISTS (SELECT 1 FROM ratings r WHERE r.fifa_version = te.fifa_version AND r.club_team_id = te.team_id)
         ORDER BY t.team_id",
    )?;
    let clubs = st.query_map([version], |r| {
        Ok((
            Club {
                id: r.get("id")?,
                name: r.get("name")?,
                league: r.get("league_id")?,
                squad: vec![],
                budget: r.get("budget")?,
                lineup: vec![],
            },
            r.get::<_, String>("league_name")?,
            r.get::<_, String>("country")?,
        ))
    })?;
    for club in clubs {
        let (club, league_name, country) = club?;
        let league = world.competitions.entry(club.league).or_insert_with(|| Competition {
            id: club.league,
            name: league_name,
            country,
            rules: Rules::double_round_robin(),
            clubs: vec![],
        });
        league.clubs.push(club.id);
        world.clubs.insert(club.id, club);
    }

    let sql = format!(
        "SELECT r.player_id AS id, p.long_name AS name, p.dob AS dob, r.player_positions AS positions,
                r.overall AS overall, r.potential AS potential, r.club_team_id AS club,
                r.wage_eur AS wage, r.value_eur AS value, r.club_contract_valid_until_year AS contract_until,
                {}
         FROM ratings r JOIN players p USING (player_id)
         WHERE r.fifa_version = ?1
         ORDER BY r.overall DESC, r.player_id",
        Attr::select_list()
    );
    let mut st = conn.prepare(&sql)?;
    let players = st.query_map([version], |r| {
        let positions: Option<String> = r.get("positions")?;
        Ok(Player {
            id: r.get("id")?,
            name: r.get("name")?,
            birth_date: r.get("dob")?,
            positions: positions.unwrap_or_default().split(',').filter_map(|p| Position::parse(p.trim())).collect(),
            attrs: Attrs::from_row(r)?,
            overall: r.get::<_, Option<u8>>("overall")?.unwrap_or(0),
            potential: r.get::<_, Option<u8>>("potential")?.unwrap_or(0),
            club: r.get("club")?,
            wage: r.get::<_, Option<u32>>("wage")?.unwrap_or(0),
            contract_until: r.get("contract_until")?,
            value: r.get::<_, Option<u32>>("value")?.unwrap_or(0),
            form: NEUTRAL_FORM,
            fitness: FULL_FITNESS,
        })
    })?;
    for player in players {
        let player = player?;
        if let Some(club) = player.club.and_then(|c| world.clubs.get_mut(&c)) {
            club.squad.push(player.id); // rows come best first, so squads are too
        }
        world.players.insert(player.id, player);
    }
    Ok(world)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attrs::Attr::*;

    fn db() -> Option<Connection> {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../history.db");
        std::path::Path::new(path).exists().then(|| Connection::open(path).unwrap())
    }

    #[test]
    fn fifa_15_loads_with_consistent_links() {
        let Some(conn) = db() else { return }; // needs `cargo run -p import` first
        let w = load_edition(&conn, 15).unwrap();
        assert_eq!((w.edition, w.year), (15, 2014));
        assert_eq!(w.players.len(), 16_182); // same as the import summary
        assert_eq!(w.clubs.len(), 578); // 625 teams minus 47 national teams

        // Every link points at something that exists, and both directions agree.
        let mut in_squads = 0;
        for c in w.clubs.values() {
            assert!(w.competitions[&c.league].clubs.contains(&c.id));
            assert!(!c.squad.is_empty(), "{} has no players", c.name);
            for p in &c.squad {
                assert_eq!(w.players[p].club, Some(c.id));
            }
            in_squads += c.squad.len();
        }
        assert_eq!(in_squads, w.players.values().filter(|p| p.club.is_some()).count());
        assert!(w.players.values().all(|p| !p.positions.is_empty()), "a position failed to parse");
        assert!(w.competitions.values().all(|l| l.name != "Friendly International"));
    }

    #[test]
    fn rooney_in_fifa_15() {
        let Some(conn) = db() else { return };
        let w = load_edition(&conn, 15).unwrap();
        let r = &w.players[&54050];
        assert_eq!(w.clubs[&r.club.unwrap()].name, "Manchester United");
        assert_eq!((r.overall, r.potential, r.wage), (86, 86, 230_000));
        assert_eq!(r.positions[0], Position::St);
        assert_eq!(r.attrs[Finishing], Some(86));
        assert_eq!((r.form, r.fitness), (NEUTRAL_FORM, FULL_FITNESS));
    }

    #[test]
    fn leagues_have_a_country_and_editions_are_listed() {
        let Some(conn) = db() else { return };
        assert_eq!(editions(&conn).unwrap(), (15..=24).collect::<Vec<i64>>());
        let w = load_edition(&conn, 15).unwrap();
        let pl = w.competitions.values().find(|l| l.name == "Premier League" && l.country == "England").unwrap();
        assert_eq!(pl.clubs.len(), 20);
        // The name alone is ambiguous, which is why the country is kept.
        assert!(w.competitions.values().filter(|l| l.name == "Premier League").count() > 1);
    }

    #[test]
    fn unknown_edition_is_an_error() {
        let Some(conn) = db() else { return };
        assert!(load_edition(&conn, 99).is_err());
    }
}
