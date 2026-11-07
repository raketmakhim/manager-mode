# Design: how to model competitions that differ

Designed 2026-11-07. This is a design decision based on [competition-formats.md](../game-features/competition-formats.md) and [fixture-rescheduling.md](../game-features/fixture-rescheduling.md), not tested code. The stage interface is the part most likely to change once the first knockout cup exists.

## The problem

Competitions differ in shape (league, knockout, groups, league phase, split), legs, how a tie ends, who plays, tie-break order, and **the season they apply to**: the same competition has different rules in 2014 and 2024. The data (FIFA 15 to FC 24) lets the player start in any of those years, so the rules must be looked up by season.

## Decision: stages in data, not a type per kind of competition

League, cup and "league + cup" are not three kinds of thing. They are lists of the same pieces, called **stages**:

| Competition | As stages |
|---|---|
| Premier League | one round-robin |
| FA Cup | one single-leg knockout |
| World Cup / Champions League before 2024 | groups (several small round-robins), then knockout |
| Champions League from 2024 | league phase, play-off knockout, knockout |
| Scottish Premiership | round-robin, then a split into two round-robins |
| Liga MX | (round-robin then knockout), twice a year |
| Belgian Pro League (until 2026) | round-robin, then playoffs |

Three base types would fail on the Champions League and Liga MX. A list of stages handles both with no new type.

## Shape (a sketch)

```rust
struct Competition { id, name, country, rules: Vec<(YearRange, Rules)> }   // rules by season
struct Rules { stages: Vec<Stage>, tie_breakers: Vec<TieBreak> }
struct Stage { format: Format, entry: Entry }       // entry: all clubs / top N of previous stage / ...
enum Format {
    RoundRobin { legs: u8 },
    Knockout { legs: u8, extra_time: bool, penalties: bool, replays: bool, seeding: Seeding },
    LeaguePhase { opponents: u8 },
}
```

Each format runs as a small state machine answering the same questions: which fixtures are ready now, is the stage finished, who advances. A round-robin answers the first at once. A knockout answers round by round, after the last round is played.

## What is shared (the reuse)

- **Pairing functions**: pure functions from clubs to pairings. `round_robin` is the first; the knockout draw and Swiss pairing follow the same pattern.
- **Calendar placement**: `World::first_free_week` and the 3-matches-per-week rule, used by every format. No format knows about the limit.
- **Standings**: one function that builds a table from results and the tie-breaker list, used by leagues, groups and the league phase.
- **Qualification table**: "country X, finishing position N, season Y goes to competition Z at stage S". Separate data, not code inside a competition.
- **Rules by year**: a lookup from (competition, season) to its `Rules`.

## Rules for the code

- Country differences live in data. No `match country` in the code.
- Use an enum for formats and a `match` over it, so the compiler checks every format is handled. Add a trait only if something outside the engine, such as a mod, must add formats.
- Keep pairing, calendar, standings and qualification as separate pieces (single responsibility).
- Rules start as Rust constants. Move them to a data file once there are dozens.
- Build a format only when a competition uses it. No Swiss or split code until something needs it.

## What the research added

- **Cups** (four checked) differ by entry round, legs per round, how a level tie ends per round, draw style, home-advantage rule and the European place for the winner, and several changed inside 2014 to 2023 (Copa del Rey 2019/20, EFL Cup 2018/19). These are the `Knockout` parameters.
- **European places are computed**, not listed: store each season's country ranking order and the band rules, add the title-holder extra place and bans. See [european-places-by-season.md](../game-features/european-places-by-season.md).
- **Rescheduling** can reuse `first_free_week`; European dates are fixed first. See [fixture-rescheduling.md](../game-features/fixture-rescheduling.md).
- **League calendars need gaps** (international and winter breaks), which today's "round N is week N" does not allow.

## Defaults for competitions we have not researched

The data has up to about 56 leagues per edition and we have researched only a handful of countries. Anything not covered uses a **default**, set in one place and overridden per country and season by the researched rules.

