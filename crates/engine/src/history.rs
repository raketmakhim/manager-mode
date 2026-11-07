//! Reading real player history out of history.db: find a player, then see them edition by edition.
use crate::attrs::{Attr, Attrs};
use rusqlite::{params, Connection, Result};

/// One search result: a player and the span of editions they appear in.
pub struct Hit {
    pub player_id: i64,
    /// Full name, e.g. "Wayne Mark Rooney".
    pub name: String,
    /// First and last edition (FIFA version number, 15 = FIFA 15) the player is in.
    pub first: i64,
    pub last: i64,
    /// Highest overall rating (0-99) across those editions.
    pub best: i64,
}

/// One player in one edition. Money is in euros.
pub struct Snapshot {
    /// FIFA version number: 15 = FIFA 15 ... 24 = FC 24.
    pub version: i64,
    /// Age in years at that edition.
    pub age: Option<i64>,
    /// Overall and potential ratings, 0-99.
    pub overall: Option<i64>,
    pub potential: Option<i64>,
    /// Club name; `None` for a free agent.
    pub club: Option<String>,
    /// Transfer value, in euros.
    pub value: Option<i64>,
    /// Wage, in euros per week.
    pub wage: Option<i64>,
    /// Comma separated, main position first, e.g. "CAM, CM, ST".
    pub positions: String,
    pub attrs: Attrs,
}

/// "FIFA 20" for version 20, "FC 24" for version 24.
pub fn edition_label(version: i64) -> String {
    if version >= 24 { format!("FC {version}") } else { format!("FIFA {version}") }
}

/// "2014/15" for a season starting in 2014.
pub fn season_name(start_year: u16) -> String {
    format!("{start_year}/{:02}", (start_year + 1) % 100)
}

/// "2014/15" for version 15 (FIFA 15 shipped in autumn 2014).
pub fn season_label(version: i64) -> String {
    season_name((1999 + version) as u16)
}

/// Live name search, best players first. shortcut: LIKE is accent-sensitive ("Mbappe" will not find "Mbappé"), fix with an unaccented name column if it annoys.
pub fn search(conn: &Connection, text: &str, limit: usize) -> Result<Vec<Hit>> {
    let like = format!("%{}%", text.trim());
    let mut st = conn.prepare_cached(
        "SELECT p.player_id AS player_id, p.long_name AS name,
                MIN(r.fifa_version) AS first, MAX(r.fifa_version) AS last, MAX(r.overall) AS best
         FROM players p JOIN ratings r USING (player_id)
         WHERE p.long_name LIKE ?1 OR p.short_name LIKE ?1
         GROUP BY p.player_id ORDER BY MAX(r.overall) DESC LIMIT ?2",
    )?;
    let rows = st.query_map(params![like, limit as i64], |r| {
        Ok(Hit {
            player_id: r.get("player_id")?,
            name: r.get("name")?,
            first: r.get("first")?,
            last: r.get("last")?,
            best: r.get("best")?,
        })
    })?;
    rows.collect()
}

/// Full name of a player.
pub fn player_name(conn: &Connection, player_id: i64) -> Result<String> {
    conn.query_row("SELECT long_name FROM players WHERE player_id = ?1", [player_id], |r| r.get(0))
}

/// Every edition the player appears in, oldest first. The later rows are what "actually happened".
pub fn career(conn: &Connection, player_id: i64) -> Result<Vec<Snapshot>> {
    let sql = format!(
        "SELECT r.fifa_version AS version, r.age AS age, r.overall AS overall, r.potential AS potential,
                t.team_name AS club, r.value_eur AS value, r.wage_eur AS wage,
                COALESCE(r.player_positions, '') AS positions, {}
         FROM ratings r LEFT JOIN teams t ON t.team_id = r.club_team_id
         WHERE r.player_id = ?1 ORDER BY r.fifa_version",
        Attr::select_list()
    );
    let mut st = conn.prepare(&sql)?;
    let rows = st.query_map([player_id], |r| {
        Ok(Snapshot {
            version: r.get("version")?,
            age: r.get("age")?,
            overall: r.get("overall")?,
            potential: r.get("potential")?,
            club: r.get("club")?,
            value: r.get("value")?,
            wage: r.get("wage")?,
            positions: r.get("positions")?,
            attrs: Attrs::from_row(r)?,
        })
    })?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use Attr::*;

    fn db() -> Option<Connection> {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../history.db");
        std::path::Path::new(path).exists().then(|| Connection::open(path).unwrap())
    }

    #[test]
    fn rooney_career_is_read_in_order() {
        let Some(conn) = db() else { return }; // needs `cargo run -p import` first
        let c = career(&conn, 54050).unwrap();
        assert_eq!(c.len(), 7);
        assert_eq!((c[0].version, c[0].overall), (15, Some(86)));
        assert_eq!(c[6].club.as_deref(), Some("Derby County"));
        assert_eq!((c[0].age, c[0].wage, c[0].positions.as_str()), (Some(28), Some(230_000), "ST, CF, CAM"));
        assert_eq!(c[6].positions, "CAM, CM, ST");
        // Named access returns the right column: values seen on the scout screen.
        assert_eq!(c[0].attrs[Finishing], Some(86));
        assert_eq!(c[0].attrs[Acceleration], Some(74));
        assert_eq!(c[6].attrs[Acceleration], Some(54));
        assert_eq!(c[0].attrs[Composure], None); // FIFA 15 has no composure
    }

    #[test]
    fn search_finds_best_first() {
        let Some(conn) = db() else { return };
        let hits = search(&conn, "rooney", 5).unwrap();
        assert_eq!(hits[0].player_id, 54050);
        assert_eq!((hits[0].first, hits[0].last), (15, 21));
        assert!(hits.windows(2).all(|w| w[0].best >= w[1].best));
    }
}
