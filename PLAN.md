# Plan

## Goal

A playable single-player manager mode: one full season loop (squad -> fixtures -> matches -> transfers -> next season) that is fun to repeat. Text-based match sim first; visuals come later.

## Decisions to confirm

| Question | Default |
|---|---|
| Language | Rust (decided) |
| Front end | `egui` window (`crates/gui`) plus a `ratatui` terminal view (`crates/tui`); both sit on the UI-free `engine` crate. Bevy only if a 2D match view is ever added |
| Match presentation | Simulation only: you never play a match. Text commentary + stats, no 2D pitch |
| Data | Historical yearly datasets (one per FIFA edition) in a read-only SQLite file, `history.db`, built by `crates/import`; start a career in any covered year. Today: FIFA 15 to FC 24 (Kaggle); earlier editions need a scrape. `engine::load_edition` turns an edition into an in-memory `World`; it becomes a `DataSource` trait only if a second source appears. The data is not committed. |
| Competitions | A competition is a list of **stages** (round-robin, knockout, league phase...), not a type per kind. Rules are data and depend on the season (formats change between 2014 and 2023). Pairing, calendar placement, standings and qualification stay separate and reusable. Design in `research/engineering/competition-design.md` |
| Match detail | Two levels, set per competition in its rules data. **Full** (events, cards, subs, ratings): the top flights of England, Spain, Germany, Italy and France, the Champions League, and the user's competitions. **Cheap** (score, scorers, assisters; clean sheets from the score): everything else, built on the same team-strength function and checked against the full simulation in `simtool`. Estimated saving about 72 to 79% of match work. Stats are totals per (season, competition, player), rebuilt from match reports. Details in `research/engineering/competition-design.md` |
| Default competitions | A league or country we have not researched gets defaults, set in one place: a plain double round-robin, and a single-leg "<Country> Cup" knockout (random draw, lower division hosts, extra time then penalties, byes for the best clubs, no cup under 4 clubs). A researched competition replaces the default by registering its own rules for that season. Details in `research/engineering/competition-design.md` |
| European places | A country's Champions League, Europa League and Conference League places are computed from its position in UEFA's country ranking that season (rank bands), plus the title-holder extra place and bans (Russia 2022/23 and 2023/24). We store each season's ranking order. Which club gets each place is a small table: finishing position or cup winner, to entry stage. See `research/game-features/european-places-by-season.md` |
| Rescheduling | A moved match is placed with `first_free_week` as soon as possible after its original week. European dates are fixed first and domestic rounds fit around them. League calendars will need gaps (international and winter breaks); today round N is always week N. See `research/game-features/fixture-rescheduling.md` |
| Calendar | Every fixture has a `competition` and a `week` (a 7-day block). A club plays at most 3 matches in any week (`MAX_MATCHES_PER_WEEK`). Leagues are scheduled up front, one round a week; cups will draw each round after the last is played and place it with `first_free_week`. `calendar_problems()` must always be empty |
| Player growth | Follow real ratings from the dataset while history matches; once the world diverges, simulated growth nudged toward the real curve; fully simulated after the last data year |
| Saves | JSON via `serde`, one file per save |
| Randomness | Our own small seeded `Rng` (SplitMix64, `engine/src/rng.rs`), so a seed gives the same numbers on every machine and library version; every sim takes an explicit `&mut Rng`, no global state |
| Player progression | The real data is the baseline, and a player's rating moves by the real change between editions (`advance_ratings`: new = current + real next - real now, kept in 0-99). An untouched player lands exactly on the real rating; one the user trained up or let slip keeps that difference, so the user can make or break players. Training, playing time and injuries will add their own changes on top. When an attribute moves, the overall moves by the position's weight times the change (`engine/src/overall.rs`; weights fitted to the data, see `research/game-features/overall-rating-formula.md`); an XP level-up goes through that. Squads keep their players for now (real transfers are not followed). The data ends at FC 24, so later seasons need generated progression |

## Data model

Built in `crates/engine/src/model.rs`; everything points at everything else by id.

- **Player**: id, name, birth date, positions, 34 typed attributes, overall, potential, club, wage, contract end, value, form, fitness
- **Club**: id, name, domestic league, squad, budget, lineup
- **Competition**: id, name, country, kind (league or cup), clubs
- **Fixture**: competition, round, week, home, away, result
- **World**: edition, year, competitions, clubs, players, fixtures (the whole game state)
- Still to add: league table, current week, transfer list, formation, the user's club

## Milestones

