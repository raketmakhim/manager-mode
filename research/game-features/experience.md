# Experience and player development

Researched 2026-11-08. **Status:** built (section 6): your club only, match XP only, an XP bar that levels a player up at 100, as decided. Sections 1 and 4 are the situation and the proposal before it was built; section 6 is what exists. Researched before any code. The questions: how do real players change over a career, how does FIFA/EA FC career mode make players grow, and how should experience (XP) work here so the user's players can drift from the real ratings. The age curves below are measured from our own ten editions (FIFA 15 to FC 24, `male_players.csv`), with `age_curves.py` in this folder (rerun it if the data changes).

## 1. What the game did before experience

- A new career starts from the real ratings of the chosen edition. At the end of a season `advance_ratings` replaces every player's ratings by the real ones of the next edition, when the database has one ("No newer data: player ratings are unchanged" otherwise). So everyone follows the real career until FC 24, nobody develops because of what happens in our simulation, and after the last edition nothing changes at all.
- `Player` has `form` (50 neutral) and `fitness` (100 full) fields that nothing uses yet, and `potential`.
- Match stats exist per player (minutes, goals, assists, average rating, player of the match), so there is something to measure experience from.

## 2. How real players change (measured from the data)

Mean change in overall from one edition to the next, by age at the first edition (n = players with a next edition, so players who left the data are not counted; sd of a single player's change about 2-3 points at every age):

| Age | 17 | 19 | 21 | 23 | 25 | 27 | 29 | 31 | 33 | 35 |
|---|---|---|---|---|---|---|---|---|---|---|
| Change | +3.0 | +2.6 | +2.0 | +1.2 | +0.7 | +0.4 | +0.1 | -0.2 | -0.5 | -0.7 |

- **Potential matters most while young.** Age 18-20: +2.2 with a gap of 5 to potential, +2.4 with 10, +2.8 with 15, +3.3 with 20 or more. Age 21-23: +1.3 with no gap, +1.5 with 5, +2.0 with 10 or more. Age 24-26: +0.6 with no gap, +1.1 with a gap of 5 or more.
- **Position:** goalkeepers develop later and decline later (24-26: +1.0; 30-32: +0.1; outfield players 30-32: -0.2 to -0.3; 36 and over: GK -0.5, attackers -0.9). Outfield positions are close to each other.
- **Attributes age differently.** Mean change per edition by age band (17-20 / 21-24 / 25-28 / 29-32 / 33+): stamina +2.4 / +1.5 / +0.2 / -1.1 / -2.3; sprint speed +1.1 / +0.3 / -0.7 / -1.7 / -2.4; acceleration +1.1 / +0.3 / -0.6 / -1.6 / -2.2; agility +2.0 / +0.9 / +0.1 / -0.6 / -1.1; finishing +1.8 / +1.1 / +0.4 / 0.0 / -0.4; short passing +2.6 / +1.4 / +0.6 / +0.1 / -0.1; vision +2.4 / +1.6 / +0.8 / +0.4 / +0.4; composure +3.0 / +2.0 / +1.1 / +0.5 / +0.2; strength +2.2 / +1.3 / +0.7 / +0.3 / -0.1. Physical attributes (pace, stamina, agility) fall from the late twenties; mental and passing attributes keep rising until the early thirties. This is also what the Swedish and Champions League studies report: physical performance falls after 30 while passing ability rises ([Frontiers in Psychology 2019](https://www.doi.org/10.3389/fpsyg.2019.00076), [PMC6360147](https://pmc.ncbi.nlm.nih.gov/articles/PMC6360147)), with an overall peak around 25-27, goalkeepers and centre-backs later (about 31).
- Caveat: the data is FIFA's rating of players, which is its own scouts' view, not a measurement of ability.

## 3. What EA FC does

- Until FC 24 growth was mostly fixed: age, potential and (for the user's own players) a development plan or training. FC 25 added *dynamic potential* and **Match XP**: players earn XP from their actions and performance in matches, reaching or missing their potential depending on playing time and form; training intensity speeds growth but costs stamina ([Sportskeeda](https://sportskeeda.com/esports/how-dynamic-potential-work-ea-fc-25-explained), [Space Coast Daily](https://spacecoastdaily.com/2025/04/fc-25-career-mode-gets-smarter-how-the-new-player-growth-system-works/); third-party guides, details unverified). EA's older notes say development speed depends on age, abilities, potential, form and playtime ([EA FIFA 21 career mode deep dive](https://www.ea.com/news/pitch-notes-fifa-21-career-mode-deep-dive)). Players over 30 decline whatever they do, and development plans only slow it ([EA FIFA 23 player development](https://www.ea.com/news/fifa-23-player-development)). Reports for FC 27 say form, morale and match fitness will feed ratings and a better coach will judge potential more accurately ([Red Bull](https://www.redbull.com/gb-en/ea-fc-27-career-mode-new-features); pre-release, unverified).
- Player complaints: a decline driven only by age feels unfair to a player who is in form; a decline with no way to slow it is what most dislike (forum posts, opinion).

## 4. The first proposal (superseded: the season-end drift became the XP bar of section 6)

Principle: everyone follows the real career by default, and only experience *different from what the real career implies* makes a player drift. That keeps the history faithful and lets the user change it.

**Experience (XP) of a season**, for each player, from what he did in our simulation:
- **Match XP:** minutes played (per 90), his average rating against the 6.3 base, goals and assists, a bonus for a player of the match. Match XP is capped per match so a few big games do not decide a season.
- **Training XP** (later): a training focus per player or per position group, with an intensity that costs fitness.
- **Expected XP** for the same player in the real career: what a player of his age and rating normally earns, taken from the minutes he played at his club in the real career if we have them, otherwise from his overall against his club's squad (the top 11 are starters). Season **drift** = XP above or below expected, turned into attribute points with a small factor.

**At the end of a season** each player's ratings become: the real next edition's ratings (the baseline) plus his accumulated drift, limited by his potential (potential itself moves a little with drift: dynamic potential). A player who is a regular starter and plays well drifts up; one who sits on the bench drifts down (a young player more than an old one); a player who plays as the real career says drifts nothing.

**After the last real edition** (FC 24, 2023/24), or for a player the data does not follow (retired, left the database), the baseline is the simulated one: the age curve of section 2 per attribute (age band and potential gap), plus drift. This is also what lets a career run for 20 years.

**Which players:** drift for the user's club only at first (cheap; the rest of the world follows the real data), then every club if the simulation speed allows. Attribute-level change uses the age bands above, so a player who gets better at 22 gets better at the attributes that grow at 22, and an ageing winger loses pace before passing.

**Visible to the user:** a squad view column with the season's XP and the expected figure, a "development" line per player (rising, as expected, stalling) and the end-of-season attribute changes.

## 5. Open questions (answered 2026-11-08: the user's club only; match XP only; an XP bar that levels up at 100 and keeps the remainder; XP is never negative; potential is a soft limit. Strength and the minutes baseline were replaced by the bar and are tuned in section 6)

1. Drift for the user's club only, or every club (the world then diverges from reality over time)?
2. Match XP only first, or training as well (needs a training screen and a fitness cost, part 3 of `fitness.md`)?
3. Is a visible XP bar wanted, or a plain "development" summary at the end of the season?
4. How strong should drift be against the real baseline (a regular starter gaining +1 a season over his real career, say)? No source gives a number; tune with the simtool by checking that a squad of regular starters ends a 10-season career a few points above the real one and a benched squad a few below.
5. Which minutes data to use for "expected": we have none for the real career (only ratings), so the design above uses the squad rank as the stand-in.

## 6. What was built (2026-11-08)

Decisions made: only the user's club levels up, match XP only, XP is worth less to a high overall, potential is a soft limit, and XP fills a bar: at 100 XP the player levels up and the XP over the line stays in the bar.

`engine/src/development/` (`ExperienceModel`: `None`, `MatchBased`, default `MatchBased`). An experience model is two pure functions: `match_xp` (the XP of one match) and `level_points` (the overall points one level gives). Every match gives each player XP, added to his bar (`Player::xp`, 0 to 99 after levelling, and to the season total in `PlayerStats::xp`). `World::level_up_club(club, rng)` levels up the user's players whose bar is full; the GUI calls it after every match it plays, watches or simulates, and says who levelled up. The automatic formation is chosen again, as the squad's strength changed. Ratings later move on to the real next edition as a difference (`follow_real_change`), so the levels stay. Everyone else follows the real data.

**Match XP** (`match_based.rs`; every number is our own design). 1 to 10 XP a match, shown in the squad view (bar, last match), on the player's page and in a "Match XP" table at full time in the live match window:
- before the overall factor: 1 for turning up, 4 x the share of the match played, 3 x form, 2 for player of the match (at most 10). Form is the match rating against what an uneventful match is worth to his overall and position (`rating::neutral_rating`) plus what a regular of his line gets above that (forwards 0.33, goalkeepers 0.32, midfielders 0.16, defenders 0.12, measured over a simulated FIFA 15 season of 5300 regulars), a rating point above or below being full form.
- times a factor that falls with his overall: 1 up to 54, `(99 - overall) / 45` below that down to 0.3 at 90 and over. So a 55 gets about 4-5 XP a match, a 70 about 3.5, an 85 or better about 2.
- Measured over a simulated season: 4.0-4.3 XP a match for overalls 50-69, 3.3 for 70s, 2.4 for 80s, 2.2 for 90s; the last match of every player in the world ranged from 1 to 10, mostly 2 to 5. A regular of 4 XP a match fills the bar about once every 25 matches.

**A level** gives 1.0 overall point to a player of 21 or younger who is below his potential; the points fall to a quarter by 33. Past his potential a level is worth exp(-points over / 6) of that: a third at 6 over and a seventh at 12 over, so a player who keeps winning keeps rising, slowly. Potential itself is not raised (the real data moves it). XP is never lost: there is no negative XP, and a player who does not play just does not level.

**Applied** to every attribute that counts for his position (each by the level's points, rounded at random so the average is exact), and the overall by what those are worth (`overall_change`, also rounded at random). So the squad view, the lineup, the team strength and the match simulation all read the same new overall (`rating_at` at his own position is his overall, and at other positions follows his attributes); a test checks the overall and the attributes move together. Shortcut: every attribute moves equally, where real players grow different attributes at different ages (section 2).

Measured (simulated careers from FIFA 15, a mid-table Premier League club, 8 seasons, levels applied at the end of each season): about 11 to 17 levels a season over the squad; the best eleven ends at about 1.7 points above the same club left to follow the real data with a level worth 0.7 (1.0 now, so about 2.4). Not tuned against anything: there is no real figure for how much playing time changes a career.

Not built: training XP, potential rising with levels, a growth profile per attribute and age, and levels for clubs other than the user's.
