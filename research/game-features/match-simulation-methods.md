# How football matches are simulated, and what our full-detail engine should do

Researched 2026-11-07. The **cheap** tier (`engine/src/sim/cheap.rs`) draws two goal counts from team strengths (Poisson with a Dixon-Coles correction), tuned to the real big-five tables. The **full** tier is built (2026-11-07, `engine/src/sim/shot_based.rs`, tuning and results in [simulation-balance.md](simulation-balance.md)) and currently plays every competition; it follows the design in section 2, with events, scorers, assisters, cards, corners and possession. Match ratings, substitutions, penalties and injuries are still to do. This note is the research and design behind it.

**How reliable this is.** Search results were thin on formulas; the pages I could read in full are named below. Where a number is not published by anyone (rating weights, injury rates) the note says so rather than guessing. The event rates in section 3 are computed by us from real data, not taken from a source.

## 1. The ways people do it

| Method | What it models | Who uses it | Good for us? |
|---|---|---|---|
| **Independent Poisson goals** | Each team's goals as a Poisson count from an attack and a defence strength | Maher 1982; the base of most forecasts | Our cheap tier. Fast, but no events. |
| **Dixon-Coles correction** | The same, with extra weight on 0-0 and 1-1, less on 1-0 and 0-1 | Dixon and Coles 1997. [Overview](https://statsultra.com/dixon-coles-model/) | Already in the cheap tier. It corrects the scoreline, not the underlying rates. |
| **Time-varying rates** | Goal rates that change with the minute, the score and red cards | Dixon and Robinson 1998; Koopman and Lit 2015; recent in-play preprints such as [this one](https://arxiv.org/html/2508.05891v1) | Real goals come late: **55.8% of goals are in the second half** (our data, section 3). We need minute-dependent rates. |
| **Shot-based (expected goals)** | A match is a sequence of shots; each shot has a probability of scoring from its quality | Analytics providers; [DataGaffer](https://www.datagaffer.com/simulator) spreads expected goals over the minutes of a match | **The best fit.** Shots are the event that gives a scorer, an assister, a goalkeeper save and a rating contribution. |
| **Possession Markov chain** | The ball as states (keep, lose, shot) with transition probabilities per action | [Pena 2014](https://arxiv.org/abs/1403.7993), fitted to all 380 Premier League games of 2012/13 | Too fine for a career game. It only reaches the shot and stops there; the paper gives no goal model, no pitch positions and no players. |
| **Full event-by-event with positions** | Every pass and carry on a pitch | Thesis work such as [Cheung 2018](https://www.diva-portal.org/smash/get/diva2:1245916/FULLTEXT01.pdf); commercial engines | Far more than the game needs, and needs data we do not have. |
| **The Football Manager style** | Chances created by player attributes, roles and tactics; hidden | Sports Interactive | Not documented publicly. "No one knows 100% how the match engine works" is the community's own view. |

**Pena's possession numbers, for reference** (from the paper's fit; the shot probability is worked out by us from its figures): per action a team keeps the ball about 75% to 80% of the time, so a possession lasts about 4 to 5 actions, and roughly 6% to 8% of possessions end in a shot. This is a useful sanity check but we do not need the chain.

## 2. The shot-based design we recommend

A match is a timeline of events over 90 minutes plus stoppage time.

1. **Team ratings from the lineup.** Instead of one strength number, each side gets an attack, a midfield and a defence value (and a goalkeeper value) from its lineup's players rated in their slots. This is what makes the formation and the lineup matter.
2. **Chances in time.** Each team's number of shots in the match is drawn from the matchup (its attack and midfield against the other's defence and midfield), then each shot is given a minute, with more shots late in each half.
3. **Each shot** picks a **shooter** (weighted by position and by his finishing and shot attributes), then resolves as blocked, off target, saved or goal, using the shooter against the goalkeeper. A goal asks for an **assister** (or none, for a solo goal), weighted by position and passing and vision attributes.
4. **Other events** with their own rates: fouls, yellow and red cards, corners, penalties, injuries, substitutions.
5. **Everything is recorded** as a list of events, from which the score, the player stats, the match ratings and a text commentary all come.
6. **A cup tie** that is level goes on to extra time (the same event timeline for 30 minutes) and then penalties.

Because the cheap tier stays, background leagues keep the fast score-only draw and only the full tier runs this.

## 3. Real numbers to calibrate against

Computed from the same football-data.co.uk results CSVs as [simulation-balance.md](simulation-balance.md) (18,084 matches, big five, 2014/15 to 2023/24; both teams together, per match):

| | Per match | Notes |
|---|---|---|
| Shots | 24.6 | Premier League 25.4, Bundesliga 25.8, La Liga 23.6, Serie A 24.4, Ligue 1 24.1 |
| Shots on target | 8.8 | 36% of shots |
| Goals per shot | 0.112 | |
| Goals per shot on target | 0.313 | So about 69% of shots on target are saved |
| Corners | 9.9 | |
| Fouls | 25.3 | Premier League 21.5, La Liga 27.0, Serie A 27.1 |
| Yellow cards | 4.10 | Premier League 3.40, La Liga 5.03, Serie A 4.58 |
| Red cards | 0.20 | 0.13 in the Premier League, 0.26 in Ligue 1 |
| Share of goals in the first half | 44.2% | So the second half has 55.8% |

Not in the data we have: penalties, injuries, substitutions and offsides. Those need another source before they are tuned.

**Who takes shots, scores and assists.** The only per-position source read was two Objective Football posts for the Premier League ([2014/15](https://blog-objective-football.blogspot.com/2015/07/201415-player-numbers-by-field-position.html), [2013/14](https://objective-football.blogspot.com/2014/09/201314-player-numbers-by-field-position.html)). It groups players into only three buckets, and it puts wingers and attacking midfielders with the midfielders. Per 90 minutes, 2014/15:

| Group | Goals | Assists | Shots | Shots on target | Players |
|---|---|---|---|---|---|
| Defenders | 0.03 | 0.04 | 0.48 | 0.12 | 173 |
| Midfielders (wingers included) | 0.12 | 0.12 | 1.45 | 0.45 | 213 |
| Attackers | 0.35 | 0.11 | 2.81 | 1.04 | 101 |

Multiplying by a 4-4-2 outfield mix (4 defenders, 4 midfielders, 2 attackers) gives roughly **9% of goals from defenders, 37% from midfielders, 54% from attackers**, and **19%, 56% and 26% of assists**. This is our arithmetic on one season in one league, so use it as a starting weight, not a fact. Goalkeepers are not in the source; they almost never score.

## 4. What this means for the build

- **The shot count and the conversion must multiply back to the tuned goal rates.** The cheap tier already matches the real tables, so the full tier is checked against the same numbers (goals, home and away shares, table spread) plus the new ones (shots, shots on target, cards, corners).
- **Order of work:** the event engine first, then scorers and assisters (they are events), then match ratings (they are built from events), then the stats and leaderboards.
- **Calibration stays in the simtool**, which already compares with real results; it will gain shots, shots on target, cards and corners.

## Open questions

- **Rates we have no source for:** penalties per match, injuries per match and how long they last, how often substitutions happen and at what minute. Each needs a source before it is set.
- **Shot quality.** Real expected goals depend on where the shot is taken. We have no positions, so shot quality would come only from the shooter's attributes, the goalkeeper and a random draw.
- **How much tactics should change the shot count** (pressing, width, tempo). Not researched.
