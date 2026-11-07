# What engines do football games use, and does Rust help us?

Researched 2026-11-06. Items marked **(unverified)** came from general knowledge, not from a source I found.

## What the big games use

| Game | Engine | Language | Notes |
|---|---|---|---|
| EA Sports FC (ex-FIFA) | Frostbite (built by DICE, first used in Battlefield) | C++ **(unverified, standard for Frostbite)** | Career Mode "Deeper Simulation" in FC 26 lets you simulate up to 5 extra leagues in the background. |
| Football Manager 26 | Unity for rendering and animation, plus an in-house match engine | Unity is C#. The in-house engine's language wasn't found. Older FM engines were probably C++ **(unverified)**. | FM25 was cancelled because the Unity switch was harder than expected. FM26 launched buggy and with a poor UI. Sports Interactive has now stopped FM26 updates to focus on FM27. |

Takeaways:

- **Yes, the big ones are C++-family**, but this is mostly because they are tied to a 3D engine (Frostbite or Unity), not because simulation needs C++.
- **A manager game's core is separate from the 3D engine.** Sports Interactive's job posting describes an in-house engine that simulates and visualises matches and **must be deterministic**. Unity only draws the result.
- **FM26 match model**: every player and official makes a decision every quarter of a second (a "slice"). FM26 also added Hawk-Eye skeletal tracking data for animations.
- **Lesson from FM25**: switching engines mid-project is what sank them. We are choosing our stack up front, so we avoid that.

## Does Rust give us an advantage?

For this project, mostly yes.

**Where Rust helps**

- **Speed for bulk simulation.** Rust is compiled and has no garbage collector, so it runs in the same performance class as C++. Running 100k matches or many seasons to balance the game is the main reason we picked it.
- **Easy parallelism.** `rayon` plus Rust's compiler checks make "run N seasons across all cores" safe and short. Data-race bugs in a parallel sim are what the compiler catches for us.
- **Determinism and replays.** A seeded RNG passed explicitly to every sim function gives exact replays. This is a design choice more than a Rust feature, but Rust's lack of hidden global state makes it easy to keep.
- **Good fit for the data layer.** `serde` makes JSON saves nearly free.

**Where Rust does not help or costs us**

- **No raw speed win over C++.** Benchmarks are mixed. Don't expect a "Rust is faster" result, just "as fast, with fewer crash and memory bugs".
- **Graphical ecosystem is younger.** Bevy is the main Rust engine (0.19 released June 2026, 48k GitHub stars) but its own README still says it is early stage. Unity, Unreal and Godot are all far more mature. This is why the plan starts with a terminal UI and keeps the engine crate independent.
- **Slower to write.** The borrow checker costs iteration time compared with C#. Most of the cost is paid once, in the data model design.
- **Fewer ready-made UI and tooling choices** than Unity or web, if we later want a polished graphical front end.

## Rust vs C# performance (added 2026-11-06)

No authoritative, current head-to-head exists in what I found. The evidence is anecdotal:

- Typical range: Rust is **roughly 1.2x to 3x faster** than modern .NET on CPU-heavy code, and uses **2-5x less memory**. Gaps are larger when the code allocates a lot, because of garbage collection.
- Benchmarks Game: one commenter says the fastest C# entry is ~6x slower than the best Rust/C++, but a rewrite on .NET 9 got within ~20% once the arena-allocator trick was used. One person's claim.
- Microsoft's own Rust-for-C# guide shows a parallel Mandelbrot: ~1.1 s / ~200 MB (Rust + Rayon) vs ~2.3 s / ~500 MB (C#). Illustrative only.
- Real app port (Markdown blog, Feb 2026): Rust used ~4.6x less memory and had 2-8x lower latency, but the author says the latency difference is imperceptible to users.
- Godot test: when most code calls the engine API, C#, GDScript and Rust perform about the same.
- Not found: any C# vs Rust benchmark of a Monte Carlo or football-style simulation.

Sources: [HN thread](https://hn.jakechampion.name/item/45197608), [Rust for C# Programmers (Microsoft)](https://microsoft.github.io/RustTraining/csharp-book/ch16-1-performance-comparison-and-migration.html), [Markdown blog benchmark](https://hamy.xyz/blog/2026-02_rust-vs-csharp-blog-benchmark), [Godot benchmark](https://awesome.ecosyste.ms/projects/github.com%2Fjerryshell%2Fgodot-survivors-benchmark).

## Recommendation

Keep the plan as is: Rust `engine` crate with no UI, `tui` first, `simtool` for bulk balance runs. If we later want a graphical UI, we can pick `egui` or Bevy, or even embed the engine in another front end, without rewriting the simulation. Do not pick a front end before M3 is playable.

## Open questions

- Which language the FM match engine and the Frostbite career-mode simulation are actually written in. Developer talks, job postings or GDC slides would settle it.
- Rust-specific benchmark for our workload. Cheapest way to answer: build the M2 match sim and time it in `simtool`.

## Sources

- [Football Manager 26 first look (Shortlist)](https://www.shortlist.com/tech/football-manager-26-first-look-shows-off-the-beautiful-game)
- [Hawk-Eye and Sports Interactive](https://www.hawkeyeinnovations.com/news/4369378/hawkeye-and-sports-interactive-team-up-to-power-football-managers-new-era)
- [Sports Interactive job listing: Senior Software Engineer, Match AI](https://hitmarker.net/jobs/sports-interactive-senior-software-engineer-match-ai-1584472)
- [Football Manager 26's Chaotic Kickoff: Unity gamble (finalboss.io)](https://finalboss.io/football-manager-26s-chaotic-kickoff-unity-gamble-missing)
- [Switch to Unity: FM25 falls through (heise)](https://heise.de/-10273919)
- [Sports Interactive ends FM26 updates](https://www.sportsgamersonline.com/games/soccer/sports-interactive-ends-fm26-updates-fm27/amp/)
- [What is Deeper Simulation in EA FC 26 (Operation Sports)](https://www.operationsports.com/what-is-deeper-simulation-in-ea-fc-26-explained)
- [EA FC 24 deep dive (mp1st)](https://mp1st.com/news/ea-sports-fc-24-matchday-experience-details-and-upgrades-showcased-in-gameplay-deep-dive)
- [Bevy news](https://bevy.org/news/)
- [Rust game engines: Bevy, Fyrox, macroquad (pistack.xyz, secondary source)](https://www.pistack.xyz/posts/2026-09-11-rust-game-engines-bevy-fyrox-macroquad/)
