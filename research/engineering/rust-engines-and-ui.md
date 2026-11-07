# Rust game engines and UI toolkits: what fits us?

Researched 2026-11-06. Sources are mostly blogs and forum threads, so numbers are rough. No benchmarks were found.

## Our use case

Manager game, simulation only (no playable matches). The heavy part is the sim; the front end is menus, tables, squad lists and text. No real-time rendering, physics or animation needed. See [football-game-engines.md](football-game-engines.md).

## Game engines

| Engine | Style | Health (Sept 2026, secondary source) | Fit for us |
|---|---|---|---|
| **Bevy** | ECS, data-driven, code-first, no editor | ~48k GitHub stars, 0.19.1 (Aug 2026), ~7.3M downloads. README still says early stage. | Overkill. Built for real-time scenes. Its built-in UI is weak for table-heavy screens (not confirmed either way in sources). |
| **Fyrox** | Scene editor, 3D focus | ~9.5k stars, 1.0.1 (Mar 2026), ~72k downloads | No. Editor and 3D are things we don't need. |
| **macroquad** | Minimal 2D library | ~4.6k stars, 0.4.16 (Jul 2026), ~1.7M downloads | Only if we want a quick 2D prototype. Has no UI widgets. |
| **ggez** | 2D library | Status not verified | No. Same reason as macroquad. |

Finding: I could not find any Rust management/sim game built on Bevy. The few examples were small hobby projects.

## UI toolkits (better match than engines)

| Toolkit | Style | Notes from sources |
|---|---|---|
| **ratatui** | Terminal UI | Already in plan. Cheapest option, no windowing. Not a desktop GUI. |
| **egui** (via `eframe`) | Immediate mode | Quick to build tools and tables. Reports of higher CPU use than Iced. Immediate mode may not suit large apps. |
| **Iced** | Retained, Elm-style | Seen as more structured for full apps. Multi-window support was an open question. |
| **Slint** | Declarative | Praised as polished and efficient, separates UI from logic. Dual-licensed, so check terms. |

No source covers large tables or charts specifically.

## Recommendation

1. **Don't use a game engine.** Nothing we need is an engine feature. The `engine` crate stays plain Rust. Bevy's ECS could model players/clubs, but plain structs and `Vec`s are simpler and enough.
2. **Keep ratatui for M1-M3.** Fastest route to a playable season loop.
3. **When we want a graphical UI, try `egui` (`eframe`) first**, since it is the quickest way to get tables and lists. Compare Slint if we want a more polished look. Build the squad table in both and keep the one that feels better.
4. **Revisit Bevy only if** we add a 2D match view or animated visuals later (M6). Bevy plus `bevy_egui` exists for mixing in egui UI.

## Open questions

- How well egui, Slint and Iced handle big, sortable tables (e.g. 500+ players with filters). Needs a small prototype.
- Whether Slint's license suits us if the game is ever sold.

## Sources

- [Rust Game Engines in 2025 (GameFromScratch)](https://gamefromscratch.com/rust-game-engines-in-2025/)
- [5 Rust game engines to consider (LogRocket)](https://blog.logrocket.com/5-rust-game-engines-consider-next-project)
- [Rust game engines: Bevy, Fyrox, macroquad (pistack.xyz)](https://www.pistack.xyz/posts/2026-09-11-rust-game-engines-bevy-fyrox-macroquad/)
- [The best game engine based on Rust-Lang (Rust forum)](https://users.rust-lang.org/t/the-best-game-engine-based-on-rust-lang/101734)
- [Rust and GUI coding (Rust forum)](https://users.rust-lang.org/t/rust-and-gui-coding/134742)
- [Choosing the right Rust GUI library in 2025](https://redlib.hbubli.cc/r/rust/comments/1jveeid/choosing_the_right_rust_gui_library_in_2025_why)
- [Thanks for All the Frames: Rust GUI Observations](https://fooqux.com/article/2726)
