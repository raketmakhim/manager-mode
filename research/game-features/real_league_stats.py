"""Real results of the five big leagues, 2014/15 to 2023/24, to compare the match simulation with.

Run from the project root:  py -I research/game-features/real_league_stats.py
Reads research/information/data/real-results/<league>_<season>.csv, downloaded from football-data.co.uk
(E0 England, D1 Germany, SP1 Spain, I1 Italy, F1 France; seasons 1415 ... 2324; not committed).
Prints, per league: goals per match, home/draw/away shares, and what the table looked like on average:
points per match of the champion, the 4th and last club, and the gap between champion and runner-up.
"""
import csv
import glob
import os
from collections import defaultdict

LEAGUES = [("E0", "Premier League"), ("D1", "Bundesliga"), ("SP1", "La Liga"), ("I1", "Serie A"), ("F1", "Ligue 1")]
SEASONS = ["1415", "1516", "1617", "1718", "1819", "1920", "2021", "2122", "2223", "2324"]
FOLDER = os.path.join("research", "information", "data", "real-results")


def read_season(path):
    """Final table and match totals of one season. A table row is [points, played, goal difference, goals for]."""
    table = defaultdict(lambda: [0, 0, 0, 0])
    matches = goals = home = draws = away = 0
    with open(path, encoding="utf-8-sig", newline="") as f:
        for row in csv.DictReader(f):
            if not row.get("FTR"):
                continue  # an empty or unfinished row
            h, a = int(row["FTHG"]), int(row["FTAG"])
            matches, goals = matches + 1, goals + h + a
            home, draws, away = home + (h > a), draws + (h == a), away + (h < a)
            for club, scored, conceded in ((row["HomeTeam"], h, a), (row["AwayTeam"], a, h)):
                t = table[club]
                t[0] += 3 if scored > conceded else 1 if scored == conceded else 0
                t[1] += 1
                t[2] += scored - conceded
                t[3] += scored
    ranked = sorted(table.items(), key=lambda kv: (-kv[1][0] / kv[1][1], -kv[1][2], -kv[1][3]))  # points per match
    return ranked, matches, goals, home, draws, away


print("| League | Seasons | Goals/match | Home % | Draw % | Away % | Champion pts/match | Runner-up | 4th | Last | Champion lead over 2nd (pts/match) |")
print("|---|---|---|---|---|---|---|---|---|---|---|")
for code, name in LEAGUES:
    totals = defaultdict(float)
    n = 0
    for season in SEASONS:
        path = os.path.join(FOLDER, f"{code}_{season}.csv")
        if not os.path.exists(path):
            continue
        ranked, matches, goals, home, draws, away = read_season(path)
        ppm = [t[0] / t[1] for _, t in ranked]
        for key, value in (("matches", matches), ("goals", goals), ("home", home), ("draws", draws), ("away", away),
                           ("first", ppm[0]), ("second", ppm[1]), ("fourth", ppm[3]), ("last", ppm[-1]), ("lead", ppm[0] - ppm[1])):
            totals[key] += value
        n += 1
    if n == 0:
        continue
    m = totals["matches"]
    print(f"| {name} | {n} | {totals['goals']/m:.2f} | {100*totals['home']/m:.1f} | {100*totals['draws']/m:.1f} | {100*totals['away']/m:.1f} "
          f"| {totals['first']/n:.2f} | {totals['second']/n:.2f} | {totals['fourth']/n:.2f} | {totals['last']/n:.2f} | {totals['lead']/n:.2f} |")

print("\nPremier League champions and points, season by season:")
for season in SEASONS:
    path = os.path.join(FOLDER, f"E0_{season}.csv")
    if os.path.exists(path):
        ranked, matches, *_ = read_season(path)
        (champ, t), (second, t2), (last, tl) = ranked[0], ranked[1], ranked[-1]
        print(f"  20{season[:2]}/{season[2:]}: {champ} {t[0]} pts (2nd {second} {t2[0]}, last {last} {tl[0]})")
