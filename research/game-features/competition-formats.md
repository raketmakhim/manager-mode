# Competition formats: leagues, cups, and how European places are decided

Researched 2026-11-07. Sources are news and explainer pages found by web search; most are secondary. Items marked **(unverified)** are from general knowledge or a single weak source. The biggest gap: I found **no source with the number of European places per country**, and no detail on the DFB-Pokal, Coppa Italia, Copa del Rey or EFL Cup formats.

## The main finding for our design: formats change by year

Our data covers editions FIFA 15 to FC 24 (2014/15 to 2023/24), and the game lets you start in any of them. Several formats changed inside or just after that window, so **a competition's rules have to depend on the season you start in**, not be fixed in code:

| Competition | What changed | Source |
|---|---|---|
| Champions League | 2024/25: group stage replaced by a single 36-team league phase. Before that: 32 teams in 8 groups of 4 **(unverified)** | below |
| FA Cup | Replays scrapped from the first round proper starting 2024/25; before that they existed (they were suspended, then returned in 2021/22) | [Flashscore](https://www.flashscore.co.uk/news/football-fa-cup-fa-cup-replays-to-be-scrapped-from-first-round-onwards-next-season/lStMTeWh/), [Sports Mole](https://www.sportsmole.co.uk/football/fa-cup/news/fa-cup-replays-to-return-for-2021-22-season_455282.html) |
| Belgian Pro League | Playoff format from 2009/10; abolished from 2026/27 (back to 18 teams, 34 games, no playoffs) | [Inside World Football](https://www.insideworldfootball.com/2025/06/17/belgian-football-association-abolishes-playoffs-pro-league-2026-27-season/) |
| Liga MX | Two tournaments a year (Apertura, Clausura) each ending in a playoff; a play-in round was added recently; promotion and relegation paused for six seasons from 2020 | [World Soccer Talk](https://worldsoccertalk.com/news/liga-mx-schedule-adopts-new-playoffs-structure-20231126-WST-472800.html), [Sporting KC](https://www.sportingkc.com/news/champions-league-countdown-13-days-explaining-unique-structure-liga-mx) |

## Formats seen so far

1. **Double round-robin league.** What we have today: everyone plays everyone home and away. Standard in most countries.
2. **League with a split.** Scottish Premiership: 12 teams, split into two groups of six after a set number of games (33 in the more detailed sources, 30 in one; they disagree), then five more games inside each group. The bottom group includes a relegation play-off. [Flashscore](https://www.flashscore.com/news/europe-s-most-unique-domestic-league-formats-part-1/lMEUFI8a), [StatsUltra](https://statsultra.com/europes-alternative-football-league-formats-explained/)
3. **League then playoffs.** Belgium (until 2026), Greece (three groups after the regular season), Malta (opening round then closing round), Liga MX (liguilla knockout for the top 8). [Flashscore](https://www.flashscore.com/news/europe-s-most-unique-domestic-league-formats-part-1/lMEUFI8a)
4. **Multiple round-robins.** Croatia: 10 teams play each other four times, 36 games each. Same source as above.
5. **Two tournaments in one season.** Apertura and Clausura (Latin America). Some countries have the two winners play each other for the title. [Wikipedia](https://www.wikipedia.org/wiki/Apertura)
6. **Single-leg knockout cup.** Coupe de France: draws go to extra time then penalties; when teams from different divisions are drawn together, the lower-ranked team hosts. [Groundhopper Guides](https://groundhopperguides.com/?p=52756)
7. **Knockout cup with some two-legged ties.** Copa del Rey: one comparison says two legs only in the semi-finals, 7 rounds, away goals don't count (garbled source, **unverified**; see [domestic-cup-formats.md](domestic-cup-formats.md) for the checked version). [Football Handbook](https://footballhandbook.com/fa-cup-vs-copa-del-rey/) The EFL Cup semi-finals were reported as reverting to two legs (no date given).
8. **European "league phase" (Swiss-style).** Champions League since 2024/25: 36 teams in one table, **8 matches each (4 home, 4 away) against 8 different opponents**, not everyone against everyone. Top 8 go straight to the round of 16; 9th to 24th play a knockout play-off for the other 8 places; 25th to 36th are out of Europe (no drop into the Europa League). Tie-breakers: goals scored by league-phase opponents, then disciplinary points, then club coefficient. [Flashscore](https://www.flashscore.co.uk/news/soccer-conference-league-no-groups-and-36-teams-the-current-champions-league-format-explained/MeRhUwjB/), [ESPN](https://global.espn.com/football/story/_/id/41040476/champions-league-draw-how-new-format-works), [90min](https://www.90min.com/how-champions-league-table-works)
9. **Not researched:** group stage plus knockout (World Cup style), promotion/relegation play-offs (English Championship, Eredivisie), MLS conference playoffs. I searched for these and got no usable results.

## How European places are decided

- **Each country's number of places comes from UEFA's association ranking** (a coefficient built from five seasons of results). The ranking sets each country's position in the "access list", which also decides which round its clubs enter. [UEFA CL regulations, Art. 3](https://documents.uefa.com/r/Regulations-of-the-UEFA-Champions-League-2025/26/Article-3-Entries-for-the-competition-Online), [UEFA 2011/12 article](https://uefa.com/uefachampionsleague/news/01f1-0e7845105196-595a1c108040-1000--2011-12-uefa-champions-league-access-list)
- **Clubs get in by finishing position in their league** (top N), plus other routes below. The exact N per country is **not in my sources**.
- **2024/25 expansion (32 to 36 teams):** one extra place to the third-placed club of the country ranked 5th, and two places to the countries with the best collective results the previous season (total points divided by number of clubs). Club coefficients stopped being used for access. [UEFA announcement](https://www.uefa.com/news-media/news/0275-151c779310c3-b92bbf0d24f9-1000--format-access-list-for-2024-25-approved/)
- **European Performance Spot (EPS):** goes to the club that finishes highest in its league among clubs that did not reach the Champions League league phase. If that club qualifies another way, the spot passes down. [UEFA regulations](https://documents.uefa.com/r/Regulations-of-the-UEFA-Champions-League-2025/26/Article-3-Entries-for-the-competition-Online)
- **Europa League and Conference League:** places come from domestic cup winners and lower league positions from the top-ranked countries, plus clubs dropping down from earlier rounds of the higher competitions. The Conference League winner gets a Europa League place if they didn't earn one otherwise. [UEFA Conference League 2026/27](https://www.uefa.com/uefaconferenceleague/news/02a6-20d57d15f093-a90cf54c928f-1000--2026-27-conference-league-teams-dates-draws-format-final/)
- **UEFA publishes a circular each season** with the final adjusted access list after the previous season's results. So the places can change year to year.

## What this means for our engine

- **Competition kinds we will need**, beyond today's `League` and the planned `Cup`: a league with a split or playoff stage, a Swiss-style "league phase", knockout rounds with one or two legs, and two-tournament seasons. Each should be a `format` with parameters (legs, seeding, tie-breaks, extra time, replays), not a new hard-coded type per country.
- **Rules by season.** The same competition has different rules in 2014 and 2023, so the format is looked up by (competition, year).
- **Qualification is a table, not logic:** for each country and season, "finishing position N goes to competition X, entering at round Y". Start with a small hand-written table for the top few countries; the rest can use a default.
- **Knockout rounds can't be generated at the start.** Each round is drawn after the previous one is played (as already noted in PLAN.md), and a draw needs seeding rules (Coupe de France hosts the lower division; Champions League seeds by coefficient).
- **Our current league calendar has no gaps.** League round N is in week N, with no international breaks or winter breaks. Real calendars leave gaps, and European matchdays are fixed first (UEFA dates are binding, see [fixture-rescheduling.md](fixture-rescheduling.md)).

## Open questions

- Number of European places per country, per season 2014/15 to 2023/24: **now covered in [european-places-by-season.md](european-places-by-season.md)** (rank bands plus each season's country ranking). Still open there: country rankings below rank 15 and the entry stage per place.
- Formats of the DFB-Pokal, Coppa Italia, Copa del Rey and EFL Cup: **now covered in [domestic-cup-formats.md](domestic-cup-formats.md)**. Still open: Scottish cups and every other country's cup.
- Pre-2024 Champions League and Europa League formats and calendar (needed for 2014 to 2023 starts).
- Promotion and relegation: how many go up and down, and any play-offs, per league.
- Which of the 38 to 56 leagues in the data matter. Most are small and could just be a plain league.
- Tie-break rules for league tables (goal difference, head-to-head, and so on) per country.

## Sources

- [Flashscore: current Champions League format](https://www.flashscore.co.uk/news/soccer-conference-league-no-groups-and-36-teams-the-current-champions-league-format-explained/MeRhUwjB/)
- [ESPN: Champions League draw, how the new format works](https://global.espn.com/football/story/_/id/41040476/champions-league-draw-how-new-format-works)
- [90min: how the Champions League table works](https://www.90min.com/how-champions-league-table-works)
- [UEFA: format and access list for 2024/25 approved](https://www.uefa.com/news-media/news/0275-151c779310c3-b92bbf0d24f9-1000--format-access-list-for-2024-25-approved/)
- [UEFA Champions League regulations 2025/26, Article 3](https://documents.uefa.com/r/Regulations-of-the-UEFA-Champions-League-2025/26/Article-3-Entries-for-the-competition-Online)
- [UEFA: 2026/27 Conference League teams, dates, format](https://www.uefa.com/uefaconferenceleague/news/02a6-20d57d15f093-a90cf54c928f-1000--2026-27-conference-league-teams-dates-draws-format-final/)
- [Flashscore: Europe's unique domestic league formats](https://www.flashscore.com/news/europe-s-most-unique-domestic-league-formats-part-1/lMEUFI8a)
- [StatsUltra: alternative league formats](https://statsultra.com/europes-alternative-football-league-formats-explained/)
- [Inside World Football: Belgium abolishes playoffs](https://www.insideworldfootball.com/2025/06/17/belgian-football-association-abolishes-playoffs-pro-league-2026-27-season/)
- [World Soccer Talk: Liga MX playoffs](https://worldsoccertalk.com/news/liga-mx-schedule-adopts-new-playoffs-structure-20231126-WST-472800.html)
- [Sporting KC: Liga MX structure](https://www.sportingkc.com/news/champions-league-countdown-13-days-explaining-unique-structure-liga-mx)
- [Wikipedia: Apertura and Clausura](https://www.wikipedia.org/wiki/Apertura)
- [Flashscore: FA Cup replays scrapped](https://www.flashscore.co.uk/news/football-fa-cup-fa-cup-replays-to-be-scrapped-from-first-round-onwards-next-season/lStMTeWh/)
- [Groundhopper Guides: Coupe de France](https://groundhopperguides.com/?p=52756)
- [Football Handbook: FA Cup vs Copa del Rey](https://footballhandbook.com/fa-cup-vs-copa-del-rey/)
