# Manager Mode

Football manager-mode game (like FIFA Career Mode) in Rust. Simulation only: the user never plays a match.

- Read [README.md](README.md) for the pitch and [PLAN.md](PLAN.md) for milestones (M1-M6) and decisions.
- Layout: Cargo workspace. `crates/import` builds `history.db` from the CSVs, `crates/engine` is the game logic (no UI), `crates/gui` is the egui window, `crates/tui` the terminal view. `crates/simtool` is planned for M2.
- Rules: engine has no UI imports; every sim takes an explicit seeded RNG; the game gets its data through `engine::load_edition` (a trait only once a second source exists). The real data (CSVs and `history.db`) is never committed to the repo.
- Calendar rule: a club plays at most 3 matches in any week; `World::calendar_problems()` must stay empty.
- Run `cargo test`; tests that need `history.db` skip themselves if it is missing.
- Research notes live in `research/`. Update PLAN.md if research changes a decision.
