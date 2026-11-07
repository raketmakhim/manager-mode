# Where do we store footballer data?

Researched 2026-11-06. Sources are fan/modding forums and general Rust database docs; no official studio documentation was found. Items marked **(unverified)** are not from a source.

## How the big games do it

Both FM and FIFA/FC use **two layers**:

1. **Starting database**: the default clubs, players, leagues shipped with the game.
2. **Save file**: a snapshot of the whole world for one career, which then diverges from the database.

| | Football Manager | FIFA / EA FC |
|---|---|---|
| Starting data | Official database shipped with the game, updated each year. Custom databases are `.fmf` files made with the official pre-game editor. | `fifa_ng_db.db` plus `fifa_ng_db-meta.xml`; one community post says ~160 tables. |
| Edits | Editor saves a small separate edit file that sits on top of the base data. | Modders export tables, edit them, re-import (tools: DB Master, Frosty). |
| Saves | Player data seems copied into each save at creation: database changes only show up in a new game. Save format is binary; no layout found. | Career saves are a separate format with no extension and a CRC check. Editing exported tables can corrupt a save, per forum reports. |

Takeaway: **no database server is attached to the game.** The data is local files; the "database" is just the starting snapshot. Editing the starting data after a save exists does nothing, because the save carries its own copy.

## What this means for us

Our data is small. 20 clubs x 25 players is ~500 players; even 10 leagues is ~15k. That fits comfortably in memory and loads in milliseconds.

Plan:

1. **Starting database = generated.** The game generates fictional clubs and players from a seed (M1). Optionally ship a hand-editable file later (e.g. JSON or RON) so modders can swap data. No licensed data.
2. **Whole game state held in memory** as plain Rust structs (`Vec<Club>`, `Vec<Player>`).
3. **Save = serialize the whole state with `serde`** to one file.
   - Start with **JSON**: human-readable, easy to debug, diffable.
   - Switch to **bincode + compression** only if saves get big or slow (measure first).
   - Include a `version` field so old saves can be migrated.
   - Write to a temp file, then rename, so a crash mid-save can't corrupt the existing save.

## Do we need a real database?

Not for now. Options if we ever do:

| Option | What it is | When it makes sense |
|---|---|---|
| **serde JSON / bincode file** | Not a database, just a serialized snapshot | **Our default.** Small saves, read whole save at once. |
| **SQLite (`rusqlite`)** | Mature embedded SQL, inspectable file, migrations | We need queries across large history (e.g. every match for 50 seasons) or want to inspect data with outside tools. |
| **redb** | Pure-Rust embedded key-value store, ACID, crash-safe, versioned file format | Want transactions without SQL. |
| **sled** | Embedded key-value store | Avoid: slower in benchmarks, API is raw bytes, and I recall it being unmaintained **(unverified)**. |

No source benchmarks these for game saves specifically. For `simtool` (bulk simulation), keep everything in memory and write results to CSV or print them; no database needed.

## Revision: 20-30 yearly datasets, start in any year (added 2026-11-06)

New requirement: store one dataset per year (e.g. every FIFA edition), let the player start a career in any year, and have players develop the way they really did. This **supersedes the "generated fictional data" and "no database" points above** for the starting data. The save-file design (serde snapshot) still stands.

### Does it make sense? Yes, with two layers

1. **Reference database (read-only, ships with the game).** All years in one file.
2. **Save file (read-write).** One career's world, as above. It records which dataset version it started from.

Starting in 2008 = query year 2008 from the reference DB, load those clubs/squads/players into memory, then play on in memory. After that, only the save changes.

### Shape of the reference data

Store a player's identity once and their ratings per year, not 25 copies of everyone:

- `players(player_id, name, birth_date, nationality, ...)`
- `ratings(player_id, year, position, overall, potential, <attributes...>)`
- `clubs(club_id, name, league_id, ...)`
- `squads(year, club_id, player_id, shirt_number, contract_end, value, wage)`
- `leagues(league_id, name, country, ...)`

Rough size, assuming ~18k players per edition x 25 editions = ~450k rating rows x ~40 attributes (1 byte each) = ~20 MB raw, less compressed **(my estimate, unverified)**. Small enough to ship and to query. At this shape and with year-based lookups, **SQLite (`rusqlite`) becomes the sensible choice** for the reference data, since it is relational, queryable by year, and inspectable. Load the chosen year into memory at new-game time; do not query SQLite inside the sim loop.

### How "grow as they really did" can work

- **Historical mode (default while data exists):** at each season rollover, a player's next ratings come from the dataset for the next year. Real careers replay.
- **Divergence:** once the world differs from history (you signed a youngster, an injury, a different club), real ratings are a *target curve*, not a script. Blend: simulated progression nudged toward the real curve.
- **After the last dataset year, or for players missing from later editions:** pure simulated progression (needs the M5 growth model anyway).
- Real fixtures, results and transfers are *not* in this data. Fixtures and AI transfers stay generated.

### Problems to plan for

- **Identity matching across years:** need one stable ID per real player. Some scraped datasets use a stable ID across editions **(unverified)**; if not, matching by name + birth date is error-prone.
- **Attribute sets change between editions.** Old editions likely have fewer or different attributes and fewer players **(unverified)**. Need a mapping layer to one internal attribute set, with derived values for gaps.
- **Missing data FIFA doesn't provide:** club budgets, finances, tactics, manager attributes. Must be derived or taken from another source (see the `information/` folder).
- **Legal risk.** Player names, likenesses and ratings from EA games are EA's (and licensors') content. Private use is one thing; shipping or selling a game that bundles it is another, and I'm not a lawyer. Real-name leagues need licences. Common workaround: ship the engine empty and let users import their own data, or ship fictionalised names. Decide this before building tooling around the data.
- **Data source:** I haven't researched where such data can be obtained or under what terms. Needs its own note in `information/`.

### Design consequence (cheap, do now)

Keep the engine independent of where the data came from: a `DataSource` boundary that returns "clubs, players, ratings for year Y". Generated fictional data and an imported real dataset then plug into the same place, and the legal decision can be made later without a rewrite.

## Open questions

- How large a save gets after 20+ seasons of match history. Cheap fix: store season summaries, not every match event, for old seasons.
- Whether we want a moddable starting-data file. Decide at M5 (save/load).
- Whether season/match history should move to SQLite. Revisit only if saves get slow or history queries get complicated.

## Sources

- [How to Install and Use Custom Databases in Football Manager](https://connect.usama.dev/blogs/43392/How-to-Install-and-Use-Custom-Databases-in-Football-Manager)
- [Install Football Manager Pre-Game Editor (SEGA support)](https://support.sega.com/hc/en-gb/articles/19393230285969-How-To)
- [Deciphering FM save files (GitHub topic fm20)](https://www.github.com/topics/fm20)
- [FIFA DB editing tutorials (soccergaming.com)](https://soccergaming.com/forums/threads/db-editing-tutorials-tips-and-tricks.6459905/post-6477166)
- [FIFA 20 default database thread](https://soccergaming.com/forums/threads/fifa-20-default-database.6467140/)
- [Sled vs RocksDB (Rust forum)](https://users.rust-lang.org/t/what-are-the-benefits-of-using-sled-vs-rocksdb/67103)
- [redb docs](https://docs.rs/crate/redb/0.4.0)