### M1 - Foundations
- Cargo workspace with `import`, `engine`, `gui`, `tui` crates (`dev-tools/simtool` for balance checks); `cargo test` running
- Data model types (plain structs; `serde` derives are added with save/load in M5)
- `load_edition`: clubs, players and ratings for one edition
- Round-robin fixture generator with the weekly match limit
- Real data import tool (separate from the game) that builds the SQLite reference DB from the yearly datasets; the dataset is not committed to the repo. **Done**: `crates/import` builds `history.db` from the Kaggle files (FIFA 15 to FC 24), design in `research/engineering/data-import.md`
- **Done when**: tests confirm a generated league has valid fixtures (everyone plays everyone home and away)
- **Status: done** except `simtool` (not needed until M2). The generated fictional source was dropped: real data replaces it. Built: `crates/import`, `crates/engine` (`attrs`, `competition`, `history`, `model`, `outcome`, `source`), `crates/gui` (Season and Player scout tabs), `crates/tui`

### M2 - Match simulation
- Team strength from lineup (attributes by position, formation fit, fitness, form)
- Minute-by-minute sim producing score, events (goals, cards, injuries) and commentary
- `dev-tools/simtool` simulates N seasons of every league and prints goals per match, home/draw/away shares, champion points and how often the strongest club wins. Done: one thread (0.5 s a season in release); add `rayon` when it gets slow. The constants in `sim.rs` are tuned to the real big-five tables (`research/game-features/simulation-balance.md`); still off: draws are 2 points too rare (needs a Dixon-Coles style correction), league goal rates are not per country, only FIFA 15 was tuned.
- **Done when**: stronger team wins noticeably more often over 100k simulated matches; same seed gives same result

### M3 - Season loop
- Play next round, update table, fitness and form
- Lineup and formation screen (GUI)
- League table and competitions screens: done (Career tab: fixtures, squad, competitions)
- Calendar: league matches are on Saturdays; default cup rounds are on Wednesdays in weeks 3, 7, 11... (a `Day` on every fixture; Champions League Wednesday and Europa League Thursday use the same field when they exist). Squad view: the Career tab's **My squad** view lists your players with overall and potential changes since the start of an earlier season (click a player for every attribute that changed). Season change: `start_next_season` does promotion and relegation (each league's `rules.relegated`, default 3, between neighbouring `level`s of a country; levels come from the data's `league_level`) and builds the next season's fixtures; player ratings follow the real change between editions (`advance_ratings`, see the Player progression decision). First game screen: a Career tab (new career, club fixtures with dates, a **Play next match** button) is done; the cheap match simulation (`sim.rs`: team strength, Poisson goals, extra time, shootout) and `play_next_match` / `play_week` are in. League tables (`standings`) and a Competitions view (tables, cup rounds) are in. The lineup picker is in (`lineup.rs`: four formations, a best-eleven pick, seven substitutes, and a Lineup view where you click two players to swap them; a player's rating in a slot comes from his own attributes using the overall weights, with a small penalty for unlisted positions; the match sim rates each starter in his slot, the bench is picked and shown but does not play yet: no substitutions are simulated; a whole-world season of about 10,000 matches takes 0.5 s in release, 10 s in debug). Still to do in M2: scorers and assisters, simtool. Competition stages: **steps 1-2 done** (types, `RoundRobin`, richer match `Outcome`, single-leg `Knockout` with a seeded draw and byes, a default cup per country, our own seeded `Rng`); next a shared standings function, then rules by season (see `research/engineering/competition-design.md`). League phase, split, groups and two-leg ties come later, each when a competition needs it
- **Done when**: a full season can be played start to finish in the GUI

### M4 - Transfers and finance
- Player valuation, transfer list, bids with AI accept/reject logic
- Wage and budget checks, contract expiry
- AI clubs trade among themselves at a basic level
- **Done when**: user can buy/sell, and the budget cannot go invalid

### M5 - Progression and multi-season
- End-of-season: aging, development toward potential, decline, retirement, new youngsters
- Board objectives and a sack/keep result
- Save/load (save records which dataset it started from)
- Historical growth: real ratings as a target curve, simulated growth when diverged
- **Done when**: 5 seasons run without broken state; reload restores the game

### M6 - Polish (optional)
- Domestic cups (once stages exist in M3; formats in `research/game-features/domestic-cup-formats.md`), European competitions and qualification, match rescheduling, calendar gaps, youth academy, training, injuries depth, graphical UI (`egui`/Bevy), 2D match view

## Risks

- **Balance**: sim and economy tuning takes the most iteration. Mitigation: seeded sim plus a script that runs bulk simulations and prints stats.
- **Scope creep**: FIFA career mode is huge. Mitigation: ship M1-M5 before adding anything from M6.
- **AI transfers**: easy to make the market unrealistic. Start with simple rules (value +/- random margin).
- **Competition rules vary by season and country**: the research covers the English, German, Italian and Spanish cups and Europe's top-15 countries for 2014/15 to 2023/24. The other leagues in the data (up to about 56) will use the default league and cup until researched.

## Next step

M2: lineup picker, team strength, a simple goal-based match, then `simtool` to tune the balance.
