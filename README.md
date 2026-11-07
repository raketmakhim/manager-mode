# Manager Mode

A football (soccer) career/manager-mode game in the style of FIFA's Career Mode. You run a club across seasons: set the lineup and tactics, simulate matches (no playable matches, simulation only), buy and sell players, and try to win trophies without going broke.

> Status: planning. See [PLAN.md](PLAN.md) for the roadmap.

## Core features (v1)

- Pick a club and start a career in any year covered by the data (e.g. 2008), with players developing as they really did
- League with fixtures, results and a live table
- Squad management: players, positions, ratings, form, fitness
- Formation and lineup selection
- Match simulation with a text commentary feed
- Transfer market: buy, sell, contract and budget limits
- Season rollover: player progression, aging, retirements

## Later

Youth academy, cups and European competitions, training drills, board objectives, save/load, real-time 2D match view.

## Tech (proposed)

Rust, as a Cargo workspace. The game logic lives in a UI-free library crate so it can be simulated thousands of times fast and tested without a front end. The first front end is a terminal UI (`ratatui`). Saves are JSON files (`serde`). Randomness is a seeded `rand_chacha` RNG, so any match or season can be replayed exactly.

## Getting started

Only the data importer exists so far. Put the Kaggle CSVs in `research/information/data/` (`male_players.csv`, `male_teams.csv`, `male_coaches.csv`), then build the reference database:

```
cargo run -p import --release     # writes history.db (~46 MB, ~20 s)
```

Then open the window. The Season tab starts a season in any edition and shows the fixtures; the Player scout tab shows how any player's stats really changed in later editions:

```
cargo run -p gui --release        # graphical window with two tabs: Season and Player scout
cargo run -p tui --release        # same thing in the terminal (Enter to open, Left/Right for seasons)
```

The rest, once scaffolded:

```
cargo run -p tui        # play
cargo run -p simtool --release   # bulk-simulate matches/seasons and print balance stats
cargo test
```

## Project layout (planned)

```
crates/
  engine/    match sim, transfers, season logic, data model, save/load (no UI)
  tui/       terminal front end (ratatui)
  simtool/   bulk simulation and balance reports
```
