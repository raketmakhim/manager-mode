# Where to get FIFA player data for every edition

Researched 2026-11-06. Sources are Kaggle pages and sofifa pages seen in search results; I did not download or open any dataset, so row counts, columns and completeness are **unverified**.

## Short answer

There is no single official database. The practical source is **sofifa.com**, a fan site that archives player ratings for every FIFA/FC edition. Ready-made Kaggle datasets exist for parts of it; the rest must be scraped.

## sofifa.com (main source)

- Keeps date-stamped rating snapshots per edition. Pages seen: FIFA 07 (30 Aug 2006), FIFA 13 (31 Aug 2012), FIFA 15 (29 Aug 2014), FIFA 16.
- Its edition selector lists FIFA 07 through the latest FC release, so **the earliest edition is FIFA 07, i.e. the earliest start year is about 2006.** Anything before that is not on sofifa **(not found in any source)**.
- Edition pages use a URL like `sofifa.com/?r=070001&set=true`, where the code appears to encode the edition **(unverified pattern)**.
- Not verified: FIFA 08-12 pages specifically, how many attributes older editions have, how many players per edition, whether sofifa player IDs stay the same across editions (I believe they do, **unverified**), and sofifa's robots.txt, terms and rate limits. Check these before crawling.

## Ready-made Kaggle datasets

