# Manager Mode

A football (soccer) career/manager-mode game in the style of FIFA's Career Mode. You run a club across seasons: set the lineup and tactics, simulate matches (no playable matches, simulation only), buy and sell players, and try to win trophies without going broke.

> Status: early development. Milestone M1 (data, world model, fixtures, first window) is done; next is M2, the match simulation. See [PLAN.md](PLAN.md) for the roadmap.

## What works today

- Imports real FIFA player data (FIFA 15 to FC 24, 2014/15 to 2023/24) into one SQLite file
- Loads any of those editions into the game: every league, club and squad, with the real attributes
- Generates each league's season fixtures (everyone plays everyone home and away, at most 3 matches per club per week)
- A window with a **Career** tab (new career: pick a year, a league and a club, see that club's fixtures with dates, and press **Play next match** to simulate it, switch to **My squad** to see how your players' ratings changed since an earlier season, **Lineup** to pick a formation, the starting eleven and the bench by clicking players to swap them (or let the game pick), **Competitions** for any league table or cup rounds, **Finish season** to sim the rest, then **Start next season** for promotion, relegation and new fixtures) and a **Player scout** tab (search any player and see how their stats really changed in later editions)

## Planned features (v1)

- Pick a club and start a career in any year covered by the data, with players developing as they really did
- League table and results
- Squad management: players, positions, ratings, form, fitness
- Formation and lineup selection
- Match simulation with a text commentary feed
- Transfer market: buy, sell, contract and budget limits
- Season rollover: player progression, aging, retirements

## Later

Youth academy, European competitions, training drills, board objectives, save/load, real-time 2D match view, editions before FIFA 15.

## Tech

Rust, as a Cargo workspace. The game logic lives in a UI-free library crate so it can be simulated thousands of times fast and tested without a front end. The window uses `egui`; there is also a terminal view (`ratatui`). Reference data is a read-only SQLite file (`rusqlite`); saves will be JSON files (`serde`). Randomness will be a seeded RNG, so any match or season can be replayed exactly.

## Getting started

The player data is not in the repo. Put the Kaggle CSVs in `research/information/data/` (`male_players.csv`, `male_teams.csv`, `male_coaches.csv`), then build the reference database:

```
cargo run -p import --release     # writes history.db (~46 MB, under a minute)
```

Then open the window:

```
cargo run -p gui --release        # tabs: Career, Player scout
cargo run -p tui --release        # the player scout in the terminal (Enter to open, Left/Right for seasons)
cargo test                        # tests that need history.db skip themselves if it is missing
```

Coming with M2:

```
cargo run -p simtool --release    # bulk-simulate matches/seasons and print balance stats
```

## Project layout

```
crates/
  import/    builds history.db from the CSVs
  engine/    game logic, no UI: attributes, world model, loading an edition, competitions and fixtures, match outcomes, player history
  gui/       the window (egui): Career (fixtures, squad, competitions) and Player scout tabs
  tui/       the terminal player scout (ratatui)
  simtool/   (planned, M2) bulk simulation and balance reports
research/    notes behind the decisions
```
