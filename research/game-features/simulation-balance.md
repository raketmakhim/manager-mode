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

## The shot-based model (2026-11-07)

`engine/src/sim/shot_based.rs` plays every competition for now (`DEFAULT_MODEL` in `competition/mod.rs`, switched on for play-testing; it was the top five leagues only before). Set `DEFAULT_MODEL` to `Cheap` to speed up background leagues. Constants were tuned in the simtool (`cargo run -p simtool --release -- --seasons 40`, 20 seasons for FC 24), first against the event rates, then against the tables. Goals come out of the shots, so the table check is the same as for the cheap model.

| Big five together | Goals | Home % | Draw % | Away % | 1st | 2nd | 4th | Last |
|---|---|---|---|---|---|---|---|---|
| Real | 2.77 | 44.4 | 25.1 | 30.5 | 2.37 | 2.11 | 1.83 | 0.63 |
| Shot-based, FIFA 15 ratings | 2.77 | 44.4 | 24.1 | 31.5 | 2.38 | 2.12 | 1.80 | 0.62 |
| Shot-based, FC 24 ratings | 2.72 | 44.1 | 24.4 | 31.4 | 2.31 | 2.06 | 1.82 | 0.67 |

| Per match, both teams | Real | Shot-based |
|---|---|---|
| Shots | 24.6 | 24.6 |
| On target | 36% | 36% |
| Goals per shot | 0.112 | 0.11 |
| Corners | 9.9 | 9.8 |
| Fouls | 25.3 | 25.3 |
| Yellow cards | 4.10 | 4.07 |
| Red cards | 0.20 | 0.20 |
| Goals in the first half | 44.2% | 44.7% |

What was needed beyond the plain shot rates:
- **Draws.** Independent shots give about 22.7% draws (the same as independent Poisson goals). A side that leads keeps only half of its later shots (`LEADER_KEEPS_SHOOTING`), which brings draws to about 24%, 1 point under real. The cost is that goals and shots fall, so the base shot rate is higher to compensate, and the first-half share of the minutes is set a little under the real 44.2% because a lead is built in the first half.
- **Red cards.** Second yellows alone made reds 0.48 a match; a player already booked is booked a quarter as often (`BOOKED_CAUTION`), which gives the real 0.20.
- **Not modelled yet:** a side with a player sent off is no weaker (he does leave the pitch: see "Parts as models" below), substitutions are only like-for-like by line and not tactical (see "Substitutions" below), penalties or injuries (no source for their rates yet), no per-league goal rate, and possession (52% for the home side on average) has no real figure to check against.

### Stars take more shots (2026-11-07)

With a flat weight by position the top scorers of a three-season run had about 20 league goals a season, and Messi did not appear (FIFA 15 puts him on the left of midfield, where the weight was the midfield average). Changes in `shot_based.rs` (the scoring part now lives in `sim/scoring.rs`):
- A player's share of his side's shots is his slot's shots per 90 times `exp(0.05 * finishing)`: a 92 takes about 3 times the shots of a 70. Assists are chosen the same way with `exp(0.03 * creativity)`.
- The slot weights split the real line averages (midfield 1.45, attack 2.81) by position: wide and attacking midfielders 2.0, central midfielders 0.8 to 1.0, wingers 2.6. The split is our own guess; the line averages are the sourced part.
- Conversion depends more on finishing against the goalkeeper (0.025 per point). To keep the tables, the shot rate per rating point fell from 0.034 to 0.024.

Result, FIFA 15 ratings, 40 seasons, big five: 2.78 goals, 44.6 / 23.8 / 31.6 %, points per match 2.40 / 2.15 / 1.83 / 0.62 (real 2.77, 44.4 / 25.1 / 30.5, 2.37 / 2.11 / 1.83 / 0.63). Three simulated seasons: Ronaldo about 40 league goals a season, Messi about 25, the best strikers about 25 to 30. Messi is still low because the lineup picker puts him at left midfield (his rating there beats his rating at striker); a better picker is a separate job.

### Limits on a star's share (2026-11-07)

Playing six seasons showed the star weighting ran away: Zlatan and Defoe scored more goals than matches played (about 1.07 a match over 230 matches), and lower-league strikers topped the all-time list. A lone good finisher in a weak side took most of its shots, and goalkeepers in weak leagues are rated low, so his conversion was boosted too. Fixes in `shot_based.rs` (now `sim/scoring.rs`):
- `MAX_SHOT_SHARE` 0.28: nobody takes more than that share of his side's shots (the rest is shared by the others); assists are limited to 0.45.
- `CONVERSION_EDGE_RANGE` 0.6 to 1.35: the finishing-against-goalkeeper multiplier stays in that range.

