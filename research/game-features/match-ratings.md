# Player match ratings: how they are built, and what we should do

Researched 2026-11-07. A match rating is a number for each player who took part, usually 0 to 10, shown after the match (and averaged over a season).

**How reliable this is.** The providers keep their formulas private. What I could read in full is Sofascore's own explainer; the Football Manager article I tried to read was cut off by a subscription wall, so I have **nothing citable on how Football Manager calculates it**. I did not find Opta's method at all. So this note records the structure that is public, and the weights are **our own design**, not a copy of anyone's.

## 1. What is public

**Sofascore** ([article, October 2025](https://www.sofascore.com/news/sofascore-rating-breakdown-what-goes-into-calculations)):
- Hundreds of actions are tracked, and an algorithm weighs each action by its **context and impact**, not by a plain count.
- Five categories: **shooting** (how the quality of a player's shots changed his team's chance to score), **passing** (progressive or riskier passes count for more than safe backward ones; a long pass that creates a goal is worth more than a short one before a routine shot), **dribbling** (carries that improved attacking potential), **defending** (how well he stopped the other team scoring) and **goalkeeping**.
- There is a match rating and a season rating (an average); the highest season average decides Player of the Season.
- From other pages the search showed: every player starts at **6.5**, and a rating appears after about 10 minutes of play. The article itself gives no formula, no weights, no position rules and does not say whether the team result matters.

**WhoScored**: according to a third-party guide ([Footyroom](https://footyroom.co/articles/whoscored-and-sofascore-the-main-resources-for-player-ratings-669e5b1cf6cd5044b86e6193)), a rating starts at **6.0**, moves up or down with the player's actions, and, unlike Sofascore, also takes the **team's result** into account.

**Football Manager**: community sources describe the scale as compressed around a few numbers, a single goal lifting a poor game's rating and a single error dropping it, and recommend not judging a player on one rating. These are opinions, not documentation.

## 2. What we can take from it

1. **Start from a base** (6.0 to 6.5) and move up and down with what the player did. Both providers do.
2. **Weight actions by how much they changed the match**, not by counting. A goal in a 1-0 win matters more than a fourth goal in a 5-0.
3. **Position matters.** A defender is judged on defending, a goalkeeper on saves and goals conceded, a striker on shots and goals. A flat points system would make strikers always top.
4. **The team result nudges everyone** a little (WhoScored does this); it is also what makes a clean sheet or a heavy defeat show in a defender's number.
5. **Short appearances need care.** A substitute playing 10 minutes should not be rated like a starter.

## 3. A first design for our game

Our full-detail engine records every shot, goal, assist, save, foul and card (see [match-simulation-methods.md](match-simulation-methods.md)), so a rating can be calculated from those events and nothing else.

- **Rating = base + the sum of event contributions + a small team-result term**, clamped to 3.0 to 10.0 and shown to one decimal.
- **Base 6.0**, so an uneventful starter ends near 6 and the usual spread sits between 5.5 and 8.
- **Event contributions (starting values to tune, not sourced):** goal about +1.0 (more for the winner, less in a heavy win), assist about +0.7, shot on target a small plus, a saved shot by a goalkeeper a plus, goal conceded a small minus for the defenders and goalkeeper on the pitch, clean sheet a plus for the back line and goalkeeper, yellow card a small minus, red card a large minus, penalty missed a minus.
- **Position shapes it.** The same contribution counts for more where it is rarer for that position (a defender's goal is worth more than a striker's), and goalkeepers and defenders get a clean-sheet term.
- **Team result:** about +0.3 for a win and -0.3 for a defeat.
- **Minutes:** the contribution is scaled by the share of the match played; a player under 10 minutes gets no rating, as on Sofascore.
- **Player of the match** is the highest rating of the winning side.
- **Season rating** is the minutes-weighted average, shown in the squad view next to appearances, goals and assists.

## 4. How we would check it

There is no real match-level rating data in our downloads. So the check is on the **shape of the distribution**, from public knowledge of how these ratings behave, to be confirmed against a real source:
- the mean of starters is near 6.5;
- most starters fall between 5.5 and 7.5; fewer than 1 in 20 above 8.5;
- strikers do not systematically beat defenders and midfielders by more than about 0.3;
- a team that wins big has a higher average than one that loses big.

If we later get real per-match ratings (for example from an open data set), fit the weights to them instead.

## Open questions

- A real source for the **typical spread of match ratings** (mean and standard deviation) for starters.
- Whether ratings should feed **form and morale** (a hot streak raising `form`): a game design choice, not a research one.

## 5. What was built (2026-11-08)

`engine/src/rating/`: a `RatingModel` enum (today only `Formula`) and one function, `rate(model, &RatingInput) -> Option<f32>`, like the match models. A model sees one player at a time (his slot, minutes, his match stats and his team's result) and returns 3.0 to 10.0, or `None` under 10 minutes. A new model is a new file, a variant and a match arm; the contract tests in `rating/mod.rs` run for every model in `RatingModel::ALL` (in range, goals and assists raise a rating, cards lower it, a win beats a draw beats a defeat). `stats::match_stats` takes the model, builds each starter's `PlayerStats` for the match, rates him, and names a player of the match (the best rating on the winning side, or of everyone after a draw). Season totals keep the rating sum in tenths, the rated matches and the player-of-the-match count; the average is `PlayerStats::average_rating`. The squad view and the Stats tab show it, and the Stats tab can rank by it (10 rated matches needed) and by player of the match.

The `formula` model: base 6.3, goal +1.2 (times 1.4 for a defender or goalkeeper, 1.15 for a midfielder, 0.9 for an attacker), assist +0.9 (times 1.2 for a defender), a shot on target that did not score +0.06, a save +0.15, yellow -0.3, red -1.5, win +0.4 and defeat -0.4 for everyone, and for what happens at the back, scaled by the share of the match played: a clean sheet +1.1 for a defender, +0.8 for a goalkeeper, +0.15 for a midfielder, a goal against -0.25 for a goalkeeper, -0.1 for a defender, -0.05 for a midfielder. Clamped to 3.0 to 10.0. The weights are our own; they were tuned on the shape of the distribution against the real figures below.

### Real figures found, and the first tuning against them (2026-11-08)

Real season averages (web search; scales differ by provider):
- **Sofascore 2023/24, full season** ([Sofascore](https://www.sofascore.com/news/?p=69351)): Premier League leader Rodri 8.01, Ligue 1 Mbappe 7.89, Serie A Calhanoglu 7.68, Eredivisie Veerman 8.26. Mid-season (January 2024) the top-five-league leaders were Mbappe 8.15, Bellingham 8.08, Kane 8.07, Sane 8.01 ([PSG Talk](https://psgtalk.com/2024/01/mbappe-real-madrid-bayern-munich/)).
- **WhoScored 2014/15, La Liga** (a [Sportskeeda](https://sportskeeda.com/football/europe-top-rated-under-21-xi-2014-15) page and other roundups; sources differ): Messi about 8.4 to 8.8, Ronaldo about 8.5. WhoScored's scale is higher at the top than Sofascore's.
No real figure was found for the spread of single-match ratings or for the mean of all starters (we use about 6.5, the Sofascore starting point).

Ours was compressed at the top: with the first weights the best season average in a simulated world was 7.74 (Muller), only 0.05% of regular players were at 7.5 or more, and none at 8.0. The positive weights were stretched (goal 0.9 to 1.2, assist 0.7 to 0.9, win and defeat 0.3 to 0.4, saves, shots and clean sheets up) and the base lowered from 6.4 to 6.3 to hold the mean. Before and after, 1,500 simulated FIFA 15 league matches and one simulated world season (players with 25+ rated matches):

| | Before | After |
|---|---|---|
| Single match: mean / standard deviation | 6.56 / 0.70 | 6.55 / 0.87 |
| Single match: between 5.5 and 7.5 / above 8.5 / above 9.0 | 86% / 0.7% / 0.2% | 80% / 2.3% / 0.8% |
| Season average: best player | 7.74 | 8.07 |
| Season average: top five | 7.74 to 7.48 | 8.07 to 7.72 |
| Season average: 7.5 or more / 7.0 or more | 0.05% / 3.3% | 0.4% / 6.1% |
| Season average: mean | 6.57 | 6.56 |
| By line: attack / midfield / goalkeeper / defence | 6.79 / 6.55 / 6.62 / 6.45 | 6.82 / 6.53 / 6.66 / 6.41 |

The best simulated seasons now sit at about 8.0, in line with Sofascore's league leaders (7.7 to 8.0); the WhoScored-style 8.4 to 8.8 is out of reach on this scale. Still off: attackers lead defenders by 0.41 (the target was 0.3 at most), and a midfielder like Rodri cannot reach 8.0 through events alone, because real ratings count passing and defending actions that our events do not have.

### The overall-based model (2026-11-08)

The user wanted the player's overall to matter, so a second model, `RatingModel::OverallBased` (`rating/overall_based.rs`), is now the default (`DEFAULT_RATING_MODEL`); `Formula` stays for comparison. Rating = 6.3 + 0.03 per point of the player's rating in his slot above 67 + 0.8 of the `formula` events part (its goals, assists, saves, clean sheets, cards and result, without the base). So a 91 starts 0.7 above a 67 and a 43 starts 0.7 below, and a quiet game puts a star in the middle only if he does nothing. The input carries his slot rating (`RatingInput.overall`, from `MatchDetail.slot_ratings`). It is a design choice, not real data: real providers rate what happened and not who the player is.

Tuning (FIFA 15, 1,500 simulated league matches for the single-match figures, one simulated world season for the season averages; reference 67 was chosen so the whole-world season mean is about 6.5):

| | Formula | Overall-based |
|---|---|---|
| Single match mean / standard deviation | 6.55 / 0.87 | 6.64 / 0.74 |
| Single match between 5.5 and 7.5 / above 8.5 | 80% / 2.3% | 86% / 1.4% |
| Season average: best five | 8.07, 8.02, 7.79, 7.74, 7.72 | 8.30, 8.13, 8.12, 8.07, 8.06 (Muller, Gomes, Robben, Messi, Ribery) |
| Season average: 7.5 or more / 7.0 or more / under 6.0 | 0.4% / 6.1% / 0.8% | 0.8% / 8.9% / 2.4% |

Other settings tried: slope 0.04 with 0.85 of the events gave a best season of 8.57 (too high against Sofascore's 8.0 leaders); slope 0.05 gave 8.75; both are inside WhoScored's range (8.4 to 8.8 for Messi in 2014/15). Attackers lead defenders by 0.35 (6.86 against 6.51).
