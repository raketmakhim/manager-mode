# How we import the data

Designed and built 2026-11-06 in `crates/import`. Based on the Kaggle files in `research/information/data/` (see [player-data-sources.md](../information/player-data-sources.md)). Column names below are from those files. Where the built tool differs from this design it is noted under "What was built" at the end.

## Approach

A small Rust command-line tool, `crates/import`, reads the CSVs and writes one SQLite file, `history.db`. The game never reads CSVs; it only reads `history.db`.

```
cargo run -p import --release            # defaults: research/information/data -> history.db
cargo run -p import --release -- <data_dir> <out.db>
```

- Crates: `csv` (read), `rusqlite` (write). Rows are streamed, not loaded all at once.
- Everything is written in one transaction with prepared statements. 180k rows should take seconds (my estimate, unmeasured).
- Re-running deletes and rebuilds `history.db`. The raw CSVs are the source of truth, `history.db` is disposable.
- `data/` and `history.db` go in `.gitignore` (96 MB CSV alone).

## Schema (proposed)

One row for who a player is, one row per player per edition for how good they were:

| Table | Key | Holds |
|---|---|---|
| `editions` | `version` | 15..24, name (FIFA 15 ... FC 24), `as_of` date from `update_as_of` |
| `players` | `player_id` | `short_name`, `long_name`, `dob`, `nationality_id`, `nationality_name`, `preferred_foot` |
| `ratings` | `player_id, version` | `overall`, `potential`, `age`, `positions`, `value_eur`, `wage_eur`, `height_cm`, `weight_kg`, `weak_foot`, `skill_moves`, `international_reputation`, `work_rate`, `body_type`, ~36 attributes, `club_team_id`, `club_position`, `club_jersey_number`, `contract_until`, `club_joined_date`, `loaned_from`, `release_clause_eur`, `player_traits`, `player_tags` |
| `teams` | `team_id` | latest `team_name` |
| `team_editions` | `team_id, version` | `league_id`, `league_level`, ratings (`overall`, `attack`, `midfield`, `defence`), `coach_id`, `home_stadium`, `rival_team`, prestige, `transfer_budget_eur`, `club_worth_eur`, tactics columns |
| `leagues` | `league_id` | `league_name`, nation |
| `coaches` | `coach_id` | `short_name`, `long_name`, `dob`, `nationality_name` |

Notes:

- Attributes stay as nullable small integers (blank in the CSV becomes NULL, not 0).
- Left out on purpose: the position-rating columns (`ls` ... `gk`), because they can be derived from attributes; face and flag URLs; player and team URLs. Can be added later.
- The league name and club name appear on every player row. The import splits them into `leagues`/`teams` and keeps ids only in `ratings`.
- Index `ratings(version, club_team_id)` so "all players at club X in edition V" is one fast query.

## Checks the importer runs (and fails loudly on)

- Row count per edition matches the CSV (16k-19k players per edition).
- `(player_id, version)` is unique.
- Every `club_team_id` in `ratings` exists in `team_editions` for the same version. Player rows may have a free agent with no club; allow NULL, but report the count.
- Every `coach_id` in `team_editions` exists in `coaches`.
- Print a summary table per edition (players, clubs, leagues, % NULL attributes).

## Mapping edition to start year

`version 15` shipped September 2014, so it describes the 2014/15 season. Store `as_of`, and let the game's "start year" menu show the edition name and season (e.g. "FIFA 15 - 2014/15"), not a bare year.

## Adding more data later

Each source gets its own small reader that outputs the same internal rows (player, rating, team). Examples:

- FC 25 Kaggle file (different columns, one new reader).
- A sofifa scrape for FIFA 07-14. Map attributes by name on the page, never by column position (see the drift finding in the data sources note).

The readers are the only place that knows about CSV column names. The engine's `DataSource` only sees `history.db`.

## How the game uses `history.db`

1. New game: ask for edition and club, query that edition's clubs, squads, ratings, and copy them into in-memory structs.
2. Season rollover: for a player, look up the next edition's `ratings` row as the real-growth target (see [data-storage.md](data-storage.md)).
3. After the last edition, or if a player is missing from later ones: simulated growth only.

## Open questions

- Missing data to fill: `transfer_budget_eur` is empty for all of FC 24, `club_worth_eur` for all of FIFA 15, tactics before FIFA 21. Likely derived from club rating and league.
- Does the game need `ratings` for all 10 editions loaded, or just the start edition plus lookups? Lookups, since the file is small.
- Size of `history.db` once built; measure after the first import.

## What was built

- It takes two optional arguments (data folder, output file) instead of one flag per CSV.
- 180,021 ratings, 53,111 players, 1,145 teams and 1,369 coaches imported; 0 orphan clubs or coaches; 1,865 ratings have no club (free agents).
- `leagues` also keeps `nationality_name` (the league's country), because several countries have a league with the same name. The game shows it as "Premier League (England)".
- `team_editions` keeps only the columns the game uses so far: league, team ratings, coach, prestige, budget, club worth, stadium and a few tactics fields (not `rival_team`, set pieces or the finer tactics).
- Checks run on every import: every CSV row landed, ids point at rows that exist, and a per-edition summary is printed.
- The game side lives in `crates/engine`: `source::load_edition` builds a `World` from one edition, and `history::career` reads one player across all editions.
