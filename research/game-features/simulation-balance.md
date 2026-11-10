# Match simulation against real results

Measured 2026-11-07 to 2026-11-08. Real data: the results of every match in the five big leagues, 2014/15 to 2023/24, from [football-data.co.uk](https://www.football-data.co.uk/data.php) (one CSV per league and season, kept in `research/information/data/real-results/`, not committed). Script: [real_league_stats.py](real_league_stats.py) (`py -I research/game-features/real_league_stats.py`). Simulation: `dev-tools/simtool` (`cargo run -p simtool --release -- --seasons 40`), seasons from FIFA 15 ratings unless said, league matches only. The constants themselves live in the code (`sim/cheap.rs`, `sim/shot_based.rs`, `sim/scoring.rs`, `sim/discipline.rs`, `sim/substitution.rs`, `sim/fatigue.rs`); this file holds the real figures, the method and the lessons.

Each "points per match" figure is the average over seasons of that place in the final table (champion, runner-up, 4th, last), so 2.41 is about 92 points in a 38-match season.

## 1. Real figures, 2014/15 to 2023/24

| League | Goals/match | Home % | Draw % | Away % | 1st | 2nd | 4th | Last |
|---|---|---|---|---|---|---|---|---|
| Premier League | 2.79 | 44.9 | 23.3 | 31.7 | 2.41 | 2.19 | 1.84 | 0.59 |
| Bundesliga | 3.02 | 45.1 | 24.9 | 30.0 | 2.38 | 2.06 | 1.77 | 0.65 |
| La Liga | 2.63 | 45.5 | 26.0 | 28.5 | 2.37 | 2.18 | 1.87 | 0.67 |
| Serie A | 2.77 | 42.7 | 25.7 | 31.6 | 2.36 | 2.11 | 1.87 | 0.56 |
| Ligue 1 | 2.65 | 43.7 | 25.8 | 30.5 | 2.35 | 2.03 | 1.80 | 0.66 |
| **Big five together** | **2.77** | **44.4** | **25.1** | **30.5** | **2.37** | **2.11** | **1.83** | **0.63** |

Premier League champions, as a check that upsets must stay possible: 2014/15 Chelsea 87 points, 2015/16 Leicester 81 (last Aston Villa 17), 2016/17 Chelsea 93, 2017/18 Man City 100, 2018/19 Man City 98 (Liverpool 97), 2019/20 Liverpool 99, 2020/21 Man City 86, 2021/22 Man City 93 (Liverpool 92), 2022/23 Man City 89, 2023/24 Man City 91. Leicester's 5000-to-1 title means a mid-rated club must still win now and then.

Limits of the comparison: simulated seasons start from FIFA 15 ratings only (real club strength changes every year, so compare averages, not single seasons); the tool plays league matches only (no cups, Europe or injuries); with 10 real seasons per league the averages are stable to about 0.1 goals a match and a point or two of percentage.

## 2. The cheap model (team strength to a score)

- **First result:** with a goal rate of 0.06 per rating point the table was far too flat (champion 1.94 to 2.06 points a match against 2.35 to 2.41, last place 0.77 to 0.90 against 0.56 to 0.67), though goals (2.74 against 2.79) were close. The cause is the rating gap: the real spread needs a larger effect of rating difference on goals. The simulation uses the mean lineup rating, which compresses real differences (the best clubs have far more depth and star power than the mean shows).
- **Tuning** (scan of the rating-gap constant, then the base and the home factor to keep goals and home wins): lesson, tune to the **big five together** row. Tuning first to an "all leagues" row looked right but made every big-five league too steep (Bundesliga champions at 2.78); leagues of 8 or more clubs include many flatter leagues and must not be tuned to the big-five table.
- **Draws were about 2.3 points too rare** (22.8% against 25.1%): two goal counts drawn independently give fewer draws than real football, where low-scoring games cluster. Fixed with a Dixon-Coles low-score correction (`LOW_SCORE_CORRECTION`, rho -0.10; a drawn pair is kept with a higher probability for 0-0 and 1-1), then the home factor and gap constant were nudged back. Scan of rho: 0 gives 22.7% draws, -0.05 gives 23.8%, -0.10 gives 25.2%, -0.15 gives 25.8%, -0.20 gives 27.0%; goals stayed at 2.77.
- **Result** (40 seasons, big five): FIFA 15 ratings 2.78 goals, 45.1 / 24.4 / 30.5 %, points 2.41 / 2.11 / 1.82 / 0.63; FC 24 ratings (20 seasons) 2.76 goals, 44.6 / 24.8 / 30.5 %, points 2.31 / 2.06 / 1.83 / 0.66 (the champion is a little low because FC 24 ratings are more compressed). Per league the champion is within 0.2 points of real (Bayern's lead in FIFA 15 gives a Bundesliga champion at 2.54 against 2.38): that comes from the ratings, not the constants.
- **Still off:** goal rates are the same in every league (Bundesliga 3.02, La Liga 2.63, Ligue 1 2.65 in reality; about 2.75 in the simulation), so a league goal rate belongs in `Rules`; there is no real figure for how often the strongest club on paper wins (the simulation: Premier League 35%, a top-3 club 80%).

## 3. The shot-based model

`sim/shot_based.rs` plays every competition for now (`DEFAULT_MODEL` in `competition/mod.rs`; `Cheap` speeds up background leagues). Goals come out of the shots, so the table check is the same as for the cheap model.

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
- **Draws.** Independent shots give about 22.7% draws. A side that leads keeps only half of its later shots (`LEADER_KEEPS_SHOOTING`), which brings draws to about 24%. The cost is that goals and shots fall, so the base shot rate is higher to compensate, and the first-half share of the minutes is a little under the real 44.2% because a lead is built in the first half.
- **Red cards.** Second yellows alone made reds 0.48 a match; a player already booked is booked a quarter as often (`BOOKED_CAUTION`), which gives the real 0.20.
- **Not modelled:** penalties and injuries (no source for their rates yet), per-league goal rate, possession (52% for the home side on average; no real figure to check against).

### Who scores and who assists (`sim/scoring.rs`, `ScoringModel { Flat, StarWeighted }`)

- **Stars take more shots.** With a flat weight by position the top scorers had about 20 league goals a season and Messi did not appear (FIFA 15 puts him on the left of midfield). A player's share of his side's shots is his slot's shots per 90 times `exp(0.05 * finishing)`: a 92 takes about 3 times the shots of a 70. The slot weights split the real line averages (midfield 1.45, attack 2.81 shots per 90) by position; the line averages are the sourced part, the split inside is our guess.
- **Limits.** Six simulated seasons showed the weighting run away: Zlatan and Defoe scored more goals than matches (about 1.07 a match over 230), and lower-league strikers topped the all-time list, because a lone good finisher in a weak side took most of its shots and weak-league keepers are rated low. Fixes: nobody takes more than 28% of his side's shots (assists 45%), and conversion uses the player's own finishing factor (`exp(0.03 * (finishing - 70))`, kept between 0.6 and 2.0) times a goalkeeper factor (`exp(0.012 * (70 - keeper))`, kept between 0.8 and 1.25), so a weak-league striker is not lifted by weak keepers. Whole-world six-season result: Ronaldo 0.84 goals a match, Messi 0.75, Zlatan 0.79, best Championship striker 0.56; big five over 40 seasons 2.80 goals, 44.2 / 24.5 / 31.3 %, points 2.38 / 2.13 / 1.79 / 0.67. FC 24 ratings give 2.84 goals (FC 24 finishing values are higher). Still true: the same limits apply in every league, so a lower-league star scores about as often as a big-league star of the same rating.
- **Assists.** 70% of goals get one (`GOAL_ASSISTED`; real about 70% by our rough memory, not sourced). The assister is chosen among the other ten outfield players, weight = assists per 90 of his slot (real line averages 0.04 defenders, 0.12 midfielders, 0.11 attackers; the split inside is our guess, favouring attackers and wide or attacking midfielders) times `exp(0.05 * creativity)`, no player over 45%. Creativity is 50% vision, 20% short passing, 11% long passing, 11% crossing, 8% dribbling (the user's ordering; the numbers are ours).
- **Real figures found for assists** (web search only; no per-player data in our downloads): the Premier League's top assister each season: 2014/15 Fabregas 18, 2015/16 Ozil 19, 2016/17 De Bruyne 18, 2017/18 De Bruyne 16, 2018/19 Hazard 15, 2019/20 De Bruyne 20, 2022/23 De Bruyne 16, 2023/24 Watkins 13 (2020/21 and 2021/22 not found; sources: [The Analyst](https://theanalyst.com/articles/most-assists-in-a-premier-league-season), [Goal](https://www.goal.com/en/news/what-is-the-record-for-most-premier-league-assists-in-a-season/hodcwbkdjple1bg1wq09aouqo), [Football Transfers](https://www.footballtransfers.com/us/transfer-news/uk-premier-league/2023/07/premier-league-assists-which-player-has-set-up-most-goals-us)). Assists by the line of the main position, sourced split: 56 / 26 / 19 % midfield / attack / defence. Not found: the share of goals with an assist by league, assists by position by league, per-player figures (candidates: FBref or Understat tables; Kaggle sets for 2017/18 to 2019/20 and five years; licences and column definitions not checked).
- **Result with the final weights** (six FIFA 15 seasons): Messi 0.36 assists a match (real about 10 a season), simulated Premier League leaders 20 to 21 a season with the 10th at 12 to 13 (real leaders 13 to 20, a little above), assists by line 60 / 26 / 13 % (sourced 56 / 26 / 19). Most assists over five seasons, all competitions: Hernandez 84, Pogba 82, Rakitic 81, Correia 80, Etxeberria 79, Ozil 77.
- **Tried and rejected: making assists depend more on creativity and less on the slot** (Messi a match, Premier League leader per season, midfield / attack / defence split):

| Variant | Messi | PL leader | Split |
|---|---|---|---|
| Kept: slot weight as is, creativity exponent 0.05 | 0.34 to 0.40 | 18 to 22 | 59 / 28 / 13 % |
| Slot weight square-rooted, exponent 0.07 | 0.33 | 16 to 18 | 61 / 22 / 17 % |
| Slot weight as is, exponent 0.07 | 0.42 | 19 to 28 | 62 / 28 / 11 % |

Flattening the slot weight pushes assists to midfielders and defenders and lowers the forwards (Messi included); a stronger creativity term alone makes single-season leaders too high (28). Lowering `GOAL_ASSISTED` is the lever if leaders stay a little high.

### Cards by player (`sim/discipline.rs`, `DisciplineModel { Even, ByAggression }`)

Cards were spread evenly over the ten outfield players, so Messi collected 5 reds in 4 simulated seasons. Real figures found by web search: Messi 3 or 4 reds in about 1,130 games (0.003 a game) and 103 yellows (0.09 a game); Ronaldo 12 or 13 reds in about 1,300 games (0.01) and 162 yellows (0.125) ([Planet Football](https://www.planetfootball.com/stats-tables/messi-ronaldo-red-cards-yellow-cards-disciplinary-record), [The Week](https://theweek.com/sport/football/90293/lionel-messi-vs-cristiano-ronaldo-career-goals-awards-internationals)); an average outfield player 0.2 yellows and 0.01 reds a match (our computation from the 4.1 and 0.2 per match). Over the last four Premier League years second yellows were about 46% of dismissals and straight reds about 54% (arithmetic on a [Sky Sports](https://www.skysports.com/football/news/11096/11074552/premier-league-red-cards-are-dismissals-on-the-rise) count; unverified). No study of bookings by position was found.

A player's share of his side's fouls is `exp(0.037 * aggression)` (60 if the attribute is missing), at most 20% of the side's fouls; the goalkeeper and empty slots never foul. Four simulated FIFA 15 seasons: whole world 4.02 yellows and 0.22 reds a match (real 4.1, 0.2), Messi (aggression 48) 0.11 yellows a game and no reds, Ronaldo (63) 0.17, Zlatan (84) 0.24, Mascherano (89) 0.39; by line 0.16 forwards, 0.19 midfielders, 0.23 defenders. Most booked: defensive and central midfielders (0.4 to 0.46 yellows a game, a little above the worst real records). Not modelled: referee strictness, derbies, more cards late or when behind, a card for a kind of foul.

### Parts as models

Scoring and discipline are swappable models; `pick` (the capped weighted choice) is in `sim/pick.rs` and goes round once per player so the cap holds when several players are far above the rest. A player sent off is replaced by an empty slot for the rest of the match, extra time included, and a goal nobody could have set up has no assister (it no longer falls back to the goalkeeper).

### Substitutions (`sim/substitution.rs`, `SubstitutionModel { Never, Planned }`)

Real figures found (Premier League unless said): about 3.9 changes per side per match under the five-change rule and 3.66 before it was widened (The Substack analysis of 2020/21; 3.85 per side in the first gameweek of 2022/23); the first change at about the 59th to 61st minute (PA analysis of 2021/22: 59th; first gameweek of 2022/23: 61:36); three in four changes after the 60th minute (a 2015/16 to 2022/23 study of four leagues); 13.3% of the goals scored by substitutes in 2023/24, a record (The Analyst; the previous high was 12.8% in 2013/14).

The model plans each side's changes: the first three allowed are used 97% of the time and later ones 50% (3.9 a side with 5 allowed, 2.9 with 3), at a minute 50 + 40 x the mean of two random numbers (mean 70, about 80% after the 60th); who goes off is chosen from the outfield with weights favouring forwards (our guess: attackers are changed most) and tired players; the replacement is the best unused substitute of the same line, otherwise the best unused one. The number allowed is `Rules.substitutions` (5 for every competition for now: right since 2020/21, 3 before). Result over 20 simulated seasons of the big five: 3.9 changes a side and 13.8% of goals by substitutes (real 13.3%). A substitute gets an appearance and the minutes from his minute; goals conceded and the clean sheet (60 minutes) count only while the player was on the pitch. Not modelled: changes by the state of the match, injuries, a goalkeeper replaced, the extra change in extra time.

### Live match, fatigue and tactics

- **A minute at a time.** `LiveMatch` plays the shot-based match a minute at a time (`simulate_with` plays one out). Each minute has a share of a side's 90-minute shots and fouls (`minute_weight`: the first half takes 41% of the events), so the strengths on the pitch at that minute set the rates: a red card (a sent-off player leaves a rating of 0, so ten men are weaker), a change or a tired player now moves the match. Strengths are worked out again every 5 minutes and at once after a change or a red card. The user's side is run by hand; the other side uses the substitution model.
- **Fatigue** (`FatigueModel`, design and real figures in `fitness.md`): tired players lose rating, substitutes start fresh, and the substitution model takes tired players off first (the line weights became 1.3 forwards, 1.1 midfielders, 0.7 defenders: with the tiredness preference the share of goals by substitutes had risen to 15-16%).
- **Retuning.** With the bench weaker than the starters and fresh legs not making up for it, the second half scored less and the first-half share of goals rose to about 46% (real 44.2%); `FIRST_HALF_SHARE` and the goal chance per shot on target were nudged. Result over 40 seasons of the big five: 2.76 goals (real 2.77), 44.3 / 24.3 / 31.3 %, points 2.36 / 2.11 / 1.81 / 0.65, 25.1 shots (real 24.6), 44.2% of goals in the first half, 3.9 changes a side, 14.4% of goals by substitutes. A season takes 0.33 s. Later checks (fitness between matches, wear, ageing) kept 2.73 to 2.77 goals and points within 0.05.
- **Tactics.** The only tactic the game has is the formation, so that and swapping two players' places is what the user can change. `MatchPlayer.ratings` holds a player's rating in every position, so a player in another slot, or a substitute from another line, plays at his real rating there with the lineup's familiarity penalty. `swap_positions` and `set_formation` (the goalkeeper stays; each outfield player goes where he is rated highest, best match first; a sent-off player leaves one slot empty) take effect from the next minute and cost no substitution. Not modelled: mentality, pressing, width; the rating model still rates a player at the position he started in.
- **Cost.** Working out ratings in all 15 positions for everyone made a simulated season about 20% slower (0.33 to 0.40 s), so only the starters of the user's club and the substitutes of both sides carry the full table; a season now takes 0.36 s. Shortcut: computer-run substitutes could carry only the positions of their formation.
