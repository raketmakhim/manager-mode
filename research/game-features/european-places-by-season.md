# European places per country, per season (2014/15 to 2023/24)

Researched 2026-11-07 from the Wikipedia page of each season's [Champions League](https://en.wikipedia.org/wiki/2014%E2%80%9315_UEFA_Champions_League), Europa League and (from 2021/22) Conference League: 26 pages in all, each read by a separate reader.

**How reliable this is.** Every page was cut off by the fetch tool at about 39k characters. What I could read as written text: the "association team allocation" rule lines, the country ranking tables (ranks 1 to 15 for every season) and, for 2014/15 to 2016/17, the round-by-round distribution. The per-country club lists and the round-by-round distribution for later seasons came from a machine summary, so they are marked **[summary]** and I did not build on them. The country counts in section 3 are **worked out by me** from the written band rules and the written ranks, and they agree with the club lists the readers saw wherever both existed. These are Wikipedia pages, not UEFA's own circulars.

## 1. The rule: places follow the country's rank

A country's number of places depends only on its **position in UEFA's country ranking** that season (the ranking built from the previous five seasons of results, labelled with the year before the season starts). So the engine can store one ranking order per season plus a few band rules, instead of a table per country.

**Champions League** (every season; "ranks" are the country's ranking position):

| Seasons | 4 teams | 3 teams | 2 teams | 1 team |
|---|---|---|---|---|
| 2014/15 to 2017/18 | ranks 1 to 3 | ranks 4 to 6 | ranks 7 to 15 | ranks 16 and below (not Liechtenstein) |
| 2018/19 to 2023/24 | ranks 1 to 4 | ranks 5 to 6 | ranks 7 to 15 | ranks 16 and below (not Liechtenstein) |

- **Russia:** banned from 2022/23 and 2023/24, so it has no places and is skipped in the count (rank 8 in 2022/23, rank 10 in 2023/24).
- **Maximum five teams** from one country (page rule from 2015/16).
- **Title-holder extra place:** the winners of the previous Champions League and Europa League get a place if they did not qualify through their league. It is added on top of the country's places. Seen in the pages: Sevilla 2015/16 and 2016/17 (Spain 5), Manchester United 2017/18 (England 5, as Europa League winners), Villarreal 2021/22 (Spain 5), Eintracht Frankfurt 2022/23 (Germany 5), Sevilla 2023/24 (Spain 5). In 2014/15, 2018/19, 2019/20 and 2020/21 it wasn't needed.

**Europa League, base places before Champions League losers drop in:**

| Seasons | Rule |
|---|---|
| 2014/15 | ranks 1 to 6: 3; ranks 7 to 9: 4; ranks 10 to 51: 3; ranks 52 to 53: 2 (plus 3 Fair Play places) |
| 2015/16 to 2019/20 | ranks 1 to 51: 3 each; ranks 52 to 54: 2 (2015/16 also had 3 Fair Play places; none from 2016/17) |
| 2020/21 | ranks 1 to 50: 3; ranks 51 to 55: 2 |
| 2021/22 | ranks 1 to 5: 2; ranks 6 to 15: 1 |
| 2022/23 | ranks 1 to 5: 2; ranks 6, 7 and 9 to 15: 1 (Russia excluded) |
| 2023/24 | ranks 1 to 5: 2; ranks 6 to 16: 1 (Russia excluded) |

On top of those: 33 clubs knocked out of the Champions League drop into the Europa League (55 from 2018/19), and the Conference League winner gets a Europa League place from 2022/23.

**Conference League (new in 2021/22), base places:**

| Season | Rule |
|---|---|
| 2021/22 | ranks 1 to 5: 1 (Spain 0, because Villarreal were Europa League winners); ranks 6 to 15 and 51 to 55: 2; ranks 16 to 50: 3 |
| 2022/23 | ranks 1 to 5: 1; ranks 6 to 17 (not Russia) and 51 to 55: 2; ranks 18 to 50: 3 |
| 2023/24 | ranks 1 to 5: 1; ranks 6 to 16 (not Russia) and 51 to 55: 2; ranks 17 to 50: 3 |

## 2. The country ranking used each season (ranks 1 to 15)

| Season | Coefficients | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 2014/15 | 2013 | Spain | England | Germany | Italy | Portugal | France | Ukraine | Russia | Netherlands | Turkey | Belgium | Greece | Switzerland | Cyprus | Denmark |
| 2015/16 | 2014 | Spain | England | Germany | Italy | Portugal | France | Russia | Netherlands | Ukraine | Belgium | Turkey | Greece | Switzerland | Austria | Czech Rep. |
| 2016/17 | 2015 | Spain | England | Germany | Italy | Portugal | France | Russia | Ukraine | Netherlands | Belgium | Switzerland | Turkey | Greece | Czech Rep. | Romania |
| 2017/18 | 2016 | Spain | Germany | England | Italy | Portugal | France | Russia | Ukraine | Belgium | Netherlands | Turkey | Switzerland | Czech Rep. | Greece | Romania |
| 2018/19 | 2017 | Spain | Germany | England | Italy | France | Russia | Portugal | Ukraine | Belgium | Turkey | Czech Rep. | Switzerland | Netherlands | Greece | Austria |
| 2019/20 | 2018 | Spain | England | Italy | Germany | France | Russia | Portugal | Ukraine | Belgium | Turkey | Austria | Switzerland | Czech Rep. | Netherlands | Greece |
| 2020/21 | 2019 | Spain | England | Italy | Germany | France | Russia | Portugal | Belgium | Ukraine | Turkey | Netherlands | Austria | Czech Rep. | Greece | Croatia |
| 2021/22 | 2020 | Spain | England | Germany | Italy | France | Portugal | Russia | Belgium | Ukraine | Netherlands | Turkey | Austria | Denmark | Scotland | Czech Rep. |
| 2022/23 | 2021 | England | Spain | Italy | Germany | France | Portugal | Netherlands | Russia | Belgium | Austria | Scotland | Ukraine | Turkey | Denmark | Cyprus |
| 2023/24 | 2022 | England | Spain | Italy | Germany | France | Portugal | Netherlands | Austria | Scotland | Russia | Serbia | Ukraine | Belgium | Switzerland | Greece |

Ranks 16 and below were only partly captured (see open questions). A few were seen: Scotland 25th (2017/18), 23rd (2018/19), 26th (2019/20), 20th (2020/21), 14th (2021/22), 11th (2022/23), 9th (2023/24); Switzerland 17th (2020/21 and 2021/22), 19th (2022/23).

## 3. Champions League places by country, worked out from sections 1 and 2

Spot counts include the title-holder extra place where it was used (marked +1).

| Season | Spain | England | Germany | Italy | France | Portugal | Netherlands | Russia | Others worth noting |
|---|---|---|---|---|---|---|---|---|---|
| 2014/15 | 4 | 4 | 4 | 3 | 3 | 3 | 2 | 2 | Turkey, Belgium, Ukraine, Greece, Switzerland 2 each; Scotland, Austria, Czech Rep. 1 |
| 2015/16 | 5 (+1) | 4 | 4 | 3 | 3 | 3 | 2 | 2 | Austria, Czech Rep. 2; Scotland 1 |
| 2016/17 | 5 (+1) | 4 | 4 | 3 | 3 | 3 | 2 | 2 | Scotland, Austria 1; Czech Rep. 2 |
| 2017/18 | 4 | 5 (+1) | 4 | 3 | 3 | 3 | 2 | 2 | Scotland, Austria 1 |
| 2018/19 | 4 | 4 | 4 | 4 | 3 | 2 | 2 | 3 | Austria 2; Scotland 1 |
| 2019/20 | 4 | 4 | 4 | 4 | 3 | 2 | 2 | 3 | Austria 2; Scotland 1 |
| 2020/21 | 4 | 4 | 4 | 4 | 3 | 2 | 2 | 3 | Austria 2; Scotland, Switzerland 1 |
| 2021/22 | 5 (+1) | 4 | 4 | 4 | 3 | 3 | 2 | 2 | Scotland 2, Denmark 2; Switzerland, Greece 1 |
| 2022/23 | 4 | 4 | 5 (+1) | 4 | 3 | 3 | 2 | 0 (banned) | Scotland 2; Austria 2 |
| 2023/24 | 5 (+1) | 4 | 4 | 4 | 3 | 3 | 2 | 0 (banned) | Scotland 2; Austria 2; Switzerland 2; Turkey 1 |

Turkey was rank 10 to 13 in most seasons (2 places) and fell to rank 20 in 2023/24 (1 place).

## 4. How the places are filled inside a country

From the 2014/15 distribution text (readable in full), which is the pattern for the whole window:

- **Group stage directly:** champions of the top countries (ranks 1 to 13 in 2014/15), runners-up of ranks 1 to 6, third place of ranks 1 to 3.
- **Play-off round:** third place of ranks 4 and 5, fourth place of ranks 1 to 3.
- **Third qualifying round:** runners-up of ranks 7 to 15, third place of rank 6.
- **Lower-ranked champions** start earlier (first or second qualifying round).
- **From 2018/19**, all four places of the top four countries go straight to the group stage (the "fourth place" play-off went away).
- **Europa League places:** the cup winner of each top country, then league places below the Champions League places. The English League Cup and French League Cup winners are the lowest-ranked qualifiers in their countries. A vacated place passes to the highest-placed club that has not qualified.
- **Entry stages shift each year** with the rank bands (for instance the champion of rank 11 or 12 enters the group stage one year and a qualifying round the next). The per-season stage lists for 2017/18 onward are from the **[summary]** only.

## 5. What this means for our engine

- **A season's qualification can be computed.** Store the ranking order (section 2) and the band rules (section 1) per season; the number of places for every country follows. No hand-written table per country.
- **Add the exception list:** the title-holder extra place, bans (Russia 2022/23 and 2023/24) and re-allocations.
- **Which club gets each place** is a separate, simple table: "country, finishing position or cup winner, entry stage". Start with the top five countries and the cup winners from [domestic-cup-formats.md](domestic-cup-formats.md).
- **Countries below rank 15** need their own ranking position per season. The rank bands are simple for them (1 Champions League place, 3 Europa League places, until 2021).

## Unresolved items the readers flagged

- **Shakhtar 2015/16:** one page shows a Champions League group-stage entry, the summary says third qualifying round.
- **Netherlands 2016/17:** Ajax's Europa League route differs between two views of the page.
- **Turkey 2020/21:** the band rule gives 2 Champions League places but only Besiktas was identified.
- **Russia 2020/21 Europa League:** the rule gives 5 but only 4 clubs were identified.
- **Conference League 2021/22 and 2022/23:** the country totals were not extracted reliably, so I give only the base rules. Treat the 2021/22 to 2023/24 Conference League club lists as unchecked.
- **Page truncation:** later-season club lists and round counts are summary-only. A re-read of each page past the 39k mark would settle them.

## Open questions

- Full ranking positions for ranks 16 to 55 each season (needed for any country outside the top 15, such as Scotland, Switzerland and Greece in some years).
- The match-by-match stage each place enters at, per season, for 2017/18 to 2023/24.
- Fair Play places (2014/15 and 2015/16 only) and how a vacated Europa League place passes on, if we model them.
- Whether to read the UEFA circular letters for the access lists, since Wikipedia is a secondary source.

## Sources

- [2014/15 Champions League](https://en.wikipedia.org/wiki/2014%E2%80%9315_UEFA_Champions_League) and the Europa League, Champions League and Conference League pages for each season up to [2023/24](https://en.wikipedia.org/wiki/2023%E2%80%9324_UEFA_Champions_League) (same address pattern, 26 pages)
- [UEFA: format and access list for 2024/25 approved](https://www.uefa.com/news-media/news/0275-151c779310c3-b92bbf0d24f9-1000--format-access-list-for-2024-25-approved/) (context from the earlier note)
