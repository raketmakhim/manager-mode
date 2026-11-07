# Manager Mode

Football manager-mode game (like FIFA Career Mode) in Rust. Simulation only: the user never plays a match.

- Read [README.md](README.md) for the pitch and [PLAN.md](PLAN.md) for milestones (M1-M6) and decisions.
- Planned layout: Cargo workspace with `crates/engine` (no UI), `crates/tui`, `crates/simtool`.
- Rules: engine has no UI imports; every sim takes an explicit seeded RNG; all data comes through the `DataSource` boundary (generated fictional data or the historical SQLite dataset). The real dataset is never committed to the repo.
- Research notes live in `research/`. Update PLAN.md if research changes a decision.
