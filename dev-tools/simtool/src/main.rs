//! simtool: simulates many league seasons and prints how the results are spread, so the match simulation can be
//! checked and tuned against real football without opening the game.
//!
//! Usage: simtool [--edition 15] [--seasons 20] [--seed 1] [--db history.db]
//! Every season starts from the same edition and plays every league (no cups, no promotion), with a different seed.
use engine::{load_edition, team_strength, ClubId, Competition, Connection, Rng, World};
use std::time::Instant;

/// Leagues printed one by one, as (name, country). All leagues are printed together as well.
const BIG_LEAGUES: [(&str, &str); 5] = [
    ("Premier League", "England"),
    ("Bundesliga", "Germany"),
    ("La Liga", "Spain"),
    ("Serie A", "Italy"),
    ("Ligue 1", "France"),
];

struct Options {
    db: String,
    edition: i64,
    seasons: u64,
    seed: u64,
}

impl Options {
    /// Reads `--name value` pairs. Anything unknown or missing a value is an error message.
    fn parse(args: &[String]) -> Result<Options, String> {
        let mut options = Options { db: "history.db".into(), edition: 15, seasons: 20, seed: 1 };
        let mut args = args.iter();
        while let Some(name) = args.next() {
            let value = args.next().ok_or(format!("{name} needs a value"))?;
            let number = |what: &str| value.parse::<u64>().map_err(|_| format!("{what} must be a number, got {value}"));
            match name.as_str() {
                "--db" => options.db = value.clone(),
                "--edition" => options.edition = number("--edition")? as i64,
                "--seasons" => options.seasons = number("--seasons")?,
                "--seed" => options.seed = number("--seed")?,
                _ => return Err(format!("unknown option {name}")),
            }
        }
        Ok(options)
    }
}

/// What one league's season looked like.
#[derive(Default)]
struct LeagueSeason {
    matches: u32,
    goals: u32,
    home_wins: u32,
    draws: u32,
    away_wins: u32,
    /// Points per match of the champion, the runner-up, the 4th club and the last club.
    champion: f64,
    runner_up: f64,
    fourth: f64,
    last: f64,
    /// 1 = the champion was the strongest club of the league on paper.
    champion_strength_rank: usize,
}

/// Adds up league seasons, for one league or for all of them.
#[derive(Default)]
struct Totals {
    seasons: u32,
    /// Clubs of every league season added, to show an average.
    club_count: usize,
    season: LeagueSeason,
    strongest_won: u32,
    top_three_won: u32,
}

impl Totals {
    fn add(&mut self, clubs: usize, s: &LeagueSeason) {
        self.seasons += 1;
        self.club_count += clubs;
        self.season.matches += s.matches;
        self.season.goals += s.goals;
        self.season.home_wins += s.home_wins;
        self.season.draws += s.draws;
        self.season.away_wins += s.away_wins;
        self.season.champion += s.champion;
        self.season.runner_up += s.runner_up;
        self.season.fourth += s.fourth;
        self.season.last += s.last;
        self.strongest_won += (s.champion_strength_rank == 1) as u32;
        self.top_three_won += (s.champion_strength_rank <= 3) as u32;
    }

    fn line(&self, label: &str) -> String {
        let (s, n) = (&self.season, self.seasons as f64);
        let matches = s.matches.max(1) as f64;
        let share = |count: u32| 100.0 * count as f64 / matches;
        format!(
            "{label:<26} {:>3.0} clubs {:>5.2} goals {:>5.1}% home {:>5.1}% draw {:>5.1}% away | points/match: 1st {:>4.2} 2nd {:>4.2} 4th {:>4.2} last {:>4.2} | strongest wins {:>3.0}% top-3 wins {:>3.0}%",
            (self.club_count as f64 / n).round(),
            s.goals as f64 / matches,
            share(s.home_wins),
            share(s.draws),
            share(s.away_wins),
            s.champion / n,
            s.runner_up / n,
            s.fourth / n,
            s.last / n,
            100.0 * self.strongest_won as f64 / n,
            100.0 * self.top_three_won as f64 / n,
        )
    }
}

/// Measures a played league: results, and how the champion ranks by strength on paper (1 = strongest).
fn measure(world: &World, league: &Competition) -> LeagueSeason {
    let mut season = LeagueSeason::default();
    for f in world.fixtures.iter().filter(|f| f.competition == league.id) {
        let Some(outcome) = f.result else { continue };
        let (home, away) = outcome.goals();
        season.matches += 1;
        season.goals += home as u32 + away as u32;
        match home.cmp(&away) {
            std::cmp::Ordering::Greater => season.home_wins += 1,
            std::cmp::Ordering::Equal => season.draws += 1,
            std::cmp::Ordering::Less => season.away_wins += 1,
        }
    }
    let table = world.table(league.id);
    let per_match = |place: usize| table[place].points as f64 / table[place].played.max(1) as f64;
    season.champion = per_match(0);
    season.runner_up = per_match(1);
    season.fourth = per_match(3);
    season.last = per_match(table.len() - 1);
    let champion = &table[0];
    let strength = |club: ClubId| team_strength(&world.lineup_ratings(club));
    let champion_strength = strength(champion.club);
    season.champion_strength_rank = 1 + league.clubs.iter().filter(|&&c| strength(c) > champion_strength).count();
    season
}

