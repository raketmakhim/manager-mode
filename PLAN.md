# Plan

## Goal

A playable single-player manager mode: one full season loop (squad -> fixtures -> matches -> transfers -> next season) that is fun to repeat. Simulation only: you never play a match; text and stats first, visuals later.

This file holds decisions and milestones. How things work and why lives in `research/` (one note per feature, indexed in `research/CLAUDE.md`); what exists lives in the code and its module docs.

## Decisions

| Question | Decision |
|---|---|
| Language | Rust |
| Front end | `egui` window (`crates/gui`) and a `ratatui` terminal view (`crates/tui`), both on the UI-free `engine` crate. Bevy only if a 2D match view is ever added |
| Data | Yearly datasets (one per FIFA edition) in a read-only SQLite file, `history.db`, built by `crates/import`; start a career in any covered year (FIFA 15 to FC 24 today). `engine::load_edition` makes an in-memory `World`; it becomes a `DataSource` trait only if a second source appears. The data is not committed. `research/engineering/data-import.md` |
| Competitions | A competition is a list of **stages** (round-robin, knockout, league phase...), not a type per kind. Rules are data and depend on the season; pairing, calendar placement, standings and qualification stay separate and reusable. Every country gets two default cups (a cup and a league cup: single-leg knockouts of all its clubs, drawn round by round, on different days); real league cups exist in only some countries and were left unresearched, and the researched real cup formats (byes, two-legged semi-finals, seeding) are not built. Default league and cup for countries not researched, set in one place. `research/engineering/competition-design.md`, `research/game-features/competition-formats.md`, `domestic-cup-formats.md` |
| European places | Computed from the country's UEFA ranking that season (a stored order per season, top 15) and the Russia ban; the title-holder extra place is left out. Built for the Champions League only: the top places of each country's first division, no qualifying rounds, a group stage of up to eight groups, two-legged knockout rounds and a one-match final (`competition/european.rs`). A new career asks whether to put your club in if it would not qualify. The Europa and Conference Leagues are not built. `research/game-features/european-places-by-season.md` |
| Calendar | Every fixture has a competition, a week (7 days) and a day. A club plays at most 3 matches a week (`MAX_MATCHES_PER_WEEK`); `calendar_problems()` must always be empty. Moved matches use `first_free_week`; league calendars will need breaks. `research/game-features/fixture-rescheduling.md` |
| Match detail | The **shot-based** model (events, cards, subs, ratings) plays every competition today (`DEFAULT_MODEL`); the **cheap** model (team strength to a score) stays for when background leagues need speed. A shot-based match can be played a minute at a time (`LiveMatch`) and watched. Both are checked against real tables in `simtool`. `research/game-features/match-simulation-methods.md`, `simulation-balance.md` |
| Swappable models | Every engine that could be argued about is an enum with `ALL`, a `DEFAULT_*` and contract tests that run for every variant: match, scoring, discipline, substitution, fatigue, rating, experience, ageing, retirement |
| Player progression | The user's club develops by our own rules (match XP levels, a summer ageing table, a real rise in the data is kept); everyone else follows the real change between editions (`advance_ratings`). After FC 24 other clubs need generated progression. `research/game-features/experience.md` |
| Transfers, retirement, fitness | The real moves between editions are made each summer with a yes/no on deals involving your club; retirement is decided at the start of a season and made at its end; fitness carries between matches. `transfers.md`, `retirement.md`, `fitness.md` |
| Awards | Worked out from the season's stats when asked (`awards.rs`), so every past season has them: a score per player (rating above an ordinary match, goals weighted by line, assists, clean sheets, player of the match; Champions League counts 1.5x; every player of a first-division champion gets +10, of the Champions League winner +15 and of a cup or league cup winner +5, recorded in his stats when it is decided), then best player, best young player (21 or younger), best of each line, a 4-3-3 best eleven, top scorers and assist leaders, for all competitions (the Ballon d'Or) or one (a cup only has scorers and assist leaders). Titles and each season's awards are kept as honours (`honours.rs`) and listed on the player's page. The weights are guesses, not researched |
| Saves | JSON via `serde`, one file per save (not built) |
| Randomness | Our own seeded `Rng` (SplitMix64), so a seed gives the same numbers everywhere; every sim takes an explicit `&mut Rng`, no global state |

## Data model

In `crates/engine/src/model.rs`; everything points at everything else by id.

- **Player**: id, name, birth date, positions, 34 typed attributes, overall, potential, club, wage, contract, value, plus game-only state (form, fitness, XP, career status)
- **Club**: id, name, league, squad, budget, formation and selection
- **Competition**, **Fixture** (competition, round, week, day, home, away, result), **World** (the whole game state: edition, year, competitions, clubs, players, fixtures, stats, settings)
- Stats are totals per (player, season, competition), rebuilt from match reports.
- Honours (titles won and each season's awards) are a list on the world, because competitions are rebuilt each season and players move clubs; the awards themselves are worked out from the stats.
- Still to add: current week as a calendar you move through, transfer list, the user's club in the world.

## Milestones

### M1 - Foundations (done)
Workspace (`import`, `engine`, `gui`, `tui`, `dev-tools/simtool`), data model, `load_edition`, round-robin fixtures with the weekly limit, the import tool. The generated fictional source was dropped: real data replaces it.

### M2 - Match simulation (done)
Team strength from lineups, the cheap and shot-based models, scorers, assisters, cards, substitutions, fatigue, ratings. `simtool` shares seasons over one thread per core (40 seasons in about 6 s) and prints goals, shares, champion points, shots, cards, corners, possession and substitutions. Tuned to the real big-five tables (`simulation-balance.md`). Still off: league goal rates are not per country.

### M3 - Season loop (done)
Career tab: new career in any year, fixtures with dates, play or watch the next match, finish the season, next season with promotion and relegation. Lineup screen (ten formations, swap players), squad view with player pages, league tables and cup rounds, stats views. A whole-world season takes about 0.4 s in release.
- **Done when**: a full season can be played start to finish in the GUI (met).

### M4 - Transfers and finance
- Done: the summer window (`transfer`), with your yes/no on deals that involve your club.
- To do: fees paid from budgets, your own bids and transfer list, player valuation, AI accept/reject, wage and budget checks, contract expiry, AI clubs trading, a model for after FC 24 (no real moves to replay).
- **Done when**: user can buy/sell, and the budget cannot go invalid.

### M5 - Progression and multi-season
- Done: match XP and levels, summer ageing, retirement, fitness between matches.
- To do: injuries, ageing of other clubs' players after FC 24, new youngsters, board objectives and a sack/keep result, save/load, a calendar you move through.
- **Done when**: 5 seasons run without broken state; reload restores the game.

### M6 - Polish (optional)
Domestic cups beyond the default, the Europa and Conference Leagues and qualifying rounds, match rescheduling, calendar gaps, youth academy, training, injuries depth, graphical UI, 2D match view.

## Risks

- **Balance**: sim and economy tuning takes the most iteration. Mitigation: seeded sim and `simtool`.
- **Scope creep**: FIFA career mode is huge. Mitigation: ship M1-M5 before anything from M6.
- **AI transfers**: easy to make the market unrealistic. Start with simple rules (value +/- random margin).
- **Competition rules vary by season and country**: researched for the English, German, Italian and Spanish cups and Europe's top-15 countries, 2014/15 to 2023/24. The other leagues in the data (about 56) use the default league and cup until researched.

## Next step

Your call. Candidates: injuries, new youngsters, your own bids (M4), a calendar you move through, save/load.
