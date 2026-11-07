"""Fits how a player's overall rating is built from his attributes, for one edition of the data.

Run from the project root:  py -I research/game-features/fit_overall.py [edition]   (default 24)
Needs numpy and history.db. Prints one markdown table row per position: how well the fit matches the real
overall, and the attribute weights (a weight under 0.02 is left out). Results are in overall-rating-formula.md.
"""
import re
import sqlite3
import sys

import numpy as np

EDITION = int(sys.argv[1]) if len(sys.argv) > 1 else 24
MIN_PLAYERS = 100  # positions with fewer players than this give a noisy fit

# The attribute columns, read from the engine so there is one list of them.
source = open("crates/engine/src/attrs.rs").read()
columns = re.findall(r'^\s+[A-Z][A-Za-z]*\s+"[^"]+"\s+"([a-z_]+)",', source, re.M)

db = sqlite3.connect("history.db")
query = f"SELECT player_positions, overall, {', '.join(columns)} FROM ratings WHERE fifa_version = ? AND overall IS NOT NULL"
by_position = {}
for positions, overall, *attrs in db.execute(query, (EDITION,)):
    main = (positions or "").split(",")[0].strip()
    if main and None not in attrs:  # only players with every attribute
        by_position.setdefault(main, []).append([overall] + attrs)

print(f"Edition {EDITION}. Weights of the attributes, per main position (under 0.02 left out).\n")
for position, rows in sorted(by_position.items(), key=lambda item: -len(item[1])):
    if len(rows) < MIN_PLAYERS:
        continue
    data = np.array(rows, float)
    y, x = data[:, 0], np.hstack([data[:, 1:], np.ones((len(data), 1))])  # last column: a constant
    weights, *_ = np.linalg.lstsq(x, y, rcond=None)
    error = np.abs(x @ weights - y)
    exact, within_one = (np.round(x @ weights) == y).mean(), (error <= 1).mean()
    shown = sorted(((w, c) for w, c in zip(weights[:-1], columns) if abs(w) >= 0.02), reverse=True)
    text = ", ".join(f"{c.split('_', 1)[-1]} {w:.2f}" for w, c in shown)
    print(f"- **{position}** ({len(rows)} players, exact {exact:.0%}, within 1: {within_one:.0%}, "
          f"weights add up to {weights[:-1].sum():.2f}): {text}")