Six FIFA 15 seasons, whole world: the best player with 100+ matches scores 0.76 goals a match (Cristiano Ronaldo 0.65, Messi 0.58, best Championship striker about 0.57 a match, 26 a season). Big five over 40 seasons: 2.78 goals, points per match 2.37 / 2.11 / 1.81 / 0.65 (real 2.37 / 2.11 / 1.83 / 0.63).
Still true: the same limits apply in every league, so a lower-league star scores about as often as a big-league star of the same rating. Real lower-league scoring (lower quality of chances) is not modelled.

### Finishing counts on its own, not against the keeper (2026-11-07)

The 0.28 share limit with a 0.6 to 1.35 edge over the keeper flattened everyone: the all-time top 25 was mostly Championship and League One strikers, who play 46 matches a season, with real stars at 0.65 a match. The conversion chance is now `0.296 * finishing factor * keeper factor`:
- finishing factor `exp(0.03 * (finishing - 70))`, kept between 0.6 and 2.0: the player's own finishing, so a weak-league striker is not lifted by weak keepers;
- keeper factor `exp(0.012 * (70 - keeper rating))`, kept between 0.8 and 1.25.

Six FIFA 15 seasons, whole world: Ronaldo 0.84 goals a match, Messi 0.75, Zlatan 0.79, and the best Championship striker 0.56 (13th on the all-time list). Big five over 40 seasons: 2.80 goals, 44.2 / 24.5 / 31.3 %, points per match 2.38 / 2.13 / 1.79 / 0.67. FC 24 ratings give 2.84 goals (FC 24 finishing values are higher), a little above real 2.77.

### Assists (2026-11-07)

How an assist is given: 75% of goals get one (`GOAL_ASSISTED`; the simulated Premier League has 74% to 76%, real about 70% by our rough memory, not sourced); the assister is chosen among the other ten outfield players with weight = assists per 90 of his slot (real line averages 0.04 defenders, 0.12 midfielders, 0.11 attackers; the split inside is our guess) times `exp(0.05 * creativity)`, no player taking more than 45%. Creativity is the mean of vision and short passing.

Raising the creativity exponent from 0.03 to 0.05 spread assists towards the real playmakers: over six FIFA 15 seasons in the Premier League the best assisters average about 14 a season (David Silva 86 in 228 matches, Di Maria 80, Eriksen 80, Ozil 78), against about 12 before. Real top assisters make 15 to 20 in a season, so single-season leaders are still a little low. Forwards are low too: Messi 31 and Zlatan 28 in six seasons (about 5 a season; real Messi about 10): the striker slot weight is only 0.09, the lowest among attacking slots.

Second pass on assists (same day): the slot weights now favour attackers and wide or attacking midfielders, as in real football: striker and centre-forward 0.14 (was 0.09), wingers 0.22 (0.15), wide and attacking midfielders 0.20 (0.16), central midfielders 0.08 (0.10), holding midfielders 0.04 (0.06). Defenders keep 0.03 to 0.06. Results over six FIFA 15 seasons, Premier League: Ozil 105 assists (17.5 a season), Coutinho 95, Silva 92, Di Maria 91, Eriksen 87; in all competitions Messi 54 (9 a season, real about 10) and Ronaldo 78. By the line of the main position, the assists split about 57% midfielders, 21% attackers, 17% defenders, which is near the sourced 56 / 26 / 19 split. Goals and tables did not move.

### Assists against real figures (2026-11-07, partial)

No per-player real data is in our downloads (football-data.co.uk has results only). What a web search found, and nothing more: the Premier League's top assister each season (sources disagree on a few; 2020/21 and 2021/22 not found): 2014/15 Fabregas 18, 2015/16 Ozil 19, 2016/17 De Bruyne 18, 2017/18 De Bruyne 16, 2018/19 Hazard 15, 2019/20 De Bruyne 20, 2022/23 De Bruyne 16, 2023/24 Watkins 13 (sources: [The Analyst](https://theanalyst.com/articles/most-assists-in-a-premier-league-season), [Goal](https://www.goal.com/en/news/what-is-the-record-for-most-premier-league-assists-in-a-season/hodcwbkdjple1bg1wq09aouqo), [Football Transfers](https://www.footballtransfers.com/us/transfer-news/uk-premier-league/2023/07/premier-league-assists-which-player-has-set-up-most-goals-us)). Simulated Premier League leaders over five FIFA 15 seasons (before the second weights pass): 18, 17, 14, 17, 17, and the 10th best 10 to 11. So the simulated leader is inside the real range of 13 to 20, with a mean of 16 against a real mean of about 17.
Not found: the share of goals that have an assist by league (we use 75%; our rough memory of real data is about 70%), assists by position, and per-player figures to compare Messi, Ronaldo and others. Candidates: FBref or Understat player tables for 2014/15 to 2023/24 (Kaggle has a three-season set for 2017/18 to 2019/20 and a five-year set; licences and column definitions not checked).