| Competition | Default (a guess, not a real format) |
|---|---|
| League | double round-robin, as today |
| Domestic cup | one single-leg knockout, called "<Country> Cup" |
| European places | by the country's ranking position, using the rank bands in [european-places-by-season.md](../game-features/european-places-by-season.md); a country outside the researched top 15 gets the one-team and three-team bands |

**Default cup parameters**
- **Entrants:** every club in the country's leagues in that edition (all divisions we have for it).
- **Legs:** one leg in every round, including the semi-finals; the final is at a neutral venue.
- **Tie ends:** extra time, then penalties. No replays, no away goals.
- **Draw:** random, round by round, from the seeded generator. A club from a lower division hosts; if both are in the same division, the first club drawn hosts.
- **Byes:** when the number of clubs is not a power of two, the extra clubs get a bye in round 1, given to the best-ranked clubs (by league division, then club rating).
- **Too small:** a country with fewer than 4 clubs in the data gets no cup.
- **Winner:** takes the country's cup-winner European place if it has one; if the winner is already in Europe, the place passes to the best league finisher not yet qualified (this rule matched all four cups we checked).
- **Calendar:** each round placed with `first_free_week`, so the 3-matches-per-week limit holds.

**Rules for the defaults**
- The default is a real `Rules` value, like any researched competition, not special-case code. A researched competition replaces it by registering its own `Rules` for that (competition, season).
- The game should label it as a generic cup so a player does not mistake it for the real one.
- Researching a country means adding its `Rules`; nothing else changes.

## Stats, competition views and match detail tiers

Designed 2026-11-07. Requirements: top scorers, assist providers, clean sheets and average ratings **per competition**; results, tables and team progression easy to view for every competition.

### Match reports and stats
- **The match report is the source of truth.** Each played match stores its outcome and its goal events (scorer, assister, minute). Fuller reports add cards, substitutions, minutes and player ratings.
- **Outcome is richer than a score:** goals, extra-time goals and a shootout result (shootout goals never count as scorer stats). Two-legged ties also need an aggregate. Today `Fixture.result` is only `(u8, u8)` and must change before the simulation writes results.
- **Totals are keyed by (season, competition, player)** and updated when a match is recorded; they can be rebuilt from reports. One competition's table reads one key; "all competitions" sums the keys.
- **One generic leaderboard** takes a stat, a competition, a season and a minimum number of games. Top scorers, assists, clean sheets and ratings all use it, so a new competition gets its tables for free.
- **Competition-season is the natural record:** a competition in one season (for example "FA Cup 2016/17") holds its rules, stages, results and stats.
- **Views come from stored data:** a league table is computed from results up to a chosen round (so position-by-matchday needs nothing extra); a cup bracket needs every round's pairings kept after they are played.
- **Stat definitions to settle once:** own goals, what counts as an assist, whether a clean sheet needs the goalkeeper to play 60 minutes, whether extra-time goals and qualifying rounds count, tie-break order on a leaderboard, and a minimum number of games for averages.
- **Stats follow the player, not the club.** A match report records which club he played for.
- **Invariants to test:** a match's goal events add up to its result; goals scored equal goals conceded across a competition; assists never exceed goals; the same seed gives the same top scorers.

### Two levels of match detail
The league has up to about 56 leagues per edition, around 10,000 to 11,000 league matches a season. Full detail for all of them is wasted work, so each competition has a **detail level**, set in its rules data (a setting, not code):

| Level | Used for | What is produced |
|---|---|---|
| **Full** | the top flights of England, Spain, Germany, Italy and France; the Champions League; every competition the user's club is in | minute-by-minute events, cards, substitutions, minutes, player ratings, commentary |
| **Cheap** | everything else | score, scorers and assisters; clean sheets follow from the score; no cards or substitutions; ratings approximated from the result and goal involvement |

