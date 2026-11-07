# How fixtures are made and how postponed matches are rescheduled

Researched 2026-11-07. Sources are the Premier League, UEFA regulations and news pages found by web search. Items marked **(unverified)** are not from a source. The biggest gap: I found **nothing on how a postponed league match gets its new date** (who picks it, the rules around international breaks or cup ties). What exists is mostly the rules for *whether* a match can be postponed, plus UEFA's own rescheduling articles.

## How a season's fixtures are first made (Premier League)

- Built since 1992 by Atos: software generates many candidate schedules, then people check them by hand for days. It has to satisfy the league's "Golden Rules" (limits on consecutive home or away games). [Compare the Cloud](https://www.comparethecloud.net/news/atos-reveals-2026-27-premier-league-fixtures-using-34-seasons-of-scheduling-technology/), [Planet Football](https://www.planetfootball.com/quick-reads/how-the-premier-league-and-football-league-fixtures-are-compiled)
- **Constraints found:**
  - Local rivals are paired so one is home when the other is away (travel and police).
  - Police can ask for certain dates to be avoided (example: 2012/13, the Olympics).
  - Boxing Day and New Year's Day are paired as a home and an away.
  - Fewer long-distance trips over holiday dates.
  - Newer builds add enough recovery time between matches.
  - Shared stadiums, policing and broadcaster requirements can force changes.
- **Changing one match can force around 40 other changes** (Planet Football), which is why people review it by hand.
- Not found: exactly how European fixtures are fed in. Common sense and the UEFA material below suggest European matchdays are fixed first and domestic rounds fit around them **(unverified)**.
- The ESPN piece on the "fixture computer" is satirical and one result was a spam page; I left both out.

## Postponements: who decides and when