| Dataset | Covers | Notes |
|---|---|---|
| [FIFA 20 complete player dataset](https://www.kaggle.com/datasets/stefanoleone992/fifa-20-complete-player-dataset) (Stefano Leone) | FIFA 15 to FIFA 20, Career Mode players | Best starting point. Author page also points to newer versions (FIFA 22). |
| [Fifa/EA FC 25 player data from Sofifa](https://www.kaggle.com/datasets/aniss7/fifa-player-data-from-sofifa-2025-06-03) | FC 25, ~18k players | Names the scraper script used. |
| [FIFA PLAYER DATA SET & SEASON STATS](https://www.kaggle.com/datasets/jacksonjohannessen/fifa-and-irl-soccer-player-data) | FIFA 2017 to FC 25 | Only ~11k unique players, so partial. |
| [FIFA Index Player Ratings (EPL V16-V24)](https://www.kaggle.com/datasets/yarknyorulmaz/fifa-index-player-ratings-dataset-epl-v16-v24) | Premier League only, FIFA 16 to 24 | Too narrow for us. |
| [FIFA Players Dataset (luisfucros)](https://www.kaggle.com/datasets/luisfucros/fifa-players), [moradi](https://www.kaggle.com/datasets/moradi/fifa-stats) | Single or few editions | Not checked. |

No dataset found that covers FIFA 07 to the latest edition in one piece.

## Suggested plan

1. **2015-2020 (and maybe up to 22):** download the Leone datasets from Kaggle. Cheapest.
2. **Later editions (FC 24, FC 25, newer):** use the Kaggle FC 25 data or scrape sofifa.
3. **2007-2014:** scrape sofifa. The only source I found. Existing scrapers to start from:
   - [sofifa-web-scraper](https://github.com/prashantghimire/sofifa-web-scraper): a full run over 18k+ players took about 2.5 hours for one edition.
   - [R package SoFIFA](https://rdrr.io/github/valentinumbach/SoFIFA/src/R/scrapi.R)
   - [GitHub topic: sofifa](https://github.com/topics/sofifa)
4. Write the import tool (M1) to read each source's columns into **one internal schema**, since column names and attribute sets differ by edition.

## Risks and checks before we commit

- **Player ID stability.** Compare one known player (e.g. Wayne Rooney) across two editions. If IDs differ, we need name + birth date matching.
- **Attribute differences in old editions.** Check what FIFA 07-10 actually provide. May need derived values.
- **Scraping etiquette.** Read sofifa's robots.txt and terms, rate-limit the crawler, and cache raw pages so we only fetch each once. One R package uses a stealth-style fetch helper, which hints the site may block plain scraping **(unverified)**.
- **Data gaps.** Datasets give players and ratings. Club finances, manager data, fixtures and transfer history are not in them; see other `information/` notes.
- **Edition vs season year.** FIFA 07 shipped in autumn 2006 and reflects the 2006/07 season. Decide how "start in 2008" maps to editions.

## Open questions

- Is there a better source for FIFA 99-06? Not found.
- Do the Kaggle files keep the same player ID as sofifa?
- Where to get managers, club budgets and historical transfers (Transfermarkt is the usual source, not yet researched).

## Sources

- [sofifa FIFA 07 players](https://sofifa.com/?r=070001&set=true)
- [sofifa FIFA 13 players](https://sofifa.com/?r=130001&set=true)
- [sofifa FIFA 15 players](https://sofifa.com/?r=150001&set=true)
- [sofifa FIFA 16 players](https://sofifa.com/?r=160044&set=true)
- Kaggle datasets linked in the table above
- Scrapers linked in the plan above

## Data found in `stats/` (checked 2026-11-06)

16 files, `player_stats_2005.csv` to `player_stats_2020.csv` (FIFA 05 to FIFA 20), ~4 MB each. **Source is fifaindex.com** (see `page_url` column), not sofifa.

- ~10.5k rows per year; exactly 10,500 from 2007 to 2020, so likely capped to the top 10,500 players (unverified). 2005: 10,993; 2006: 10,176.
- **Player IDs are stable across years**: Thierry Henry is `1625` in 2005, 2008 and 2012, with the same birthdate. No duplicate IDs within a year. About 70-80% of one year's players appear in the next.
- Columns: identity (name, birthdate, nationality, height, weight), club, position(s), `rating` (overall; **no separate potential column**), ~30 skill attributes, 5 goalkeeper attributes, kit numbers, work rates, weak foot, skill moves.
- Missing values are the string `NA`. Share of NA in the 30 outfield attributes: ~37% for 2005 and 2007-2010, 50% for 2006, 23% for 2011, 10% for 2012-2016, 7% for 2017-2020. Old editions use a different, smaller attribute set, so the mapping layer is needed.
- `value` and `wage` are 100% NA in 2008 (and presumably the older years); filled (98%) by 2020. `contract_length` is nearly always present.
- Does not cover 2021 onward.

### Follow-up: which attributes exist per edition (checked 2026-11-06)

- Columns that are ~100% empty: 13 of 36 in 2005 and 2007-2010, 18 in 2006, 9 in 2011, 4 in 2012-2016, 3 in 2017-2020 (`creativity`, `shot_accuracy`, `gk_rushing`). So old editions really have fewer stats.
- **Column meaning drifts between editions (unverified).** fifaindex's page for Thierry Henry (FIFA 05) shows Attack Position 98, Vision 88, Finishing 98. The 2005 CSV row has those numbers under `reactions` (98), `creativity` (88) and `shot_accuracy` (98), while `att_position`, `vision`, `finishing` are NA. The scraper's columns seem to follow each edition's own old attribute names, not one fixed set. Before building the mapping layer, compare a handful of players per edition against the site.
- fifaindex shows **Potential** (Henry 99) and **Growth** (+1) on the player page, but the CSVs have no such column. Could be scraped from `page_url`.
- The site's career history for Henry reads "FIFA 05 - FC 27", so it also covers 2021 onward, which the CSVs lack.

### Decision (2026-11-06): `stats/` CSVs are incomplete, do not use as the main dataset

Each year is capped at ~10.5k players (exactly 10,500 for 2007-2020), well short of a full edition. A career mode needs every player in the covered leagues, including lower-rated squad players and youth. Keep the files only as a reference for column names and ID stability. Source of a complete dataset is still open: Kaggle (Leone, FIFA 15-20), sofifa scrape (FIFA 07-14), fifaindex scrape. Row counts of each must be checked before use.

### Column drift result (2026-11-06, statistical, same-player values across consecutive years)

Column meaning is stable inside these blocks and shifts at the boundaries: **2005 | 2006 | 2007-2010 | 2011 | 2012-2020** (2018 to 2019 has one small blip, `marking`). Many columns shift by one position at the 2005-06, 2006-07, 2010-11 and 2011-12 boundaries. This is a best-match guess, not proof, and it came from files we have since rejected. Lesson for any new scrape: map each attribute by the name on the page, per edition, never by column position.

## Kaggle data in `data/` (checked 2026-11-06): complete, use this as the base for 2014-2023

Files: `male_players.csv` (96 MB, 180,021 rows), `male_teams.csv` (6,947 rows), `male_coaches.csv` (1,369 rows). This is the sofifa-based Kaggle set (player URLs are sofifa). The earlier `stats/` folder (fifaindex, capped at 10.5k/year) was replaced by it.

- **Editions:** `fifa_version` 15 to 24 (FIFA 15 to FC 24), exactly one snapshot each (`update_as_of` 2014-09-18 for v15, 2023-09-22 for v24; `fifa_update` is always 2).
- **Completeness: yes.** 16,182 players in v15 rising to 19,239 in v22; 18,350 in v24. 579-702 clubs and 38-56 leagues per edition. Real edition-sized squads, so lower-rated players and youth are included.
- **Player ID stable:** Rooney is `54050` in every edition; about 70% of one edition's players appear in the next.
- **Fields in the player rows:** `overall` and `potential` (both ~0% blank), `value_eur`, `wage_eur`, contract end year, club, league, positions, ~30 outfield + 5 GK attributes under consistent names, `dob`, height/weight, traits, tags, release clause, international reputation. 99-100% filled for value and wage in all editions.
- **Attribute names are consistent across all 10 editions.** Only gap: `mentality_composure` is blank in v15. The `pace`/GK NA (11%) is goalkeepers, expected.
- **Teams file:** per edition team ratings, `coach_id`, home stadium, rivals, prestige, and **club finances**: `transfer_budget_eur` (~7% blank, but 100% blank in v24), `club_worth_eur` (100% blank in v15), tactics (`def_style` etc.) are blank before v21.
- **Coaches file:** id, name, dob, nationality. No coach attributes or ratings.
- **Coverage gap:** nothing before FIFA 15 (2014) and nothing after FC 24. FC 25+ would come from the other Kaggle set or a scrape. FIFA 07-14 would still need a sofifa scrape.
