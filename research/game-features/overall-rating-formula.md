# How a player's overall rating is built from his attributes

Worked out 2026-11-07 by fitting our own data. Script: [fit_overall.py](fit_overall.py) (`py -I research/game-features/fit_overall.py [edition]`, needs numpy and `history.db`). This is a fit to the data, not EA's published formula, so treat the weights as a very good approximation, not as the truth.

## Finding

A player's overall is a **weighted average of his attributes, with different weights for each position**. For every main position the fit reproduces the stored overall with a mean error of about 0.27 points (about the size of rounding), an exact match for 87% to 96% of players, and within one point for 99% to 100%.

- The weights of each position add up to about 1.0 (0.97 to 1.02), with no constant worth keeping. So "overall = weighted average" is the right picture.
- **The weights do not change between editions.** Striker weights fitted on FC 24 and on FIFA 21 differ by at most 0.01. One set of weights can serve every edition.
- **Goalkeepers** use a different set of attributes (diving, handling, reflexes, positioning at 0.21 each).
- **FIFA 15 cannot be fitted.** It lacks several attributes (composure and others), so no player has the complete set. The formula is still usable there as a change (see below).

## Weights per main position (FC 24)

Under 0.02 left out. Names are the column name without its category (`marking_awareness` is `defending_marking_awareness`).

- **ST** (2457 players, exact 94%, within 1: 100%, sum 1.01): finishing 0.19, positioning 0.13, ball_control 0.10, heading_accuracy 0.10, shot_power 0.10, reactions 0.08, dribbling 0.07, short_passing 0.05, strength 0.05, sprint_speed 0.05, acceleration 0.04, long_shots 0.03
- **CF** (131 players, exact 87%, within 1: 100%, sum 0.97): ball_control 0.18, positioning 0.13, dribbling 0.13, finishing 0.12, reactions 0.10, short_passing 0.09, vision 0.08, acceleration 0.07, shot_power 0.06, heading_accuracy 0.04, sprint_speed 0.04, long_shots 0.03, diving -0.02, jumping -0.03
- **LW** (416 players, exact 91%, within 1: 99%, sum 1.02): ball_control 0.15, dribbling 0.15, finishing 0.12, crossing 0.09, positioning 0.09, short_passing 0.08, reactions 0.07, acceleration 0.07, vision 0.07, sprint_speed 0.06, long_shots 0.04, agility 0.03
- **RW** (454 players, exact 93%, within 1: 99%, sum 1.01): dribbling 0.15, ball_control 0.15, positioning 0.11, finishing 0.10, short_passing 0.09, crossing 0.09, acceleration 0.07, reactions 0.07, vision 0.05, sprint_speed 0.05, long_shots 0.04, agility 0.03
- **CAM** (1049 players, exact 95%, within 1: 100%, sum 0.99): short_passing 0.16, ball_control 0.15, vision 0.14, dribbling 0.13, positioning 0.09, reactions 0.07, finishing 0.07, long_shots 0.05, long_passing 0.04, acceleration 0.04, sprint_speed 0.03, agility 0.03
- **LM** (957 players, exact 95%, within 1: 100%, sum 0.98): dribbling 0.15, ball_control 0.14, short_passing 0.11, crossing 0.10, positioning 0.09, reactions 0.07, vision 0.07, acceleration 0.07, finishing 0.06, sprint_speed 0.06, long_passing 0.05, stamina 0.05
- **RM** (944 players, exact 94%, within 1: 100%, sum 1.00): ball_control 0.15, dribbling 0.14, short_passing 0.11, crossing 0.10, positioning 0.09, reactions 0.07, acceleration 0.07, vision 0.06, finishing 0.06, sprint_speed 0.06, stamina 0.05, long_passing 0.05
- **CM** (2204 players, exact 92%, within 1: 100%, sum 1.00): short_passing 0.17, ball_control 0.15, long_passing 0.13, vision 0.12, reactions 0.09, dribbling 0.07, positioning 0.06, stamina 0.06, standing_tackle 0.05, interceptions 0.05, long_shots 0.04, finishing 0.02
- **CDM** (1519 players, exact 94%, within 1: 100%, sum 1.00): short_passing 0.14, interceptions 0.13, standing_tackle 0.13, ball_control 0.11, long_passing 0.10, marking_awareness 0.09, reactions 0.08, stamina 0.06, aggression 0.05, sliding_tackle 0.05, strength 0.04, vision 0.04
- **CB** (3260 players, exact 96%, within 1: 100%, sum 1.00): standing_tackle 0.18, marking_awareness 0.14, interceptions 0.13, sliding_tackle 0.10, heading_accuracy 0.10, strength 0.10, aggression 0.07, reactions 0.05, short_passing 0.05, ball_control 0.04, jumping 0.03
- **LB** (1162 players, exact 95%, within 1: 100%, sum 1.00): sliding_tackle 0.14, interceptions 0.12, standing_tackle 0.12, crossing 0.09, reactions 0.08, marking_awareness 0.08, stamina 0.08, ball_control 0.07, short_passing 0.07, sprint_speed 0.06, acceleration 0.06, heading_accuracy 0.04
- **RB** (1186 players, exact 95%, within 1: 100%, sum 1.00): sliding_tackle 0.13, standing_tackle 0.12, interceptions 0.11, crossing 0.09, reactions 0.09, marking_awareness 0.08, stamina 0.08, ball_control 0.07, short_passing 0.07, sprint_speed 0.07, acceleration 0.05, heading_accuracy 0.04
- **LWB** (265 players, exact 95%, within 1: 100%, sum 0.98): crossing 0.14, interceptions 0.13, stamina 0.10, sliding_tackle 0.10, standing_tackle 0.09, short_passing 0.09, ball_control 0.09, reactions 0.08, marking_awareness 0.07, sprint_speed 0.06, acceleration 0.04, dribbling 0.04
- **RWB** (301 players, exact 96%, within 1: 100%, sum 1.02): crossing 0.12, interceptions 0.12, short_passing 0.10, sliding_tackle 0.10, stamina 0.10, reactions 0.09, standing_tackle 0.09, ball_control 0.08, marking_awareness 0.07, sprint_speed 0.06, acceleration 0.04, dribbling 0.04
- **GK** (2045 players, exact 96%, within 1: 100%, sum 0.98): diving 0.21, handling 0.21, reflexes 0.21, positioning 0.21, reactions 0.11, kicking 0.10, shot_power -0.07

