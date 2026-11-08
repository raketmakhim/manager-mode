# Formations: which ones exist in real football, and which ones the game should offer

Researched 2026-11-07. The game picks each club's formation automatically (the one whose best eleven rates highest) and lets the user choose. This note is about which formations to define.

**How reliable this is.** I found real usage figures for the **Premier League only**. For the other four leagues I found no usable per-formation shares: a 2023/24 six-league study ([Springer, 2026](https://link.springer.com/article/10.1186/s13102-026-01657-1)) exists, but its page is behind a login and I could not read its tables. So the usage section is Premier League data plus qualitative statements; the formation shapes in section 3 are **our own design** (not copied from the game or any source).

## 1. What real usage looks like (Premier League)

From the league's own season reviews ([2024/25 review](https://www.premierleague.com/news/4323030), [October 2024 snapshot](https://www.premierleague.com/news/4147412)), times a formation was used as a team's starting shape in a season (760 team-matches):

| Formation | 2023/24 | Share | 2024/25 |
|---|---|---|---|
| 4-2-3-1 | 296 | 38.9% | 408 |
| 4-3-3 | 172 | 22.6% | 114 |
| 3-4-2-1 | 71 | 9.3% | 117 |
| 4-4-2 (classic) | 54 | 7.1% | 20 |
| 4-4-1-1 | 41 | | no figure |
| 3-5-2 | 41 | | no figure |
| 4-1-4-1 | no figure | | 27 |

Back four against back three (team-matches): 2020/21 70.0% / 30.0%, 2021/22 71.4% / 28.6%, 2022/23 79.3% / 20.7%, 2023/24 78.3% / 21.7%. In 2022/23 the 4-2-3-1 was used 261 times. The 2024/25 figures from the October snapshot are partial-season and are not used here.

Qualitative, from search results (not read in full, so treat as soft):
- Serie A is the outlier for three at the back: the 3-5-2 is played more there than in any other major league ([chart](https://datawrapper.dwcdn.net/AZzly/1)).
- A 2020/21 summary names the 4-3-3 and a 4-4-2 diamond as common in La Liga ([The Mastermind](https://themastermindsite.com/2021/07/19/quick-take-most-popular-formations-in-europes-top-5-leagues-2020-21/)).
- A 2013 Bleacher Report piece calls the 3-4-3 the least popular formation in Europe's top leagues. It is too old to count for the 2014 to 2023 range.
- A 2013-era WhoScored ranking (Bleacher Report) is the only cross-league share list I found, and it is too old.

## 2. What it means for us

- A club needs about **ten formations** to cover real use: four-at-the-back (4-2-3-1, 4-3-3, 4-4-2, 4-1-4-1, 4-4-1-1, a diamond) and three-at-the-back (3-4-2-1, 3-5-2, 3-4-3, 5-3-2).
- **Use changes by year.** The 4-4-2 was a main shape in 2014 and nearly gone by 2024; the 4-2-3-1 and 3-4-2-1 grew. The automatic pick avoids encoding that, because it chooses from the squad, not from fashion. If the formations ever need weights by season, it would be data in `Rules`-like tables, not code.
- Choosing by total rating can pick odd shapes. Starting set: each formation must have exactly one goalkeeper and no more than five defenders or four attackers; consider a small preference for common shapes.

## 3. Proposed set (our design)

Rows from the goalkeeper up the pitch, each row left to right, using our positions.

| Formation | Rows |
|---|---|
| 4-4-2 (have) | GK / LB CB CB RB / LM CM CM RM / ST ST |
| 4-3-3 (have) | GK / LB CB CB RB / CM CDM CM / LW ST RW |
| 4-2-3-1 (have) | GK / LB CB CB RB / CDM CDM / LM CAM RM / ST |
| 3-5-2 (have) | GK / CB CB CB / LWB CDM CM CAM RWB / ST ST |
| 4-1-4-1 | GK / LB CB CB RB / CDM / LM CM CM RM / ST |
| 4-4-1-1 | GK / LB CB CB RB / LM CM CM RM / CAM / ST |
| 4-4-2 diamond | GK / LB CB CB RB / CDM / CM CM / CAM / ST ST |
| 3-4-2-1 | GK / CB CB CB / LWB CM CM RWB / CAM CAM / ST |
| 3-4-3 | GK / CB CB CB / LWB CM CM RWB / LW ST RW |
| 5-3-2 | GK / LWB CB CB CB RWB / CM CDM CM / ST ST |

Not proposed yet: 4-3-2-1, 5-2-1-2, 3-4-1-2 and 4-5-1 (FC 24's own menu has many more; a search found no complete list, only that a source counts 53 formations for FC 24, which includes variants that differ only in instructions we do not model).

## Open questions

- Per-league and per-year shares for the other four leagues: needs the Springer study's tables, or FBref / WhoScored / Opta data.
- Whether wing-backs (LWB, RWB) should count as defenders or midfielders in shot weights; they are listed as defence in `positions!`.
- How big a gap in total rating should be needed before the automatic choice leaves the club's usual shape.

## 4. What was built (2026-11-07)

All ten formations are defined (`Formation::ALL` in `engine/src/lineup.rs`). `best_formation` picks the one whose best eleven has the highest total rating, the earlier one in the list winning a tie. `World::choose_formations` applies it to every club the user has not set, and runs when a career starts, when a new season starts and after the ratings advance. A club's formation is kept once the user picks one or changes its lineup; the Automatic button gives it back to the game.

FIFA 15 clubs, chosen automatically: 4-4-2 155, 4-2-3-1 132, 4-3-3 108, 4-1-2-1-2 (diamond) 73, 4-4-1-1 56, 4-1-4-1 30, 3-5-2 14, 3-4-3 7, 3-4-2-1 2, 5-3-2 1. That is more four-at-the-back and less three-at-the-back than real football (the Premier League was 78% back four in 2023/24, Serie A is the exception), and the diamond and 4-4-1-1 are likelier than real. Weights by league or year, or a preference for common shapes, would fix that; nothing does yet.

Effect on the simulation: big five over 40 seasons 2.76 goals, 44.1 / 24.4 / 31.5 %, points per match 2.38 / 2.09 / 1.81 / 0.64 (real 2.77, 44.4 / 25.1 / 30.5, 2.37 / 2.11 / 1.83 / 0.63), so the tuning holds. Barcelona now line up 4-3-3 and Messi scores 0.89 a match over six simulated seasons (the best of anyone).
