//! Builds history.db from the Kaggle CSVs. Usage: import [data_dir] [out.db]
use rusqlite::{params_from_iter, types::Value, Connection};
use std::{collections::HashMap, error::Error, fs, path::Path};

type Res<T> = Result<T, Box<dyn Error>>;

/// One table filled from a CSV; columns are read by header name and keep that name in SQL.
struct Spec {
    table: &'static str,
    ints: &'static [&'static str],
    texts: &'static [&'static str],
    pk: &'static str,
    verb: &'static str, // "INSERT" fails on duplicates, "INSERT OR IGNORE" keeps the first row
}

const EDITIONS: Spec = Spec { table: "editions", ints: &["fifa_version"], texts: &["update_as_of"], pk: "fifa_version", verb: "INSERT OR IGNORE" };
// shortcut: identity comes from the first row seen; names/dob rarely change, switch to the latest-edition row if that matters
const PLAYERS: Spec = Spec {
    table: "players",
    ints: &["player_id", "nationality_id"],
    texts: &["short_name", "long_name", "dob", "nationality_name", "preferred_foot"],
    pk: "player_id",
    verb: "INSERT OR IGNORE",
};
const RATINGS: Spec = Spec {
    table: "ratings",
    ints: &[
        "player_id", "fifa_version", "overall", "potential", "age", "value_eur", "wage_eur", "height_cm", "weight_kg",
        "club_team_id", "club_jersey_number", "club_contract_valid_until_year", "weak_foot", "skill_moves",
        "international_reputation", "release_clause_eur", "pace", "shooting", "passing", "dribbling", "defending", "physic",
        "attacking_crossing", "attacking_finishing", "attacking_heading_accuracy", "attacking_short_passing",
        "attacking_volleys", "skill_dribbling", "skill_curve", "skill_fk_accuracy", "skill_long_passing",
        "skill_ball_control", "movement_acceleration", "movement_sprint_speed", "movement_agility", "movement_reactions",
        "movement_balance", "power_shot_power", "power_jumping", "power_stamina", "power_strength", "power_long_shots",
        "mentality_aggression", "mentality_interceptions", "mentality_positioning", "mentality_vision",
        "mentality_penalties", "mentality_composure", "defending_marking_awareness", "defending_standing_tackle",
        "defending_sliding_tackle", "goalkeeping_diving", "goalkeeping_handling", "goalkeeping_kicking",
        "goalkeeping_positioning", "goalkeeping_reflexes", "goalkeeping_speed",
    ],
    texts: &["player_positions", "club_position", "work_rate", "body_type", "club_joined_date", "club_loaned_from", "player_tags", "player_traits"],
    pk: "player_id, fifa_version",
    verb: "INSERT",
};
const TEAMS: Spec = Spec { table: "teams", ints: &["team_id"], texts: &["team_name"], pk: "team_id", verb: "INSERT OR IGNORE" };
// nationality_name here is the country of the team's league, so "Premier League" can be told apart by country
const LEAGUES: Spec = Spec { table: "leagues", ints: &["league_id"], texts: &["league_name", "nationality_name"], pk: "league_id", verb: "INSERT OR IGNORE" };
const TEAM_EDITIONS: Spec = Spec {
    table: "team_editions",
    ints: &[
        "team_id", "fifa_version", "league_id", "league_level", "overall", "attack", "midfield", "defence", "coach_id",
        "international_prestige", "domestic_prestige", "transfer_budget_eur", "club_worth_eur", "def_team_width",
        "def_team_depth", "off_team_width",
    ],
    texts: &["home_stadium", "def_style", "off_style"],
    pk: "team_id, fifa_version",
    verb: "INSERT",
};
const COACHES: Spec = Spec {
    table: "coaches",
    ints: &["coach_id"],
    texts: &["short_name", "long_name", "dob", "nationality_name"],
    pk: "coach_id",
    verb: "INSERT",
};

/// CSV number to SQL integer; blank means NULL (never 0). Accepts "16.0" style floats.
fn int(s: &str) -> Value {
    match s.trim().parse::<f64>() {
        Ok(f) => Value::Integer(f as i64),
        Err(_) => Value::Null,
    }
}

fn text(s: &str) -> Value {
    if s.is_empty() { Value::Null } else { Value::Text(s.to_string()) }
}

fn create_sql(s: &Spec) -> String {
    let cols: Vec<String> = s.ints.iter().map(|c| format!("\"{c}\" INTEGER"))
        .chain(s.texts.iter().map(|c| format!("\"{c}\" TEXT"))).collect();
    format!("CREATE TABLE {} ({}, PRIMARY KEY ({}))", s.table, cols.join(", "), s.pk)
}

