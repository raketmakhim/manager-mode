# Match simulation against real results

Measured 2026-11-07. Real data: the results of every match in the five big leagues, 2014/15 to 2023/24, from [football-data.co.uk](https://www.football-data.co.uk/data.php) (one CSV per league and season, kept in `research/information/data/real-results/`, not committed). Script: [real_league_stats.py](real_league_stats.py) (`py -I research/game-features/real_league_stats.py`). Simulation: `dev-tools/simtool` (`cargo run -p simtool --release -- --edition 15 --seasons 20 --seed 1`), 20 seasons from FIFA 15 ratings, league matches only.

Each "points per match" figure is the average over seasons of that place in the final table (champion, runner-up, 4th, last), so 2.41 is about 92 points in a 38-match season.

## Real, 2014/15 to 2023/24

| League | Goals/match | Home % | Draw % | Away % | 1st | 2nd | 4th | Last |
|---|---|---|---|---|---|---|---|---|
| Premier League | 2.79 | 44.9 | 23.3 | 31.7 | 2.41 | 2.19 | 1.84 | 0.59 |
| Bundesliga | 3.02 | 45.1 | 24.9 | 30.0 | 2.38 | 2.06 | 1.77 | 0.65 |
| La Liga | 2.63 | 45.5 | 26.0 | 28.5 | 2.37 | 2.18 | 1.87 | 0.67 |
| Serie A | 2.77 | 42.7 | 25.7 | 31.6 | 2.36 | 2.11 | 1.87 | 0.56 |
| Ligue 1 | 2.65 | 43.7 | 25.8 | 30.5 | 2.35 | 2.03 | 1.80 | 0.66 |

## Simulation before tuning (goal rate 0.06 per rating point, home factor 1.15, base 1.35 goals)

| League | Goals/match | Home % | Draw % | Away % | 1st | 2nd | 4th | Last |
|---|---|---|---|---|---|---|---|---|
| Premier League | 2.74 | 46.0 | 24.7 | 29.4 | 1.94 | 1.86 | 1.63 | 0.90 |
| Bundesliga | 2.78 | 46.2 | 23.9 | 29.9 | 2.06 | 1.84 | 1.62 | 0.84 |
| La Liga | 2.72 | 46.7 | 24.0 | 29.3 | 2.02 | 1.86 | 1.65 | 0.89 |
| Serie A | 2.75 | 45.4 | 24.9 | 29.7 | 1.97 | 1.84 | 1.66 | 0.77 |
| Ligue 1 | 2.75 | 45.5 | 24.9 | 29.6 | 1.95 | 1.76 | 1.62 | 0.87 |

## What the comparison says

1. **Goals per match are close** (Premier League 2.74 against 2.79). The Bundesliga scores more in reality (3.02 against 2.78) and La Liga and Ligue 1 less (2.63 and 2.65 against 2.72 and 2.75), so a league-wide goal rate belongs in `Rules` later, not one global number.
2. **The table is far too flat.** Real champions take 2.35 to 2.41 points a match; ours take 1.94 to 2.06. Real last-place clubs take 0.56 to 0.67; ours take 0.77 to 0.90. The spread between best and worst is about 1.8 points a match in reality and about 1.0 in the simulation. A 38-match season shows it: real champion about 92 points and last about 22, simulated champion about 74 and last about 34.
3. **Home advantage is a little high and away wins too rare.** Real away wins are 28.5% to 31.7%, simulated 29.3% to 29.9%; real home wins 42.7% to 45.5%, simulated 45.4% to 46.7%. The Premier League real draw rate (23.3%) is below the simulated one (24.7%).
4. **The cause is the rating gap.** The real spread needs a larger effect of rating difference on goals. This was the fix: see "After tuning" below.
5. **Strength is measured on paper.** The simulation uses the mean lineup rating, which compresses real differences (the best clubs have far more depth and star power than the mean shows). If raising the constant alone makes the top too strong or the middle odd, weight the best players more in `team_strength`.

## Premier League champions, for reference

| Season | Champion | Points | Runner-up (points) | Last (points) |
|---|---|---|---|---|
| 2014/15 | Chelsea | 87 | Man City (79) | QPR (30) |
| 2015/16 | Leicester | 81 | Arsenal (71) | Aston Villa (17) |
| 2016/17 | Chelsea | 93 | Tottenham (86) | Sunderland (24) |
| 2017/18 | Man City | 100 | Man United (81) | West Brom (31) |
| 2018/19 | Man City | 98 | Liverpool (97) | Huddersfield (16) |
| 2019/20 | Liverpool | 99 | Man City (81) | Norwich (21) |
| 2020/21 | Man City | 86 | Man United (74) | Sheffield United (23) |
| 2021/22 | Man City | 93 | Liverpool (92) | Norwich (22) |
| 2022/23 | Man City | 89 | Arsenal (84) | Southampton (25) |
| 2023/24 | Man City | 91 | Arsenal (89) | Sheffield United (16) |

Leicester's 81 points in 2015/16 (a 5000-to-1 title) is a useful check that upsets must stay possible: the tuned simulation should still let a mid-rated club win now and then.

## Limits of this comparison

- The simulated seasons start from FIFA 15 ratings only. Real seasons differ in club strength every year, so compare averages, not single seasons.
- The tool plays league matches only: no cups, European matches or injuries, which also affect real results.
- Real draws by league and season vary by a few points; with 10 seasons per league the real averages are stable to about 0.1 goals a match and a point or two of percentage.

## After the first tuning (2026-11-07, before the draw correction)

Constants in `engine/src/sim.rs` changed to **0.135 per rating point, home factor 1.13, base 1.29 goals**. Found by scanning the rating-gap constant and then adjusting the base and the home factor to keep goals and home wins in place. 40 seasons from seed 1.

One lesson: tuning first to the "all leagues" row looked right (0.20 per point) but made every big-five league too steep (Bundesliga champions at 2.78 points a match). The real numbers are for the big five, so the target is the **big five together** row; all leagues of 8+ clubs include many flatter leagues and should not be tuned to the big-five table.

| | Goals | Home % | Draw % | Away % | 1st | 2nd | 4th | Last |
|---|---|---|---|---|---|---|---|---|
| Real, big five | 2.77 | 44.4 | 25.1 | 30.5 | 2.37 | 2.11 | 1.83 | 0.63 |
| Simulated, big five | 2.76 | 45.1 | 22.8 | 32.2 | 2.39 | 2.11 | 1.83 | 0.64 |
| Real, Premier League | 2.79 | 44.9 | 23.3 | 31.7 | 2.41 | 2.19 | 1.84 | 0.59 |
| Simulated, Premier League | 2.75 | 44.8 | 23.1 | 32.2 | 2.28 | 2.11 | 1.92 | 0.61 |
| Real, Bundesliga | 3.02 | 45.1 | 24.9 | 30.0 | 2.38 | 2.06 | 1.77 | 0.65 |
| Simulated, Bundesliga | 2.79 | 45.1 | 22.9 | 32.0 | 2.51 | 2.09 | 1.73 | 0.59 |
| Real, La Liga | 2.63 | 45.5 | 26.0 | 28.5 | 2.37 | 2.18 | 1.87 | 0.67 |
| Simulated, La Liga | 2.76 | 45.5 | 22.7 | 31.8 | 2.42 | 2.22 | 1.86 | 0.71 |
| Real, Serie A | 2.77 | 42.7 | 25.7 | 31.6 | 2.36 | 2.11 | 1.87 | 0.56 |
| Simulated, Serie A | 2.77 | 44.7 | 22.4 | 33.0 | 2.32 | 2.09 | 1.87 | 0.59 |
| Real, Ligue 1 | 2.65 | 43.7 | 25.8 | 30.5 | 2.35 | 2.03 | 1.80 | 0.66 |
| Simulated, Ligue 1 | 2.73 | 45.3 | 22.8 | 31.9 | 2.44 | 2.01 | 1.75 | 0.70 |

The champion, runner-up, 4th and last place points per match of the big five together now match the real table to within 0.02, and goals per match to 0.01.

### What was still off at that point

- **Draws are about 2.3 points too rare and away wins about 1.7 too common** (22.8% and 32.2% against 25.1% and 30.5%). Two goal counts drawn independently produce fewer draws than real football, where low-scoring games cluster. Fixing this needs a change to the model (for example a Dixon-Coles style correction that raises 0-0 and 1-1), not another constant: raising the home factor lowers away wins but pushes home wins above 46%.
- **Per-league goal rates.** The Bundesliga scores 3.02 a match in reality, La Liga 2.63 and Ligue 1 2.65; the simulation gives every league about 2.75. A league goal rate belongs in `Rules` once rules are data by country.
- **Single seasons.** Individual leagues differ from their real averages by up to 0.13 points a match (Bundesliga champion 2.51 against 2.38, because Bayern are far ahead of FIFA 15's other clubs; Premier League 2.28 against 2.41). Those come from the ratings, which is expected, not from the constants.
- **Real upsets.** The strongest club on paper wins the Premier League only 35% of the time in the simulation, and a top-3 club 80% of the time, so a Leicester 2015/16 stays possible. There is no real figure for this to compare with.
- **Other editions.** Tuned on FIFA 15. A check with FC 24 ratings (20 seasons, big five together) gives 2.75 goals, 45.4 / 22.7 / 32.0 % home / draw / away and points per match of 2.28 (1st), 2.07 (2nd), 1.83 (4th) and 0.65 (last) against the real 2.37, 2.11, 1.83 and 0.63. So the tuning carries over; the champion is slightly low because FC 24 ratings are a little more compressed.

## After the draw correction (2026-11-07)

Added a Dixon-Coles low-score correction to `full_time_score` in `engine/src/sim.rs` (constant `LOW_SCORE_CORRECTION`, rho = -0.10): the two goal counts are still drawn from independent Poisson distributions, and a drawn pair is kept with a probability that is higher for 0-0 and 1-1 and lower for 1-0 and 0-1 (rejection sampling). Then the home factor went to **1.14** and the rating-gap constant to **0.14** to bring home wins and the table spread back. Final constants: **base 1.29 goals, home factor 1.14, 0.14 per rating point, rho -0.10**.

A scan of rho (big five together, other constants as before): rho 0 gives 22.7% draws, -0.05 gives 23.8%, **-0.10 gives 25.2%**, -0.15 gives 25.8%, -0.20 gives 27.0%. Goals per match stayed at 2.77 throughout.

| 40 seasons from seed 1 | Goals | Home % | Draw % | Away % | 1st | 2nd | 4th | Last |
|---|---|---|---|---|---|---|---|---|
| Real, big five | 2.77 | 44.4 | 25.1 | 30.5 | 2.37 | 2.11 | 1.83 | 0.63 |
| Simulated, FIFA 15 ratings | 2.78 | 45.1 | 24.4 | 30.5 | 2.41 | 2.11 | 1.82 | 0.63 |
| Simulated, FC 24 ratings (20 seasons) | 2.76 | 44.6 | 24.8 | 30.5 | 2.31 | 2.06 | 1.83 | 0.66 |

Per league with FIFA 15 ratings (first / last place points per match against the real figure): Premier League 2.28 / 0.58 (real 2.41 / 0.59), Bundesliga 2.54 / 0.57 (2.38 / 0.65), La Liga 2.43 / 0.74 (2.37 / 0.67), Serie A 2.32 / 0.58 (2.36 / 0.56), Ligue 1 2.47 / 0.70 (2.35 / 0.66). Draws are 23.9% to 25.3% in each of the five (real 23.3% to 26.0%).

Still off: goal rates are the same in every league (Bundesliga 3.02, La Liga 2.63 in reality), and single leagues follow their ratings (Bayern's lead over the Bundesliga in FIFA 15 gives a champion at 2.54).
