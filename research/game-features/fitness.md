# Fitness: tiring in a match, recovery between matches, injuries

Researched 2026-11-08. **Status:** part 1 below is built as `sim/fatigue.rs` (stamina only; work rate is left out for now, by decision), tuning figures in `simulation-balance.md` ("Live match and fatigue"). Parts 2 and 3 are not built; `Player.fitness` exists in the model but nothing uses it yet and is the natural starting fitness for part 3. Researched before any code. The questions: how fast do real players tire, what decides it (stamina, work rate, position, possession, age), and how long do they take to recover. Sources are mostly sports-science papers; each figure says where it comes from. Figures marked *(guess)* are ours, to be tuned with the simtool.

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

## 6. Proposed design (for review; nothing built)

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