- **Premier League:** the Board decides, case by case, under Rules L.1.2 and L.15. [Premier League guidance](https://www.premierleague.com/en/news/2646833)
- **Player shortages:** since 2022/23 postponements for unavailable players are only granted in "truly exceptional" circumstances. [Premier League: insufficient players guidance](https://premierleague.com/news/2641863), [90min](https://www.90min.com/posts/premier-league-revises-postponement-rules-2022-23)
  - The threshold sources disagree. One version says a postponement is refused if the club has 14 or more players available (counting qualifying under-21s). Others give different numbers, and the Covid-era versions differ again. **Don't rely on a number.**
  - The goalkeeper is excluded from the "short in a position" argument.
- **Rescheduling aim:** the league wants postponed matches played "as quickly as the fixture list allows". [ESPN](https://global.espn.com/football/story/_/id/37623545/answering-big-questions-protocols-plans-postponements)
- One page quoted a "22-day reschedule rule" with a 4-point deduction for missing it, but it came from a Hong Kong site and was not about the English Premier League, so I did not use it.

## UEFA club competitions

- **The UEFA match calendar is binding.** Dates are final, subject only to the exceptions in the regulations. [UEFA Champions League regulations, Art. 27 Rescheduling](https://documents.uefa.com/r/Regulations-of-the-UEFA-Champions-League-2025/26/Article-27-Rescheduling-of-matches-Online)
- A rescheduled match is normally played at the **same stadium**; a change needs UEFA's approval, and UEFA's administration sets the new kick-off time, taking the teams' needs into account where possible. (Youth League wording; the Champions League article is similar but I only saw snippets.) [Youth League Art. 25](https://documents.uefa.com/r/Regulations-of-the-UEFA-Youth-League-2025/26/Article-25-Rescheduling-of-matches-Online)
- **Match stopped on the day:** in the Nations League rules, the full or remaining match is normally played the next day, which may break the usual pattern. I did not find the equivalent clause for the Champions League. [Nations League Art. 24](https://documents.uefa.com/r/Regulations-of-the-UEFA-Nations-League-2026/27/Article-24-Rescheduling-of-matches-Online)
- **Stadium unfit:** UEFA can ask the clubs to propose another venue and can choose a neutral one if they can't. The home club bears the costs. [inkl](https://inkl.com/news/arsenal-to-forfeit-psv-eindhoven-clash-europa-league-rules-explained-amid-fixture-postponement)
- **Missed deadline:** Covid-era knockout rules forfeited a match 3-0 against the club responsible if it was not played by a set date. Historical, not current. [AP via WTOP](https://wtop.com/lifestyle/2021/02/uefa-sets-rules-deadlines-to-play-champions-league-games/)

## What this means for our engine

What we already have helps: `World::first_free_week(clubs, from_week)` finds the first week where every club involved has room under the 3-a-week limit, and `calendar_problems()` checks the whole calendar. A reschedule can be: take the fixture, call `first_free_week` from the original week, move it.

Questions the game has to answer, and how I'd start:

| Question | Simple first answer |
|---|---|
| Why does a match move? | Start with none; later reasons: weather, cup or European clash, international duty, a stadium problem. Player shortage is almost never granted in real life, so skip it. |
| Who picks the new date? | The game does, with `first_free_week`, as soon as possible after the original. |
| Which competition gives way? | European matchdays are fixed and binding, so domestic matches move around them. Among domestic ones, I'd move the league match, since the cup has the tighter calendar **(unverified)**. |
| Is there a deadline? | Yes: the season's last week. A match that can't fit needs a rule (play it anyway or forfeit). Not researched. |
| Midweek vs weekend? | Our calendar only has weeks, not days, so it can't tell. A day-of-week field is a later change. |

Also needed: **gaps in the league calendar** (international breaks, winter breaks). Today league round N is always in week N, so those gaps don't exist yet.

## Open questions

- How a postponed Premier League match gets its new date, and the rules around international breaks and cup ties. Next searches: Premier League handbook sections on fixtures; FA Cup rules on clashing fixtures.
- Which league rounds get moved for television (broadcaster picks) and how far.
- How the English Football League and other countries handle postponed matches. Only the Premier League and UEFA were researched.
- Whether to model days of the week, or stay with weeks.
- The Champions League same-day stoppage rule.

## Sources

- [Compare the Cloud: Atos 2026-27 Premier League fixtures](https://www.comparethecloud.net/news/atos-reveals-2026-27-premier-league-fixtures-using-34-seasons-of-scheduling-technology/)
- [Planet Football: how fixtures are compiled](https://www.planetfootball.com/quick-reads/how-the-premier-league-and-football-league-fixtures-are-compiled)
- [Premier League: match postponement guidance](https://www.premierleague.com/en/news/2646833)
- [Premier League: insufficient players guidance](https://premierleague.com/news/2641863)
- [90min: Premier League revises postponement rules 2022/23](https://www.90min.com/posts/premier-league-revises-postponement-rules-2022-23)
- [ESPN: protocols, plans, postponements](https://global.espn.com/football/story/_/id/37623545/answering-big-questions-protocols-plans-postponements)
- [UEFA Champions League regulations 2025/26, Article 27](https://documents.uefa.com/r/Regulations-of-the-UEFA-Champions-League-2025/26/Article-27-Rescheduling-of-matches-Online)
- [UEFA Youth League regulations, Article 25](https://documents.uefa.com/r/Regulations-of-the-UEFA-Youth-League-2025/26/Article-25-Rescheduling-of-matches-Online)
- [UEFA Nations League regulations, Article 24](https://documents.uefa.com/r/Regulations-of-the-UEFA-Nations-League-2026/27/Article-24-Rescheduling-of-matches-Online)
- [inkl: Europa League venue rules](https://inkl.com/news/arsenal-to-forfeit-psv-eindhoven-clash-europa-league-rules-explained-amid-fixture-postponement)
- [AP via WTOP: UEFA sets rules, deadlines](https://wtop.com/lifestyle/2021/02/uefa-sets-rules-deadlines-to-play-champions-league-games/)
