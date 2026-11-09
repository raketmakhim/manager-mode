# Retirement

Status: built (`engine/src/retirement.rs`). Players decide at the start of a season and retire when it ends.

## 1. What the data says

The data cannot tell us who really retired. Share of players with a club who are missing from the next edition (FIFA 16 to 17, 20 to 21, 23 to 24):

| Age | Missing a year later |
|---|---|
| under 25 | 29% |
| 25 to 29 | 24 to 25% |
| 30 to 32 | 31 to 32% |
| 33 to 34 | 37 to 40% |
| 35 to 36 | 39 to 47% |
| 37 and over | 47 to 56% |

A quarter of 25 to 29 year olds vanish too: the game drops players (small clubs, lost licences). So "missing from the next edition" is not retirement, and the transfer window keeps treating it as leaving the game, not as a career end. The retirement curve is our own.

## 2. The design as built

- **Decided at the start of the season**, so the user sees who is planning to retire and can plan for it: `Player::career` is `Playing`, `Retiring` or `Retired` (`CareerStatus`). When the user starts the next season, in this order: `retire_planned` (the players who planned leave their squad, `club` becomes `None`, they stay in `World::players` so stats and history survive), the transfers, the new ratings, then `plan_retirements` for the new season. Deciding last means players who just arrived decide too and the chance uses the new overall; retiring only after the transfer plan is made and the next season is known to exist means nothing changes if reading the data fails. The first season of a career decides when it starts.
- **The model** is one pure function, `chance(model, age, overall)`, chosen by `RetirementModel { None, ByAge }` (contract tests run through `ALL`). `ByAge`: nothing before 32, then `((age - 31) / 10)^2` (1% at 32, 16% at 35, 49% at 38, 100% from 42), times `(100 - overall) / 35` between 0.4 and 1.6 (a 90 retires at 0.4 of the average chance, a 60 at 1.1). Real careers end around 35 on average; goalkeepers play longer (not modelled).
- **The user is told**: the player page says "Planning to retire at the end of this season", the squad list adds "(retiring)", and the line after "Start next season" names who retired and who plans to (yours in brackets). There is no yes or no: a retirement is not the user's decision.
- **The data still lists retired players** for a few editions, and a player about to retire is not sold: `transfers_to_reality` leaves out everyone whose `career` is not `Playing`.

## 3. Not built

- Staying on a year (persuading a player, a contract offer): the `Retiring` status is where it would go.
- A different curve for goalkeepers, and for a player with no club (free agents retire like everyone).
- Refilling squads: nobody replaces a retired player yet except the data's new players (M5, new youngsters).
- The curve is a guess: check it against the age profile of a long simulated career when seasons beyond FC 24 exist.
