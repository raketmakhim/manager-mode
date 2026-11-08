# Game features research

What FIFA/EA FC Career Mode and Football Manager offer (squad, tactics, transfers, finance, training, youth, objectives), and which we include in v1 vs later.

Done: `match-simulation-methods.md` (how matches are simulated, real shot, card and corner rates, a shot-based design for the full tier), `match-ratings.md` (how match ratings are built, the design and the rating module built from it), `simulation-balance.md` (real big-five results against the match simulation, with `real_league_stats.py`), `overall-rating-formula.md` (how overall is built from attributes, weights per position, with `fit_overall.py`), `competition-formats.md` (league and cup formats, European places), `fixture-rescheduling.md` (how fixtures are made and moved). `domestic-cup-formats.md` (DFB-Pokal, Coppa Italia, Copa del Rey, EFL Cup). `european-places-by-season.md` (European places per country, 2014/15 to 2023/24). `formations.md` (real formation use in the Premier League, a proposed set of ten formations as rows). `fitness.md` (tiring in a match, recovery, injuries: real figures and a proposed design, nothing built). `experience.md` (how real ratings change with age, how EA FC grows players, a proposed XP design with `age_curves.py`, nothing built).

Keep scope in mind: simulation only, M1-M5 first. Tag each feature `v1`, `later` or `skip`.