/// Plays `seasons` seasons of every league and returns the totals: all leagues together, the big five together,
/// then each of the big five.
fn run(world: &mut World, options: &Options) -> Vec<(String, Totals)> {
    let (mut all, mut big_five) = (Totals::default(), Totals::default());
    let mut big: Vec<(String, Totals)> = BIG_LEAGUES.iter().map(|(name, country)| (format!("{name} ({country})"), Totals::default())).collect();
    for season in 0..options.seasons {
        let mut rng = Rng::new(options.seed + season);
        world.generate_fixtures(); // starts every league again, results cleared
        world.play_season_to_end(&mut rng);
        for league in world.competitions.values().filter(|c| c.is_league() && c.clubs.len() >= 8) {
            let result = measure(world, league);
            all.add(league.clubs.len(), &result);
            let name = (league.name.as_str(), league.country.as_str());
            if let Some(i) = BIG_LEAGUES.iter().position(|&l| l == name) {
                big[i].1.add(league.clubs.len(), &result);
                big_five.add(league.clubs.len(), &result);
            }
        }
    }
    let mut totals = vec![("All leagues of 8+ clubs".to_string(), all), ("The big five together".to_string(), big_five)];
    totals.extend(big.into_iter().filter(|(_, t)| t.seasons > 0));
    totals
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let options = Options::parse(&args).unwrap_or_else(|message| {
        eprintln!("{message}\nUsage: simtool [--edition 15] [--seasons 20] [--seed 1] [--db history.db]");
        std::process::exit(2);
    });
    let conn = Connection::open(&options.db).unwrap_or_else(|e| {
        eprintln!("cannot open {}: {e}. Build it first: cargo run -p import --release", options.db);
        std::process::exit(1);
    });
    let mut world = load_edition(&conn, options.edition).unwrap_or_else(|e| {
        eprintln!("cannot load edition {}: {e}", options.edition);
        std::process::exit(1);
    });

    let start = Instant::now();
    let totals = run(&mut world, &options);
    let elapsed = start.elapsed();
    println!("Edition {}, {} seasons from seed {}.\n", options.edition, options.seasons, options.seed);
    for (label, t) in &totals {
        println!("{}", t.line(label));
    }
    println!("\n{:.2} s per season ({} seasons in {:.1} s)", elapsed.as_secs_f64() / options.seasons.max(1) as f64, options.seasons, elapsed.as_secs_f64());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn options_have_defaults_and_read_named_values() {
        let o = Options::parse(&[]).unwrap();
        assert_eq!((o.edition, o.seasons, o.seed, o.db.as_str()), (15, 20, 1, "history.db"));
        let o = Options::parse(&strings(&["--edition", "24", "--seasons", "5", "--seed", "9", "--db", "x.db"])).unwrap();
        assert_eq!((o.edition, o.seasons, o.seed, o.db.as_str()), (24, 5, 9, "x.db"));
    }

    #[test]
    fn bad_options_are_errors_not_panics() {
        assert!(Options::parse(&strings(&["--seasons"])).is_err());
        assert!(Options::parse(&strings(&["--seasons", "many"])).is_err());
        assert!(Options::parse(&strings(&["--colour", "red"])).is_err());
    }

    #[test]
    fn a_run_reports_shares_that_add_up_and_is_repeatable() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../history.db");
        if !std::path::Path::new(path).exists() { return; } // needs `cargo run -p import` first
        let mut world = load_edition(&Connection::open(path).unwrap(), 15).unwrap();
        let options = Options { db: path.into(), edition: 15, seasons: 1, seed: 3 };
        let first = run(&mut world, &options);
        let second = run(&mut world, &options);
        let (all, again) = (&first[0].1, &second[0].1);
        let s = &all.season;
        assert!(s.matches > 5000);
        assert_eq!(s.matches, s.home_wins + s.draws + s.away_wins);
        let goals_per_match = s.goals as f64 / s.matches as f64;
        assert!((1.5..4.5).contains(&goals_per_match), "{goals_per_match} goals a match");
        assert_eq!((s.goals, s.home_wins), (again.season.goals, again.season.home_wins), "the same seed gives the same run");
    }
}