Third pass (2026-11-08): Messi had 15 assists in 87 matches although his passing, vision and dribbling are better than his team-mates'. Two causes: the striker slot weight (0.14) put him under a CAM of lower rating, and creativity ignored dribbling. Creativity is now the mean of vision, short passing and dribbling, the striker weight is 0.18, and the share of goals with an assist is 0.70 (it was 0.75; the higher creativity of the whole squad had pushed leaders above the real range). Six FIFA 15 single seasons: Messi 0.40 assists a match (101 in 253 matches; real about 10 a season), simulated Premier League leaders 18 to 22 a season with the 10th at 11 to 12 (real leaders 13 to 20), and 69 to 71% of goals assisted. Leaders are still a little above real; lowering `GOAL_ASSISTED` further is the next lever.

Tried and rejected (2026-11-08): making assists depend more on creativity and less on the slot. Six FIFA 15 seasons for Messi, five Premier League seasons for the leaders, assists by the line of the main position (sourced split 56 / 26 / 19 midfield / attack / defence):

| Variant | Messi a match | PL leader per season | Midfield / attack / defence |
|---|---|---|---|
| Kept: slot weight as is, creativity exponent 0.05 | 0.40 | 18 to 22 | 59 / 28 / 13 % |
| Slot weight square-rooted, exponent 0.07 | 0.33 | 16 to 18 | 61 / 22 / 17 % |
| Slot weight as is, exponent 0.07 | 0.42 | 19 to 28 | 62 / 28 / 11 % |

Flattening the slot weight pushes assists to midfielders and defenders and lowers the forwards (Messi included); a stronger creativity term alone makes the single-season leaders too high (28). The kept setting is the closest to the real leader range and the sourced line split.

Creativity widened (2026-11-08): from vision, short passing and dribbling to vision, short passing, long passing, crossing and dribbling (the mean of the five). Same checks as above:

| Creativity | Messi a match | PL leader per season | Midfield / attack / defence |
|---|---|---|---|
| Vision, short passing, dribbling | 0.40 | 18 to 22 | 59 / 28 / 13 % |
| Plus long passing and crossing (kept) | 0.34 | 18 to 20 | 60 / 26 / 15 % |

The five-attribute version brings the leaders into the real range (13 to 20) and the line split to the sourced 56 / 26 / 19. Messi falls to about 13 assists a 38-match season (real La Liga seasons: about 11 to 18), because his crossing and long passing are lower than his dribbling.

