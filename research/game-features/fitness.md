# Fitness: tiring in a match, recovery between matches, injuries

Researched 2026-11-08. **Status:** part 1 below is built as `sim/fatigue.rs` (stamina only; work rate is left out for now, by decision), tuning figures in `simulation-balance.md` ("Live match and fatigue"). Part 3 is built except injuries: fitness carries from match to match (`fitness.rs`, section 8). Part 2 (work rate) and injuries are not built. Researched before any code. The questions: how fast do real players tire, what decides it (stamina, work rate, position, possession, age), and how long do they take to recover. Sources are mostly sports-science papers; each figure says where it comes from. Figures marked *(guess)* are ours, to be tuned with the simtool.

## 1. What the data has

- **Stamina** (`power_stamina`, 0-99) is loaded as `Attr::Stamina` and used in the overall formula (about 5-10% of most positions).
- **Work rate** (`work_rate` in `male_players.csv`) is in every edition FIFA 15 to FC 24. It is a pair, attacking/defensive, each Low, Medium or High (9 combinations). The importer and the engine ignore it. Counts over all editions: Medium/Medium 98k, High/Medium 31k, Medium/High 16k, High/High 9k, Medium/Low 8.7k, High/Low 7.2k, Low/Medium 4.6k, Low/High 4.3k, Low/Low 0.4k.
- **Work rate follows stamina.** Mean stamina by work rate: High/High 78, Medium/High 71, High/Medium 70, High/Low 65, Low/Medium 63, Medium/Low 62, Medium/Medium 58. A hard worker mostly is a high-stamina player already, so using both to make him tire faster counts the same trait twice.
- **Work rate is not in the overall formula** (FIFA's own overall ignores it), so a player's rating does not reward it. Making hard workers tire faster with nothing in return would punish them.
- **Age** is in the data. There is no per-player fitness, injury or match-load data in the CSVs, so fitness state has to be created by the game.

## 2. Tiring during a match

- **Running falls through the match.** High-intensity running is lower in the last 15 minutes than in the first 15 in every study found: about 20% less for wide midfielders in the Premier League (Bradley et al. 2009, with recovery between sprints 28% longer), 35-45% less across two standards of play and all positions, and 57% less in a match played at 31 degrees ([PubMed 21029199](https://pubmed.ncbi.nlm.nih.gov/21029199/)).
- **Not all of it is fatigue.** One study estimates that 57.9% of the usual decline is not physical fatigue but more interruptions as the match goes on: the ball is in play about 66% of the time in the first 15 minutes and 56% in the last 15 ([JSSM 17(4) 662](https://jssm.org/17-4-662.p_d_f)). Pacing, score and tactics also explain part of it. So the physical part is about 40% of the headline figures: a drop of roughly 8-18% in high-intensity running by the end of a match for an ordinary professional.
- **Our goals already contain it.** The model's minute curve is fitted to the real split of goals (44.2% in the first half), which includes whatever fatigue does to scoring. A fatigue model that lowers every tired player's rating on top of that would lower total goals, and would move the first-half share. Fatigue has to be neutral for an average team, and change results only through differences: a tired side against a fresh one, a high-stamina player against a low-stamina one. After building it, goals per match, the first-half share and the table points have to be tuned back to the targets in `simulation-balance.md`.

## 3. What decides how fast a player tires

**Position.** Total distance is about 9-12 km a match. Central midfielders cover the most (about 11 km), full-backs about 10, central defenders the least (about 9). Wide midfielders, wingers and full-backs do the most high-speed running; central defenders the least at high speed. Sources: [PMC4633253](https://pmc.ncbi.nlm.nih.gov/articles/PMC4633253), [PMC9536366](https://pmc.ncbi.nlm.nih.gov/articles/PMC9536366), [PlayerData](https://playerdata.com/blog/positional-variations-in-match-day-distance). I did not find a single table across leagues; thresholds and tracking systems differ between studies.

**Possession.** Bradley et al. (2013, 810 Premier League players, 54 matches): teams with a lot of the ball ran about 22% less at high intensity when they did not have it, and about 31% more at high intensity with it; total distance did not differ between high and low possession teams (unverified: the percentages are from a press summary of the paper, to check against it). A five-season Premier League study with optical tracking (1675 matches, Allen et al., Biology of Sport 2026) finds that success goes with possession and with more running out of possession, and that running in possession relates negatively to results. So possession changes the shape of the load, not the total: the side without the ball does more sprinting, and the side with the ball does less. The effect is modest, so in the model it is a small adjustment *(guess)*.

**Stamina.** The attribute is the natural input. It is a 0-99 rating that FIFA fits to the player's endurance; no paper gives a mapping to minutes, so the scale is ours *(guess)*.

**Age.** Older players recover more slowly and lose speed. No usable figure was found for elite men (unverified that age matters beyond the attribute); use the attribute (stamina already declines with age in the data) and add nothing.

**Work rate.** No study maps FIFA's work rate to anything. It is a tactical instruction in the game (how much a player moves up and back), so it is closest to *effort*: harder workers give more while fresh and tire faster. See the design below.

## 4. Recovery between matches

- Measures return in stages: jump height recovers within about 48 hours, sprint performance and match-running about 48-72 hours; muscle-damage markers and soreness stay up longer. With at least 72 hours between matches no difference in physical performance was found; under 72 hours in congested periods, measures stay suppressed ([review](https://core.ac.uk/download/227102464.pdf), [Frontiers in Physiology 2021](https://www.frontiersin.org/journals/physiology/articles/10.3389/fphys.2021.685804/pdf)).
- Acute fatigue in the 72-96 hours before a match goes with a higher injury rate and severity ([Sports Injury Bulletin](https://www.sportsinjurybulletin.com/anatomy/fatigue-increases-injury-risk-in-soccer)). Whether cumulative fatigue does is largely unknown.
- Consequence for the game: a player who played 90 minutes three days ago starts at about full fitness; one who played two days ago, or in extra time, starts lower. This needs state stored between matches (`World`), which the match model does not have.

## 5. Injuries

- UEFA Elite Club Injury Study (4483 injuries in 566,000 hours of exposure): about 8.0 injuries per 1000 hours overall; 27.5 per 1000 hours in matches against 4.1 in training; reviews give a range of 16.6-42.0 per 1000 match hours ([UEFA injury study record](https://lida.sport-iat.de/ta/Record/4023585?lng=en), [BJSM 55(14)](https://bjsm.bmj.com/content/55/14/794.full.pdf)). That is about 0.04 injuries per player per 90-minute match, about 0.45 per team per match.
- Injury risk rises through each half ([UEFA study](https://lida.sport-iat.de/ta/Record/4023585?lng=en)); no minute-by-minute table was found.
- Hamstring injuries are 19% of all (24% in 2021/22) and rising 3.9% a year in matches ([PMC9985757](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC9985757/)); a short period of higher-than-usual running raises the odds of a muscle injury ([PMC10765438](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC10765438/)).
- Not found: a clean estimate of injury risk by age for senior professionals, how long injuries last by type in a form we can use, and the share of injuries that force a substitution (many are minor).

## 6. The proposed design (part 1 and most of part 3 are built, see section 8; part 2 and injuries are not)

Three parts, each its own swappable model like the others, in this order.

**Part 1: tiring in the match (`FatigueModel`, in `sim/`).**
- Each player has a *freshness* from 1.0 at kick-off, falling through the match. His effective rating in the slot is his rating times a factor of freshness, so tired players matter less in the team strengths, shot weights and fouls.
- Drain per minute = base × position factor × stamina factor × possession factor *(all guesses to tune)*. Position: central midfielders and wide players 1.1, forwards 1.0, central defenders 0.85, goalkeeper 0. Stamina: a 90 tires about half as fast as a 50. Possession: ±5% with the ball share.
- Neutral for an average side: the base is set so an average squad loses what is already in the calibrated minute curve (section 2), and the simtool checks goals, the first-half share and points stay on target.
- The substitution model reads freshness to choose who goes off (the tiredest first, with the position weights as a tie-break), which is the change you asked to wait for.

**Part 2: work rate as effort.** Each of the nine combinations maps to an effort level (Low 0.9, Medium 1.0, High 1.1 *(guess)*) for attacking and defensive work separately. Effort multiplies the player's contribution while fresh (small boost to midfield and defence ratings for defensive work, to shot and assist weights for attacking work) and multiplies his drain by the same factor. A hard worker gives more early and fades sooner; nobody is punished without a return. Drain from stamina and effort together must not make High/High players tire faster than the attribute alone suggests, since they are already high-stamina players (section 1): effort acts on drain with half the strength of stamina *(guess)*.

**Part 3: fitness between matches and injuries.**
- A stored fitness (0-100) per player: matches played lower it by minutes played and extra time, days of rest restore it (about full recovery in 72 hours, section 4); it sets the freshness a player starts a match with.
- Injuries from the rate in section 5, with risk rising through the match and with low freshness; an injury ends the player's match (a forced substitution) and keeps him out for a number of days. The lineup picker skips injured players and (optionally) rotates tired ones.
- Needs `Player`/`World` state, the calendar (days between matches), and the lineup picker. It is the largest part.

## 7. Open questions

1. Does the user want work rate used as effort (part 2), or stamina only?
2. Fatigue model scale: no source maps stamina to minutes. The tuning check is the simtool (late-goal share, goals, points) plus a test that a tired side loses more than a fresh one.
3. Injuries: how often do minor injuries force a substitution? Data not found.
4. Should the user control rest and rotation (part 3 and the GUI)?

## 8. What is built of part 3 (fitness between matches)

- `Player.fitness` (0 to 100) and `Player.fitness_day` (the day of the season it was worked out for; `Fixture::day_number` is week times 7 plus the days after Saturday). Rest is worked out lazily: `Player::fitness_on(day)` brings back half of the fitness still missing every 2 days (`HALF_LIFE_DAYS`), so the first day gives the most and each later day less: after a full match (about 44 points lost) a player is at about 69 one day later, 84 after three days, 96 after a week. Real running is back to normal in about 72 hours (section 4) but tiredness lingers; the curve is our guess.
- **A match costs fitness** by the same tiring as in the match (`sim::fitness_loss`): 80 points times how tired the model says he got, so a midfielder with average stamina loses about 44 in 90 minutes, a centre-back less, a high-stamina player less, extra time more, a goalkeeper nothing. `World::record_match` applies it with the minutes each player played, so a watched match, a played match and a cheap-model match with detail all do the same.
- **A tired start**: `MatchPlayer.fitness` becomes starting tiredness (0.6 at fitness 0, in proportion), so a player at 50 fitness starts as tired as one 25 minutes in (about 3 rating points) and gets more tired from there. The `None` model ignores it.
- **The game rests worn-out players**: the automatic lineup picks by rating minus what the tired start costs (`sim::rating_cost_of_fitness`), so an AI club (and your club on Automatic) rests a player when a fresh one is nearly as good. `choose_formations` ignores fitness.
- **Wear** (a run of matches lowers the fitness he can recover to): `Player.matches_in_a_row` counts matches of 45 minutes or more, each within 7 days of the one before; his ceiling is 100 minus wear, wear is 1.5 points for each match in the run beyond 3, at most 15. A week without a match ends the run and clears the wear completely, so resting a tired player for one match gives his recovery back. Stamina shifts both: each point above 70 shortens the recovery half-life (about 0.75 times at 90, 1.33 times at 50) and cuts the wear (0.67 times at 90, about 1.5 times at 50), settings `recovery_stamina_effect` and `wear_stamina_effect`. An average ever-present in a league season therefore recovers to about 85% instead of 96%, a player who is rested every few weeks does not. Goalkeepers do not tire, so they have no run. (First tried as a 7-day window of the match load, which never bit: recovery is fast enough that a player who has just played twice is still below any ceiling.)
- **Settings**: every number between matches is in `FitnessSettings` (`fitness.rs`), kept on `World::fitness_settings`: recovery half-life 2 days at stamina 70, stamina effects 0.0144 on recovery and 0.02 on wear, fitness lost when fully spent 80, tired start 0.6 at zero fitness, run ends after 7 days, free matches 3, wear per match 1.5, wear maximum 15, minimum minutes 45. The numbers of the tiring inside a match are constants in `sim/fatigue.rs`. All are guesses. The simtool has no flags for them yet; a test shows a setting changes the result the expected way.
- **Timing**: `World::rest_for(fixture)` brings both clubs up to the fixture's day, called when a match is played and when the next match is opened to watch; `generate_fixtures` (a new season) resets everyone to 100, so the summer is full rest. `play_week` plays a week's fixtures in day order.
- **Shown**: the lineup cards and lists ("fit 87%"), the squad table (Fit) and the player page, coloured green above 75, yellow 60 to 75, orange 45 to 59 and red below. The screens assume today is the day of your club's next match, because that is when you look at the lineup: after each match you play, `World::rest_to_next_match(club)` rests your squad to that day, so the number shown is the fitness the match will use. The game has no days between matches yet; a calendar you move through (planned) would replace this.
- **Effect on results**: none in the simtool (leagues are weekly, so everyone is back to 100 by the next match); it shows with cup congestion and when the user plays a tired side.
- Shortcut: the loss uses his main position and an even ball share, not the slot he played.
- Not built: a simtool flag per setting, a settings screen, injuries (section 5), the `Fitness` of other competitions' calendars (Europe), rotation advice, training, age (older players recovering slower), the cheap model's matches (no minutes, so no fitness cost).
