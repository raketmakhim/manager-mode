"""How real FIFA ratings change from one edition to the next, by age, potential gap, position and attribute.
Usage: python research/game-features/age_curves.py [male_players.csv]. Results are in experience.md."""
import csv, collections, statistics, sys

path = sys.argv[1] if len(sys.argv) > 1 else "research/information/data/male_players.csv"
rows = list(csv.DictReader(open(path, encoding="utf-8")))
print(len(rows), "rows; columns with id/name:", [c for c in rows[0] if "id" in c or c in ("short_name", "long_name")][:6])

idcol = "player_id" if "player_id" in rows[0] else "sofifa_id"
by = collections.defaultdict(dict)  # player -> edition -> row
for r in rows:
    try:
        by[r[idcol]][int(float(r["fifa_version"]))] = r
    except (ValueError, KeyError):
        pass


def num(r, c):
    try:
        return float(r[c])
    except (ValueError, KeyError):
        return None


# 1. mean change in overall from one edition to the next, by age at the first edition
delta = collections.defaultdict(list)
delta_pot = collections.defaultdict(list)
for pid, eds in by.items():
    for e, r in eds.items():
        n = eds.get(e + 1)
        if not n:
            continue
        a, o1, o2, p1 = num(r, "age"), num(r, "overall"), num(n, "overall"), num(r, "potential")
        if None in (a, o1, o2, p1):
            continue
        delta[int(a)].append(o2 - o1)
        gap = p1 - o1
        delta_pot[(int(a) // 3 * 3, min(int(gap) // 5 * 5, 20))].append(o2 - o1)

print("\nage: mean overall change to next edition (n), sd")
for a in range(16, 40):
    d = delta.get(a)
    if d and len(d) > 100:
        print(a, round(statistics.mean(d), 2), len(d), round(statistics.pstdev(d), 2))

print("\nage band x potential gap -> mean change (n)")
for (band, gap), d in sorted(delta_pot.items()):
    if len(d) > 200 and 16 <= band <= 30:
        print(f"age {band}-{band+2}, gap {gap}+: {statistics.mean(d):+.2f} ({len(d)})")

# 2. attribute changes by age bands for a few attributes
attrs = ["power_stamina", "movement_sprint_speed", "movement_acceleration", "power_strength", "attacking_finishing", "mentality_vision", "attacking_short_passing", "defending_standing_tackle", "mentality_aggression", "mentality_composure", "movement_agility", "skill_dribbling", "power_jumping"]
change = collections.defaultdict(lambda: collections.defaultdict(list))
for pid, eds in by.items():
    for e, r in eds.items():
        n = eds.get(e + 1)
        if not n:
            continue
        a = num(r, "age")
        if a is None:
            continue
        band = "17-20" if a <= 20 else "21-24" if a <= 24 else "25-28" if a <= 28 else "29-32" if a <= 32 else "33+"
        for c in attrs:
            x, y = num(r, c), num(n, c)
            if x is not None and y is not None:
                change[c][band].append(y - x)
print("\nmean attribute change to next edition by age band")
bands = ["17-20", "21-24", "25-28", "29-32", "33+"]
print("attribute".ljust(28), *[b.rjust(7) for b in bands])
for c in attrs:
    print(c.ljust(28), *[f"{statistics.mean(change[c][b]):+7.2f}" if change[c][b] else "      -" for b in bands])

# 3. goalkeepers vs outfield overall curve
gk = collections.defaultdict(list)
for pid, eds in by.items():
    for e, r in eds.items():
        n = eds.get(e + 1)
        if not n:
            continue
        pos = (r.get("player_positions") or "").split(",")[0].strip()
        a, o1, o2 = num(r, "age"), num(r, "overall"), num(n, "overall")
        if None in (a, o1, o2):
            continue
        grp = "GK" if pos == "GK" else "DEF" if pos in ("CB", "LB", "RB", "LWB", "RWB") else "MID" if pos in ("CM", "CDM", "CAM", "LM", "RM") else "ATT"
        gk[(grp, int(a) // 3 * 3)].append(o2 - o1)
print("\nposition group x age band: mean overall change")
for grp in ["GK", "DEF", "MID", "ATT"]:
    print(grp, " ".join(f"{b}:{statistics.mean(gk[(grp, b)]):+.2f}" for b in range(18, 39, 3) if len(gk[(grp, b)]) > 100))
