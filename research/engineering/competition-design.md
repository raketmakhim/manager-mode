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

## Order of work

1. `Stage` with two formats: `RoundRobin` (wrapping what exists) and single-leg `Knockout` with a seeded draw.
2. Make league fixtures come out of a stage, plus the shared standings function.
3. Rules-by-year lookup, with the top few countries written by hand.
4. League phase, split, groups and two-leg ties, each when a competition needs it.

## Open questions

- Exact stage interface (does the state machine read results from `World`, or get them passed in?).
- How a stage's entry rule refers to another competition (the qualification table).
- Whether the weekly calendar placement of knockout rounds needs preferred weeks (midweek), which needs a day-of-week field.
- Where the real per-country rules come from (see the open questions in competition-formats.md).
