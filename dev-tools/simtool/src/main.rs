//! simtool: simulates many league seasons and prints how the results are spread, so the match simulation can be
//! checked and tuned against real football without opening the game.
//!
//! Usage: simtool [--edition 15] [--seasons 20] [--seed 1] [--model cheap|shot-based] [--db history.db]
//! Every season starts from the same edition and plays every league (no cups, no promotion) with the chosen match
//! model, with a different seed. With the shot-based model it also counts the events (shots, cards, corners...).
use engine::{load_edition, simulate, team_strength, ClubId, Competition, Connection, Event, MatchModel, Rng, World, TOP_FIVE_LEAGUES};
use std::ops::Range;
use std::time::Instant;

struct Options {
    db: String,
    edition: i64,
    seasons: u64,
    seed: u64,
    /// Plays every league with this model; `None` leaves each league to its own rules.
    model: Option<MatchModel>,
}

impl Options {
    /// Reads `--name value` pairs. Anything unknown or missing a value is an error message.
    fn parse(args: &[String]) -> Result<Options, String> {
        let mut options = Options { db: "history.db".into(), edition: 15, seasons: 20, seed: 1, model: None };
        let mut args = args.iter();
        while let Some(name) = args.next() {
            let value = args.next().ok_or(format!("{name} needs a value"))?;
            let number = |what: &str| value.parse::<u64>().map_err(|_| format!("{what} must be a number, got {value}"));
            match name.as_str() {
                "--db" => options.db = value.clone(),
                "--edition" => options.edition = number("--edition")? as i64,
                "--seasons" => options.seasons = number("--seasons")?,
                "--seed" => options.seed = number("--seed")?,
                "--model" => options.model = match value.as_str() {
                    "cheap" => Some(MatchModel::Cheap),
                    "shot-based" => Some(MatchModel::ShotBased),
                    _ => return Err(format!("--model must be cheap or shot-based, got {value}")),
                },
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

/// What happened in the events of a set of matches, added up. All zero for a model without detail.
#[derive(Default)]
struct EventStats {
    matches: u32,
    shots: u32,
    on_target: u32,
    corners: u32,
    fouls: u32,
    yellows: u32,
    reds: u32,
    substitutions: u32,
    goals_by_substitutes: u32,
    /// Sum of the home side's possession in percent.
    home_possession: u32,
    /// Goals in the first half (minute 45 or earlier).
    first_half_goals: u32,
    goals: u32,
}

impl EventStats {
    /// Plays every league fixture once more with its league's model and counts its events, to check the model's detail.
    fn measure(world: &World, league: &Competition, rng: &mut Rng) -> EventStats {
        let mut stats = EventStats::default();
        for f in world.fixtures.iter().filter(|f| f.competition == league.id) {
            let report = simulate(league.rules.model, &world.match_input(f.home, f.away, league, None), rng);
            let Some(detail) = report.detail else { return stats };
            stats.matches += 1;
            stats.home_possession += detail.home_possession as u32;
            let mut substitutes = vec![];
            for event in &detail.events {
                match *event {
                    Event::Substitution { on, .. } => {
                        stats.substitutions += 1;
                        substitutes.push(on);
                    }
                    Event::Shot { on_target, .. } => {
                        stats.shots += 1;
                        stats.on_target += on_target as u32;
                    }
                    Event::Goal { minute, scorer, .. } => {
                        stats.goals += 1;
                        stats.goals_by_substitutes += substitutes.contains(&scorer) as u32;
                        stats.first_half_goals += (minute <= 45) as u32;
                    }
                    Event::Corner { .. } => stats.corners += 1,
                    Event::Foul { .. } => stats.fouls += 1,
                    Event::Yellow { .. } => stats.yellows += 1,
                    Event::Red { .. } => stats.reds += 1,
                    _ => {}
                }
            }
        }
        stats
    }

    fn add(&mut self, o: &EventStats) {
        self.matches += o.matches;
        self.shots += o.shots;
        self.on_target += o.on_target;
        self.corners += o.corners;
        self.fouls += o.fouls;
        self.yellows += o.yellows;
        self.reds += o.reds;
        self.substitutions += o.substitutions;
        self.goals_by_substitutes += o.goals_by_substitutes;
        self.home_possession += o.home_possession;
        self.first_half_goals += o.first_half_goals;
        self.goals += o.goals;
    }

    /// One line per match figures, or `None` when there were no events.
    fn line(&self) -> Option<String> {
        if self.matches == 0 {
            return None;
        }
        let n = self.matches as f64;
        Some(format!(
            "    per match: {:.1} shots ({:.0}% on target), {:.2} goals per shot, {:.1} corners, {:.1} fouls, {:.2} yellow, {:.2} red, {:.1} substitutions a side ({:.1}% of goals by substitutes), home possession {:.0}%, {:.1}% of goals in the first half",
            self.shots as f64 / n,
            100.0 * self.on_target as f64 / self.shots.max(1) as f64,
            self.goals as f64 / self.shots.max(1) as f64,
            self.corners as f64 / n,
            self.fouls as f64 / n,
            self.yellows as f64 / n,
            self.reds as f64 / n,
            self.substitutions as f64 / (2.0 * n),
            100.0 * self.goals_by_substitutes as f64 / self.goals.max(1) as f64,
            self.home_possession as f64 / n,
            100.0 * self.first_half_goals as f64 / self.goals.max(1) as f64,
        ))
    }
}

/// Adds up league seasons, for one league or for all of them.
#[derive(Default)]
struct Totals {
    events: EventStats,
    seasons: u32,
    /// Clubs of every league season added, to show an average.
    club_count: usize,
    season: LeagueSeason,
    strongest_won: u32,
    top_three_won: u32,
}

impl Totals {
    fn add(&mut self, clubs: usize, s: &LeagueSeason, events: &EventStats) {
        self.events.add(events);
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

    /// Adds the seasons of another `Totals` (played by another thread).
    fn merge(&mut self, o: &Totals) {
        self.events.add(&o.events);
        self.seasons += o.seasons;
        self.club_count += o.club_count;
        let (s, t) = (&mut self.season, &o.season);
        s.matches += t.matches;
        s.goals += t.goals;
        s.home_wins += t.home_wins;
        s.draws += t.draws;
        s.away_wins += t.away_wins;
        s.champion += t.champion;
        s.runner_up += t.runner_up;
        s.fourth += t.fourth;
        s.last += t.last;
        self.strongest_won += o.strongest_won;
        self.top_three_won += o.top_three_won;
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

/// The totals of a run, by label: all leagues together, the big five together, then each of the big five.
type Report = Vec<(String, Totals)>;

/// Plays the seasons in `seasons` (each with its own seed) of every league on `world`.
fn run(world: &mut World, options: &Options, seasons: Range<u64>) -> Report {
    let (mut all, mut big_five) = (Totals::default(), Totals::default());
    let mut big: Report = TOP_FIVE_LEAGUES.iter().map(|(name, country)| (format!("{name} ({country})"), Totals::default())).collect();
    for season in seasons {
        let mut rng = Rng::new(options.seed + season);
        world.generate_fixtures(); // starts every league again, results cleared
        for league in world.competitions.values_mut() {
            league.rules.model = options.model.unwrap_or(league.rules.model);
        }
        world.play_season_to_end(&mut rng);
        for league in world.competitions.values().filter(|c| c.is_league() && c.clubs.len() >= 8) {
            let result = measure(world, league);
            let events = EventStats::measure(world, league, &mut rng);
            all.add(league.clubs.len(), &result, &events);
            let name = (league.name.as_str(), league.country.as_str());
            if let Some(i) = TOP_FIVE_LEAGUES.iter().position(|&l| l == name) {
                big[i].1.add(league.clubs.len(), &result, &events);
                big_five.add(league.clubs.len(), &result, &events);
            }
        }
    }
    let mut report = vec![("All leagues of 8+ clubs".to_string(), all), ("The big five together".to_string(), big_five)];
    report.extend(big);
    report
}

/// Seasons are independent (each has its own seed), so they are shared out over threads, each with its own world.
/// Below this many seasons the cost of loading a world per thread outweighs the gain.
const SEASONS_WORTH_THREADING: u64 = 3;

/// Plays all the seasons, on as many threads as there are cores (one thread for a short run), and adds them up.
fn play(options: &Options) -> Report {
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get() as u64);
    let threads = if options.seasons < SEASONS_WORTH_THREADING { 1 } else { cores.min(options.seasons) };
    let share = options.seasons.div_ceil(threads);
    let reports: Vec<Report> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let range = (t * share)..((t + 1) * share).min(options.seasons);
                scope.spawn(move || run(&mut load_world(options), options, range))
            })
            .collect();
        handles.into_iter().map(|h| h.join().expect("a simulation thread panicked")).collect()
    });
    let mut reports = reports.into_iter();
    let mut total = reports.next().expect("at least one thread");
    for report in reports {
        for ((_, t), (_, o)) in total.iter_mut().zip(&report) {
            t.merge(o);
        }
    }
    total
}

/// The chosen edition from the database, or an error message and exit.
fn load_world(options: &Options) -> World {
    let conn = Connection::open(&options.db).unwrap_or_else(|e| {
        eprintln!("cannot open {}: {e}. Build it first: cargo run -p import --release", options.db);
        std::process::exit(1);
    });
    let mut world = load_edition(&conn, options.edition).unwrap_or_else(|e| {
        eprintln!("cannot load edition {}: {e}", options.edition);
        std::process::exit(1);
    });
    world.choose_formations(); // as a new career does
    world
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let options = Options::parse(&args).unwrap_or_else(|message| {
        eprintln!("{message}\nUsage: simtool [--edition 15] [--seasons 20] [--seed 1] [--model cheap|shot-based] [--db history.db]");
        std::process::exit(2);
    });
    let start = Instant::now();
    let totals = play(&options);
    let elapsed = start.elapsed();
    println!("Edition {}, {} seasons from seed {}, {} model.\n", options.edition, options.seasons, options.seed, options.model.map_or("each league's own".to_string(), |m| format!("{m:?}")));
    for (label, t) in totals.iter().filter(|(_, t)| t.seasons > 0) {
        println!("{}", t.line(label));
        if let Some(events) = t.events.line() {
            println!("{events}");
        }
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
        assert_eq!((o.edition, o.seasons, o.seed, o.db.as_str(), o.model), (15, 20, 1, "history.db", None));
        let o = Options::parse(&strings(&["--edition", "24", "--seasons", "5", "--seed", "9", "--db", "x.db"])).unwrap();
        assert_eq!((o.edition, o.seasons, o.seed, o.db.as_str()), (24, 5, 9, "x.db"));
    }

    #[test]
    fn bad_options_are_errors_not_panics() {
        assert!(Options::parse(&strings(&["--seasons"])).is_err());
        assert!(Options::parse(&strings(&["--seasons", "many"])).is_err());
        assert!(Options::parse(&strings(&["--colour", "red"])).is_err());
        assert!(Options::parse(&strings(&["--model", "magic"])).is_err());
    }

    #[test]
    fn a_run_reports_shares_that_add_up_and_is_repeatable() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../history.db");
        if !std::path::Path::new(path).exists() { return; } // needs `cargo run -p import` first
        let mut world = load_edition(&Connection::open(path).unwrap(), 15).unwrap();
        let options = Options { db: path.into(), edition: 15, seasons: 1, seed: 3, model: None };
        let first = run(&mut world, &options, 0..1);
        let second = run(&mut world, &options, 0..1);
        let (all, again) = (&first[0].1, &second[0].1);
        let s = &all.season;
        assert!(s.matches > 5000);
        assert_eq!(s.matches, s.home_wins + s.draws + s.away_wins);
        let goals_per_match = s.goals as f64 / s.matches as f64;
        assert!((1.5..4.5).contains(&goals_per_match), "{goals_per_match} goals a match");
        assert_eq!((s.goals, s.home_wins), (again.season.goals, again.season.home_wins), "the same seed gives the same run");
    }

    #[test]
    fn sharing_seasons_over_threads_gives_the_same_totals_as_playing_them_in_a_row() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../history.db");
        if !std::path::Path::new(path).exists() { return; } // needs `cargo run -p import` first
        let options = Options { db: path.into(), edition: 15, seasons: 4, seed: 3, model: None };
        let in_a_row = run(&mut load_world(&options), &options, 0..4);
        let threaded = play(&options);
        let (a, b) = (&in_a_row[0].1.season, &threaded[0].1.season);
        assert_eq!((a.matches, a.goals, a.home_wins, a.draws), (b.matches, b.goals, b.home_wins, b.draws));
    }
}