Vision weighted (2026-11-08): creativity is now half vision plus half the mean of short passing, long passing, crossing and dribbling (the user's choice; no real data to fit the weights). Messi 0.36 assists a match, Premier League leaders 20 to 21 a season (10th 12 to 13), assists by line 60 / 27 / 13 % (midfield / attack / defence; sourced 56 / 26 / 19). Most assists over five seasons, all competitions: Hernandez 84, Pogba 82, Rakitic 81, Correia 80, Etxeberria 79, Ozil 77.

Creativity weights (2026-11-08, replaces the 50 / 12.5 split above): vision 50%, short passing 20%, long passing 11%, crossing 11%, dribbling 8% (the user's ordering; the numbers are our own). Results are almost the same as the equal split of the other four: Messi 0.36 assists a match, Premier League leaders 20 to 21 a season, assists by line 60 / 26 / 13 %.

### Cards by player (2026-11-08)

Cards were spread evenly over the ten outfield players, so Messi collected 5 red cards in 4 simulated seasons (an average outfield player has about 0.01 reds a match, so even by chance 1 or 2 is normal). Real figures found by web search: Messi has 3 or 4 red cards in about 1,130 games (about 0.003 a game) and 103 yellows (about 0.09 a game); Ronaldo 12 or 13 reds in about 1,300 games (0.01 a game) and 162 yellows (0.125 a game) ([Planet Football](https://www.planetfootball.com/stats-tables/messi-ronaldo-red-cards-yellow-cards-disciplinary-record), [The Week](https://theweek.com/sport/football/90293/lionel-messi-vs-cristiano-ronaldo-career-goals-awards-internationals)); an average outfield player 0.2 yellows and 0.01 reds a match (our computation from the 4.1 and 0.2 per match above). Over the last four Premier League years second yellows were about 46% of dismissals and straight reds about 54% (arithmetic on a [Sky Sports](https://www.skysports.com/football/news/11096/11074552/premier-league-red-cards-are-dismissals-on-the-rise) count; unverified). No study of bookings by position was found, only an unsourced opinion that defensive midfielders are booked most.

Change: a player's share of his side's fouls is now `exp(0.037 * aggression)` (the FIFA aggression attribute; 60 if missing), at most 20% of the side's fouls, and the goalkeeper and empty slots never foul. Four simulated FIFA 15 seasons: the whole world 4.02 yellows and 0.22 reds a match (real 4.1 and 0.2), Messi (aggression 48) 0.11 yellows a game and no reds, Ronaldo (63) 0.17, Zlatan (84) 0.24, Mascherano (89) 0.39; by the line of the main position 0.16 forwards, 0.19 midfielders, 0.23 defenders. The most booked players are defensive and central midfielders (0.4 to 0.46 yellows a game and up to 0.05 reds, a little above the worst real records). Not modelled: referee strictness, derbies, more cards late in matches or when behind, a card for a particular kind of foul.

### Parts as models (2026-11-08)

The scoring and discipline parts of the shot-based model moved out of `shot_based.rs` into swappable models, with no change to the figures above: `sim/scoring.rs` (`ScoringModel`: `Flat`, `StarWeighted`; the constants for shot share, conversion, keeper factor and assists are there now) and `sim/discipline.rs` (`DisciplineModel`: `Even`, `ByAggression`; the foul, card and aggression constants are there now). Both keep tuning against the figures in this file. `pick` (the capped weighted choice) is in `sim/pick.rs`; it now goes round once per player so the cap holds when several players are far above the rest.

Two fixes came with it: a player sent off is replaced by an empty slot for the rest of the match, extra time included, so he no longer shoots, assists, fouls or is carded again; and a goal that no teammate can have set up has no assister instead of falling back to the goalkeeper.

### Substitutions (2026-11-08)

`sim/substitution.rs` (`SubstitutionModel`: `Never`, `Planned`, default `Planned`) plans each side's changes and picks who goes off and who comes on from the seven on the bench (`MatchTeam.bench`, picked by `lineup::pick_bench`). The number allowed is `Rules.substitutions` (5 for every competition for now: right since 2020/21, 3 before; the rules-by-year work should make it depend on the season).

Real figures found (all Premier League unless said): about 3.9 changes per side per match under the five-change rule, and 3.66 before it was widened (The Substack analysis of 2020/21; 3.85 per side in the first gameweek of 2022/23); the first change at about the 59th to 61st minute (PA analysis of 2021/22: 59th; first gameweek of 2022/23: 61:36); three in four changes after the 60th minute (a 2015/16 to 2022/23 study of four leagues); 13.3% of the goals scored by substitutes in 2023/24, a record (The Analyst; the previous high was 12.8% in 2013/14).

The model: each of the first three allowed changes is used 97% of the time and later ones 50% (3.9 a side when 5 are allowed, 2.9 when 3); a change is at a minute 50 + 40 * the mean of two random numbers (mean 70, so about 80% after the 60th); a player to go off is chosen from the outfield with weights 1.6 forwards, 1.2 midfielders, 0.7 defenders (our guess: attackers are changed most); the replacement is the best unused outfield substitute of the same line, otherwise the best unused one. The newcomer plays the slot at his own rating. The simtool now prints changes per side and the share of goals by substitutes: 3.9 changes and 13.8% of goals over 20 simulated seasons of the big five (real 13.3%), with the other figures above unchanged.

Stats: a substitute gets an appearance and the minutes from his minute; a player taken off or sent off stops at that minute; goals conceded and the clean sheet (60 minutes) now count only while the player was on the pitch (this replaces the old shortcut that gave the whole lineup every goal against).

Not modelled: changes by the state of the match (a side that is losing brings on an attacker), injuries, a goalkeeper replaced, the extra change in extra time, tired players (no stamina in the model), the user choosing changes, and a side with ten men or a changed shape being weaker or stronger.