## Caveats

- **Noisy small fits.** A few weights are negative or tiny and are fitting noise, not football: goalkeeper shot_power (-0.07), centre-forward jumping and diving (-0.03, -0.02). Before these go into the engine, drop them and refit with non-negative weights, then re-check that the result stays within one point for at least 98% of players.
- **Main position only.** The fit uses the first position listed. A player rated for several positions has one overall in the data, and it follows his main position.
- **Positions with few players** (CF, LWB, RWB) have the loosest fits; CF is the weakest at 87% exact.
- **Only the 34 attributes we store.** The data has no other inputs, and the fit matches to rounding, so nothing important is missing.

## How the engine will use it

As a **change**, not a total: when an attribute moves (XP, training), the new overall is `overall + sum of weight x change in attribute`. That works for any edition (including FIFA 15 with missing attributes), keeps the real rounding of the stored overall, and makes a striker gain more from +2 finishing than a centre-back does. See the Player progression decision in [PLAN.md](../../PLAN.md).

## In the engine (done 2026-11-07)

`engine/src/overall.rs` holds the weights (option C: as fitted, with every weight under 0.02 and every negative one dropped and the rest refitted, rounded to three decimals) and `overall_change(position, changes)`. The test `the_formula_predicts_the_real_change_in_overall_between_editions` predicts each player's FC 24 overall from his FIFA 23 overall and the change in his attributes, and needs at least 98% within one point.

Measured when the weights were generated (players with all 34 attributes and the same main position in both editions):

| Editions | Players | Predicted overall within 1 of the real one | Mean error |
|---|---|---|---|
| FIFA 23 to FC 24 | 10,670 | 99.7% | 0.32 |
| FIFA 18 to FIFA 19 | 10,823 | 99.9% | 0.32 |
| FIFA 20 to FIFA 21 | 11,153 | 92.2% | 0.75 |

FIFA 20 to FIFA 21 is the weakest. The weights were fitted on FC 24, so a different formula or a position change in that edition pair is the likely cause; check it before relying on the formula for early-edition careers.
