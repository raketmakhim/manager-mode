# Manager Mode

Football manager-mode game (like FIFA Career Mode) in Rust. Simulation only: the user never plays a match.

- Read [README.md](README.md) for the pitch and [PLAN.md](PLAN.md) for milestones (M1-M6) and decisions.
- Layout: Cargo workspace. `crates/import` builds `history.db` from the CSVs, `crates/engine` is the game logic (no UI), `crates/gui` is the egui window, `crates/tui` the terminal view. `dev-tools/simtool` simulates many seasons and prints balance numbers (`cargo run -p simtool --release`). Tools for developers live in `dev-tools/`, the game itself in `crates/`.
- Rules: engine has no UI imports; every sim takes an explicit seeded `engine::Rng` (our own, not the `rand` crate); a new game is `load_edition` then `World::start_season`; the game gets its data through `engine::load_edition` (a trait only once a second source exists). The real data (CSVs and `history.db`) is never committed to the repo.
- Calendar rule: a club plays at most 3 matches in any week; `World::calendar_problems()` must stay empty.
- Competition rules depend on the season (formats changed between 2014 and 2023) and live in data. No per-country `match` in code. Design: `research/engineering/competition-design.md`.
- Run `cargo test`; tests that need `history.db` skip themselves if it is missing.
- Research notes live in `research/`. Update PLAN.md if research changes a decision.

## How we write code here

Easy to read, no repeated code, nothing clever.

- **One place per fact.** A name, rule or number is defined once and everything else points to it (see the `attrs!` list in `engine/src/attrs.rs`). If a second copy appears, stop and share it.
- **Wait for the third use before abstracting.** Two similar pieces are fine. A trait or generic comes only when a third use shows what varies. Say in a comment when to upgrade (`shortcut: ..., upgrade when ...`).
- **Pure functions at the core.** Pairing, draws, standings and match simulation take plain inputs (clubs, results, a seeded RNG) and return plain outputs. They do not touch `World`; a thin layer applies the results to it.
- **Differences are data, not code.** No `if country == ...`. A country or season differs by its `Rules` value.
- **Enums and `match`, not trait objects or deep generics.** The compiler then lists every place to update. Each format's logic lives in its own file; the `match` only dispatches.
- **Small public surface.** `lib.rs` re-exports only what other crates need. A feature lives in its own module and adds its `impl World` methods there, not in `model.rs`, which stays plain data; keep those methods few and read-mostly.
- **Short functions named for their job.** Comments explain why, not what. Every public item has a one-line doc with its unit (euros per week, years, 0 to 99).
- **One pattern per recurring job.** Randomness is always an explicit seeded RNG passed in. Database columns are read by name. Errors use one style.
- **Tests state rules, not output.** Prefer properties ("no club plays twice in a round", `calendar_problems()` is empty) that hold for every format. Exact numbers only as a few anchors (64 clubs give 63 matches).
- **Small steps.** Each change does one thing and leaves all tests passing.
- **Docs stay lean.** `research/` notes hold findings, decisions and why; the code holds numbers and what it does. No changelog (git has it), no constants copied from code (point to the constant), no superseded plans (delete them, do not strike them through), no restating of what a module doc already says. `PLAN.md` is milestones and decisions only. A change that makes a doc wrong updates that doc in the same commit.

Before every commit run `cargo clippy --workspace --all-targets` and `cargo test`. The `OUTFIELD` and `KEEPER` lists and the `Position` enum use blank lines between groups on purpose, so do not run `cargo fmt` over `engine/src/attrs.rs` and `model.rs` until `#[rustfmt::skip]` is added to them (`overall.rs` already has it).

- **Line endings are LF** (`.gitattributes`). When a script rewrites a text file on Windows, open it with `newline="\n"` (Python) or write bytes, because the default turns every line into CRLF and makes the whole file show as changed in `git diff`.
