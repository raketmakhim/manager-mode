# Plan

## Goal

A playable single-player manager mode: one full season loop (squad -> fixtures -> matches -> transfers -> next season) that is fun to repeat. Text-based match sim first; visuals come later.

## Decisions to confirm

| Question | Default |
|---|---|
| Language | Rust (decided) |
| Front end | Terminal UI with `ratatui` (alternative later: `egui` or Bevy for a graphical UI; the engine crate stays the same) |
| Match presentation | Simulation only: you never play a match. Text commentary + stats, no 2D pitch |
| Data | Historical yearly datasets (one per FIFA edition, ~20-30 years) in a read-only SQLite reference DB; start a career in any year. Generated fictional data stays as a fallback and for tests. Both sit behind a `DataSource` boundary in the engine. |
| Calendar | Every fixture has a `competition` and a `week` (a 7-day block). A club plays at most 3 matches in any week (`MAX_MATCHES_PER_WEEK`). Leagues are scheduled up front, one round a week; cups will draw each round after the last is played and place it with `first_free_week`. `calendar_problems()` must always be empty |
| Player growth | Follow real ratings from the dataset while history matches; once the world diverges, simulated growth nudged toward the real curve; fully simulated after the last data year |
| Saves | JSON via `serde`, one file per save |
| Randomness | Seeded `rand_chacha`; every sim takes an explicit RNG, no global state |

## Data model (minimal)

- **Player**: id, name, age, position, attributes (pace, shooting, passing, defending, physical, goalkeeping), overall, potential, form, fitness, contract (wage, years), value
- **Club**: id, name, budget, wage budget, squad, formation, lineup
- **Fixture**: round, home, away, result
- **Season**: year, league table, fixtures, current round
- **GameState**: user club, all clubs, season, transfer list, date

## Milestones

### M1 - Foundations
- Cargo workspace with `engine`, `tui`, `simtool` crates; `cargo test` running
- Data model types (plain structs, `serde` derives)
- `DataSource` trait: returns clubs, players and ratings for year Y
- Generated source first: 8-20 fictional clubs with 20-25 players each, round-robin fixture generator
- Real data import tool (separate from the game) that builds the SQLite reference DB from the yearly datasets; the dataset is not committed to the repo. **Done**: `crates/import` builds `history.db` from the Kaggle files (FIFA 15 to FC 24), design in `research/engineering/data-import.md`
- **Done when**: tests confirm a generated league has valid fixtures (everyone plays everyone home and away)
- **Status: done** except `simtool` (not needed until M2) and the generated fictional source (dropped: real data replaces it). Built: `crates/import`, `crates/engine` (`model`, `source::load_edition`, `fixtures::round_robin`), `crates/gui`, `crates/tui`

### M2 - Match simulation
- Team strength from lineup (attributes by position, formation fit, fitness, form)
- Minute-by-minute sim producing score, events (goals, cards, injuries) and commentary
- `simtool` runs N matches/seasons in parallel (`rayon`) and prints win rates, goals per game, etc.
- **Done when**: stronger team wins noticeably more often over 100k simulated matches; same seed gives same result

### M3 - Season loop
- Play next round, update table, fitness and form
- Lineup and formation screen (TUI)
- League table and fixtures screens (TUI)
- **Done when**: a full season can be played start to finish in the TUI

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
- Cups, youth academy, training, injuries depth, graphical UI (`egui`/Bevy), 2D match view

## Risks

- **Balance**: sim and economy tuning takes the most iteration. Mitigation: seeded sim plus a script that runs bulk simulations and prints stats.
- **Scope creep**: FIFA career mode is huge. Mitigation: ship M1-M5 before adding anything from M6.
- **AI transfers**: easy to make the market unrealistic. Start with simple rules (value +/- random margin).

## Next step

Confirm the decisions table above, then start M1.