**Cheap simulation (a sketch).** It reuses the full simulation's team-strength function, so both levels agree statistically:
1. Each side's expected goals come from its attack against the other's defence, plus home advantage; goals are drawn from the seeded generator.
2. Each goal picks a scorer, weighted by position and attributes (finishing, positioning, shot power) of the club's usual eleven; most goals get an assister, weighted by passing and vision, who is never the scorer.
3. A clean sheet goes to the goalkeeper of that eleven when the opponent scores nothing.
4. Lineups are the club's auto-picked eleven, with no rotation or in-season injuries at this level.
5. Cost is two random draws plus about 2.7 weighted picks per match, with no minute loop.

**Calibration test.** `simtool` runs the same fixtures through both levels and checks that goals per match, home-win rate and the spread of scorers agree within a tolerance. A top scorer in a 20-club league should land around 20 to 35 goals (my general knowledge, not from a source; calibrate against real figures when found).

**Optional: view on demand.** A cheap match stores only its score and goal events. If the user opens it, plausible full events can be generated to fit that score, seeded from the match id so the result is always the same. This costs nothing until a match is viewed.

### Estimated saving (from the FIFA 15 to FC 24 data)
- League matches per season: 9,838 (FIFA 15), 11,568 (FIFA 20), 11,238 (FC 24). The five top flights are 1,750 to 1,830 of them (16 to 19%).
- Full detail covers the top five flights, the Champions League (about 190 matches in the 36-team format) and the user's competitions: about **2,000 to 2,400 matches, 18 to 22%**. The other 78 to 82% are cheap.
- If a cheap match costs 10 to 100 times less than a full one, total match work falls to about 21 to 28% of full cost, a **saving of about 72 to 79%**. The ceiling is about 80%, because the full-detail 20% still costs full price.
- Storage: a goal event is a few bytes, so a cheap match report is a small fraction of a full one. Full-report sizes are an assumption; **measure both costs once the match simulation exists.**
- Not reduced by the tiers: player development (the dataset supplies real ratings at rollover for cheap leagues), transfers, lineup picking and the save file's players.

### Open questions
- Where to get real goals and assists per season to calibrate the cheap simulation (the FIFA data has none).
- Whether cheap leagues need injuries and suspensions at all.
- Whether an old season's full reports are compacted to summaries (final table, winner, top scorers) when it ends, to keep saves small.
- How the detail level changes during a career (the user's club moves league, or a club in a cheap league wins a place in the Champions League).

## Order of work

1. **Done (2026-11-07):** `crates/engine/src/competition/` with `Competition`, `Rules`, `Stage`, `Format::RoundRobin` and `Entry::AllClubs`; `round_robin` and the calendar functions moved in unchanged; a richer match `Outcome` (full time, extra time, shootout) in `outcome.rs`. Built simpler than the sketch above: a `Competition` holds the `Rules` already resolved for its season (the year lookup comes in step 4), and `RoundRobin` has no `legs` parameter until a competition needs a single round-robin.
2. **Done (2026-11-07):** `Format::Knockout` in `competition/knockout.rs`: `draw_round` (a preliminary round for the extra clubs, byes for the rest), `World::advance_cups` (draws the next round once the last is played; survivors are worked out from results), a default cup per country. Also `World::start_season` as the one game-setup call (`load_edition` stays real data only), our own seeded `Rng`, and `week_date`. Not done: seeding, lower-division hosts, two legs, real cup dates (cup rounds take the first week with room, marked `shortcut:` in the code).
3. Make league fixtures come out of a stage state machine, plus the shared standings function.
4. Rules-by-year lookup, with the top few countries written by hand.
5. League phase, split, groups and two-leg ties, each when a competition needs it.

## Open questions

- Exact stage interface (does the state machine read results from `World`, or get them passed in?).
- How a stage's entry rule refers to another competition (the qualification table).
- Whether the weekly calendar placement of knockout rounds needs preferred weeks (midweek), which needs a day-of-week field.
- Where the real per-country rules come from. Covered so far: [domestic-cup-formats.md](../game-features/domestic-cup-formats.md) (four cups) and [european-places-by-season.md](../game-features/european-places-by-season.md) (places per country, 2014/15 to 2023/24). Everything else is listed under the open questions in competition-formats.md.
