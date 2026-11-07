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