/// Reads one CSV once and fills every spec from it. Returns rows read.
fn load(conn: &Connection, path: &Path, specs: &[&Spec]) -> Res<usize> {
    let mut rdr = csv::Reader::from_path(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let idx: HashMap<String, usize> = rdr.headers()?.iter().enumerate().map(|(i, h)| (h.to_string(), i)).collect();
    let col = |c: &str| idx.get(c).copied().ok_or_else(|| format!("{}: missing column {c}", path.display()));
    let mut plans = Vec::new();
    for s in specs {
        let ints = s.ints.iter().map(|c| col(c)).collect::<Result<Vec<_>, _>>()?;
        let texts = s.texts.iter().map(|c| col(c)).collect::<Result<Vec<_>, _>>()?;
        let marks = vec!["?"; ints.len() + texts.len()].join(",");
        plans.push((s, ints, texts, format!("{} INTO {} VALUES ({marks})", s.verb, s.table)));
    }
    let mut n = 0;
    for rec in rdr.records() {
        let rec = rec?;
        for (s, ints, texts, sql) in &plans {
            let vals: Vec<Value> = ints.iter().map(|&i| int(&rec[i])).chain(texts.iter().map(|&i| text(&rec[i]))).collect();
            conn.prepare_cached(sql)?.execute(params_from_iter(vals))
                .map_err(|e| format!("{} row {}: {e}", s.table, n + 2))?;
        }
        n += 1;
    }
    Ok(n)
}

fn count(conn: &Connection, sql: &str) -> Res<i64> {
    Ok(conn.query_row(sql, [], |r| r.get(0))?)
}

fn main() -> Res<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = Path::new(args.first().map(String::as_str).unwrap_or("research/information/data"));
    let out = args.get(1).map(String::as_str).unwrap_or("history.db");
    let _ = fs::remove_file(out); // rebuild from scratch; the CSVs are the source of truth
    let conn = Connection::open(out)?;
    conn.execute_batch("BEGIN")?;
    for s in [&EDITIONS, &PLAYERS, &RATINGS, &TEAMS, &LEAGUES, &TEAM_EDITIONS, &COACHES] {
        conn.execute(&create_sql(s), [])?;
    }
    let player_rows = load(&conn, &dir.join("male_players.csv"), &[&EDITIONS, &PLAYERS, &RATINGS])?;
    let team_rows = load(&conn, &dir.join("male_teams.csv"), &[&TEAMS, &LEAGUES, &TEAM_EDITIONS])?;
    let coach_rows = load(&conn, &dir.join("male_coaches.csv"), &[&COACHES])?;
    conn.execute_batch(
        "CREATE INDEX ratings_club ON ratings(fifa_version, club_team_id);
         CREATE INDEX team_editions_league ON team_editions(fifa_version, league_id);
         COMMIT",
    )?;

    // Checks: every CSV row landed, and ids point at rows that exist.
    for (table, csv_rows) in [("ratings", player_rows), ("team_editions", team_rows), ("coaches", coach_rows)] {
        let db = count(&conn, &format!("SELECT COUNT(*) FROM {table}"))?;
        if db as usize != csv_rows {
            return Err(format!("{table}: {csv_rows} CSV rows but {db} in db").into());
        }
    }
    let orphan_clubs = count(&conn, "SELECT COUNT(*) FROM ratings r WHERE club_team_id IS NOT NULL AND NOT EXISTS
        (SELECT 1 FROM team_editions t WHERE t.team_id = r.club_team_id AND t.fifa_version = r.fifa_version)")?;
    let free_agents = count(&conn, "SELECT COUNT(*) FROM ratings WHERE club_team_id IS NULL")?;
    let orphan_coaches = count(&conn, "SELECT COUNT(*) FROM team_editions t WHERE coach_id IS NOT NULL
        AND NOT EXISTS (SELECT 1 FROM coaches c WHERE c.coach_id = t.coach_id)")?;

    println!("edition  players  clubs  leagues");
    let mut st = conn.prepare("SELECT fifa_version, COUNT(*), COUNT(DISTINCT club_team_id),
        (SELECT COUNT(DISTINCT league_id) FROM team_editions t WHERE t.fifa_version = r.fifa_version)
        FROM ratings r GROUP BY fifa_version ORDER BY fifa_version")?;
    for row in st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?, r.get::<_, i64>(3)?)))? {
        let (v, p, c, l) = row?;
        println!("{v:>7}  {p:>7}  {c:>5}  {l:>7}");
    }
    println!("\nplayers {}  ratings {player_rows}  teams {}  coaches {coach_rows}",
        count(&conn, "SELECT COUNT(*) FROM players")?, count(&conn, "SELECT COUNT(*) FROM teams")?);
    println!("warnings: {orphan_clubs} ratings with a club missing from that edition's teams, {free_agents} with no club, {orphan_coaches} team rows with an unknown coach");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_is_null_and_floats_truncate() {
        assert_eq!(int(""), Value::Null);
        assert_eq!(int("NA"), Value::Null);
        assert_eq!(int("16.0"), Value::Integer(16));
        assert_eq!(int("181500000.0"), Value::Integer(181_500_000));
        assert_eq!(text(""), Value::Null);
    }
}
