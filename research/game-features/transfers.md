# Transfers

Status: the summer window is built (`engine/src/transfer.rs`, `source::plan_transfers`, the GUI screen `transfers.rs`). Buying and selling by the user, fees and budgets are not.

## 1. What the data says (history.db, FIFA 15 to FC 24)

Compared by player id between each edition and the next (numbers for 15 to 16 and 20 to 21):

| | 15 to 16 | 20 to 21 |
|---|---|---|
| players in both editions | 11,417 | 13,227 |
| same club | 7,503 | 9,524 |
| club to club | 3,867 | 3,613 |
| club to no club | 29 | 45 |
| no club to club | 18 | 45 |
| in the old edition only (retired or dropped) | 4,765 | 5,256 |
| in the new edition only (youngsters, new to the game) | 5,289 | 5,665 |

- Every club to club move names clubs that exist in `team_editions`; the ids are clean. A club that is not in the loaded edition (a team promoted from outside our data) is treated as "nowhere".
- About 15% of the moves are loans (`club_loaned_from` is set in 529 of the 3,613 moves 20 to 21). A loan shows him back at his own club a year later, as another move.
- `club_joined_date` has the real date: 2020-21 moves joined mostly in July to September (about 2,200 of 3,600), with a January peak (767). We use the summer only, one window a year.
- Half the moves stay inside one league (1,696 of 3,613 in 20 to 21).
- There is no real fee in the data. Value (`value_eur`, average 2.6m for movers) is the best price we have; `release_clause_eur` exists but is higher (4.5m).
- If only the moves were applied, squads would fall from 24 to about 17 (retired and dropped players leave, nothing joins). The new-only players (average age 23, many 16 to 19) refill them to about 24.5, the real size. So the window must also bring players in from nowhere and take some out.

## 2. The design as built

- `Transfer { player, from: Option<ClubId>, to: Option<ClubId>, kind, fee }`: one record for every move. No club before means a new player, no club after means he leaves the game.
- `transfers_to_reality(world, before, after, user)` is a pure function over the world and two `Membership` maps (club, on loan, value). It moves every player to the club the next edition really has him at, so a world that drifted is pulled back. `source::plan_transfers` reads the maps from the database and loads the players new to the world with their real next ratings.
- A club with nobody in the next edition is one the data dropped (Catania, Arles-Avignon...): its players stay, or the club would play its league with an empty team and every opponent would score five (found when a Lokomotiv striker had 81 goals in 2019/20). Tests: `a_club_the_data_dropped_keeps_its_players...` and a three-summer career test.
- A deal that involves your club only happens if the player really moved that summer, so a player you kept or refused is not sold or signed unasked later.
- `Window::new` keeps only what can happen in this world: the player is at the club the transfer says, a destination outside the world counts as nowhere, and a signing from nowhere is a free agent or a new player with data.
- The user is asked only about moves between two clubs that involve their club. Their players who would leave the game (or move to a club we do not have) stay: nobody disappears from a squad unasked. New players who join their club arrive without a question.
- `World::make_transfers(window)` applies every deal not declined, then sorts the touched squads best first. A declined deal leaves the player where he is. It runs before `advance_ratings`, which gives everyone the real next ratings.
- Fee = his value for a permanent club to club move, 0 for a loan or a move with no club on one side. It is shown on the screen; no budget is changed yet.

## 3. Not built, and what to do when

- **Budgets and fees paid**: the data has `transfer_budget_eur` only up to FC 23 (`Club.budget` is `None` in FC 24). Decide with the finance work (M4).
- **The user's own bids, a transfer list, AI accepting or rejecting an offer**: the next step. The `Transfer` record and `make_transfers` are what a bid turns into.
- **After FC 24 there are no real moves to replay**: a second model is needed (rule-based, by value and need). Make it a `TransferModel` enum with the real-moves one when it exists; today there is one source, so there is no enum yet.
- **Contracts**: `contract_until` is overwritten by the real one each year. Expiry and renewals belong with the bids.
- **A rejected deal stays rejected**: only deals that are real moves of that summer are offered to you, so a player you kept stays until the data moves him again.
- **Dropped clubs freeze**: a club the data drops keeps its old squad (no transfers, ratings as they were) until the data has it again, then it is pulled back to the real squad.
- Loans are treated as plain moves; the player does not go back by itself except through the real data.
