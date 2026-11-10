# Fitness: tiring in a match, recovery between matches, injuries

Researched 2026-11-08, built 2026-11-08 to 2026-11-10: tiring in the match (`sim/fatigue.rs`, tuning in `simulation-balance.md`, "Live match, fatigue and tactics") and fitness between matches (`engine/src/fitness.rs`). Not built: work rate and injuries. The questions: how fast do real players tire, what decides it (stamina, work rate, position, possession, age), and how long do they take to recover. Sources are mostly sports-science papers; each figure says where it comes from. Figures marked *(guess)* are ours. The numbers that live in code (`FitnessSettings`, the constants in `sim/fatigue.rs`) are not copied here.

## 1. What the data has

- **Stamina** (`power_stamina`, 0-99) is loaded as `Attr::Stamina` and used in the overall formula (about 5-10% of most positions).
- **Work rate** (`work_rate` in `male_players.csv`) is in every edition FIFA 15 to FC 24. It is a pair, attacking/defensive, each Low, Medium or High (9 combinations). The importer and the engine ignore it. Counts over all editions: Medium/Medium 98k, High/Medium 31k, Medium/High 16k, High/High 9k, Medium/Low 8.7k, High/Low 7.2k, Low/Medium 4.6k, Low/High 4.3k, Low/Low 0.4k.
- **Work rate follows stamina.** Mean stamina by work rate: High/High 78, Medium/High 71, High/Medium 70, High/Low 65, Low/Medium 63, Medium/Low 62, Medium/Medium 58. A hard worker mostly is a high-stamina player already, so using both to make him tire faster counts the same trait twice.
- **Work rate is not in the overall formula** (FIFA's own overall ignores it), so a player's rating does not reward it. Making hard workers tire faster with nothing in return would punish them.
- **Age** is in the data. There is no per-player fitness, injury or match-load data in the CSVs, so fitness state has to be created by the game.

## 2. Tiring during a match

- **Running falls through the match.** High-intensity running is lower in the last 15 minutes than in the first 15 in every study found: about 20% less for wide midfielders in the Premier League (Bradley et al. 2009, with recovery between sprints 28% longer), 35-45% less across two standards of play and all positions, and 57% less in a match played at 31 degrees ([PubMed 21029199](https://pubmed.ncbi.nlm.nih.gov/21029199/)).
- **Not all of it is fatigue.** One study estimates that 57.9% of the usual decline is not physical fatigue but more interruptions as the match goes on: the ball is in play about 66% of the time in the first 15 minutes and 56% in the last 15 ([JSSM 17(4) 662](https://jssm.org/17-4-662.p_d_f)). Pacing, score and tactics also explain part of it. So the physical part is about 40% of the headline figures: a drop of roughly 8-18% in high-intensity running by the end of a match for an ordinary professional.
- **Our goals already contain it.** The model's minute curve is fitted to the real split of goals (44.2% in the first half), which includes whatever fatigue does to scoring. A fatigue model that lowers every tired player's rating on top of that would lower total goals and move the first-half share. Fatigue has to be neutral for an average team and change results only through differences: a tired side against a fresh one, a high-stamina player against a low-stamina one. After building it, goals, the first-half share and the table points were tuned back to the targets in `simulation-balance.md`.

## 3. What decides how fast a player tires

**Position.** Total distance is about 9-12 km a match. Central midfielders cover the most (about 11 km), full-backs about 10, central defenders the least (about 9). Wide midfielders, wingers and full-backs do the most high-speed running; central defenders the least at high speed. Sources: [PMC4633253](https://pmc.ncbi.nlm.nih.gov/articles/PMC4633253), [PMC9536366](https://pmc.ncbi.nlm.nih.gov/articles/PMC9536366), [PlayerData](https://playerdata.com/blog/positional-variations-in-match-day-distance). No single table across leagues was found; thresholds and tracking systems differ between studies.

**Possession.** Bradley et al. (2013, 810 Premier League players, 54 matches): teams with a lot of the ball ran about 22% less at high intensity when they did not have it, and about 31% more at high intensity with it; total distance did not differ between high and low possession teams (unverified: the percentages are from a press summary of the paper). A five-season Premier League study with optical tracking (1675 matches, Allen et al., Biology of Sport 2026) finds that success goes with possession and with more running out of possession, and that running in possession relates negatively to results. So possession changes the shape of the load, not the total: the side without the ball does more sprinting. The effect is modest, so in the model it is a small adjustment *(guess)*.

**Stamina.** The attribute is the natural input: a 0-99 rating that FIFA fits to the player's endurance. No paper gives a mapping to minutes, so the scale is ours *(guess)*.

**Age.** Older players recover more slowly and lose speed. No usable figure was found for elite men; the model uses the attribute (stamina already declines with age in the data) and adds nothing.

**Work rate.** No study maps FIFA's work rate to anything. It is a tactical instruction in the game (how much a player moves up and back), so it is closest to *effort*: harder workers give more while fresh and tire faster.

## 4. Recovery between matches

- Measures return in stages: jump height recovers within about 48 hours, sprint performance and match-running about 48-72 hours; muscle-damage markers and soreness stay up longer. With at least 72 hours between matches no difference in physical performance was found; under 72 hours in congested periods, measures stay suppressed ([review](https://core.ac.uk/download/227102464.pdf), [Frontiers in Physiology 2021](https://www.frontiersin.org/journals/physiology/articles/10.3389/fphys.2021.685804/pdf)).
- Acute fatigue in the 72-96 hours before a match goes with a higher injury rate and severity ([Sports Injury Bulletin](https://www.sportsinjurybulletin.com/anatomy/fatigue-increases-injury-risk-in-soccer)). Whether cumulative fatigue does is largely unknown.

## 5. Injuries (not built)

- UEFA Elite Club Injury Study (4483 injuries in 566,000 hours of exposure): about 8.0 injuries per 1000 hours overall; 27.5 per 1000 hours in matches against 4.1 in training; reviews give a range of 16.6-42.0 per 1000 match hours ([UEFA injury study record](https://lida.sport-iat.de/ta/Record/4023585?lng=en), [BJSM 55(14)](https://bjsm.bmj.com/content/55/14/794.full.pdf)). That is about 0.04 injuries per player per 90-minute match, about 0.45 per team per match.
- Injury risk rises through each half ([UEFA study](https://lida.sport-iat.de/ta/Record/4023585?lng=en)); no minute-by-minute table was found.
- Hamstring injuries are 19% of all (24% in 2021/22) and rising 3.9% a year in matches ([PMC9985757](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC9985757/)); a short period of higher-than-usual running raises the odds of a muscle injury ([PMC10765438](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC10765438/)).
- Not found: a clean estimate of injury risk by age for senior professionals, how long injuries last by type in a form we can use, and the share of injuries that force a substitution (many are minor).
- Plan: injuries from the rate above, with risk rising through the match and with low fitness; an injury ends the player's match (a forced substitution) and keeps him out for a number of days; the lineup picker skips injured players. Wear (section 6) is the natural thing to raise the risk.

## 6. What was built

Each part is its own swappable model or settings struct, like the others.

**Tiring in the match** (`FatigueModel { None, ByStamina }`, stamina only, by decision). Each player has a tiredness from 0 at kick-off to 1, growing through the match with minutes (slowly at first, faster late), faster for a low stamina, for a wide or central midfielder than a centre-back (goalkeepers do not tire) and for the side without the ball. It lowers his rating in the slot, so tired players matter less in the team strengths, shot weights and fouls, and the substitution model takes the tiredest off first. All numbers are guesses, tuned so an average side is unchanged. Work rate is left out: if it is added it would be effort (Low 0.9, Medium 1.0, High 1.1 *(guess)*): more contribution while fresh and faster tiring, with stamina and effort not double-counted (section 1).

**Fitness between matches** (`engine/src/fitness.rs`; every number is in `FitnessSettings` on `World::fitness_settings`, all guesses; a test shows each setting moves the result the expected way).
- **A match costs fitness** (0 to 100) by the same tiring as in the match (`sim::match_tiredness`), so a midfielder with average stamina loses about 44 points in 90 minutes, a centre-back less, a high-stamina player less, extra time more, a goalkeeper nothing. `World::record_match` applies it, so a watched and a played match do the same.
- **Rest** is worked out lazily from the days since the last match (`Fixture::day_number` is week times 7 plus the days after Saturday): half of the fitness still missing comes back every 2 days, so the first day gives the most (about 69 after one day, 84 after three, 96 after a week, from a full match). Real running is back to normal in about 72 hours (section 4) but tiredness lingers; the curve is our guess. Stamina shortens the half-life and cuts wear.
- **A tired start**: fitness becomes starting tiredness (`MatchPlayer::start_tiredness`), so a player at 50 fitness starts as tired as one 25 minutes in. The automatic lineup picks by rating minus what the tired start costs, so an AI club, and yours on Automatic, rests a worn-out player when a fresh one is nearly as good.
- **Wear**: the more matches a player plays in a row (each of 45 minutes or more, within 7 days of the one before), the lower the fitness he can recover to (a ceiling of 100 minus wear). A week without a match clears it, so resting a tired player for one match gives his recovery back; goalkeepers have no run. An average ever-present in a league season recovers to about 85% instead of 96%. (First tried as a 7-day window of the match load, which never bit: recovery is fast enough that a player who has just played twice is still below any ceiling.)
- **Timing**: `World::rest_for(fixture)` brings both clubs up to a fixture's day when it is played or opened to watch; `generate_fixtures` (a new season) resets everyone to 100, so the summer is full rest.
- **Shown** (lineup, squad table, player page): fitness coloured green above 75, yellow 60 to 75, orange 45 to 59, red below. The screens assume today is the day of your next match, because that is when you look at the lineup: after each match you play, `World::rest_to_next_match` rests your squad to that day, so the number shown is the fitness the match will use. A calendar you move through (planned) would replace this.
- **Effect on results**: none in the simtool (leagues are weekly, so everyone is back to about 96); it shows with cup congestion and when the user plays a tired side.
- Shortcut: the loss uses his main position and an even ball share, not the slot he played.
- Not built: a simtool flag per setting, a settings screen, injuries, rotation advice, training, older players recovering slower, and the cheap model's matches (no minutes, so no fitness cost).
