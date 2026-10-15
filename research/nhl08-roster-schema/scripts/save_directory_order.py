"""Save directory order vs meta document order of the parents, and role adjacency. Read-only.
  python save_directory_order.py <save> <game.db> <meta.xml>
"""
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from tdblib import parse, scan, load_meta, roles

save_path, game_path, meta_path = sys.argv[1:4]
b = open(save_path, "rb").read()
save = parse(b, scan(b)[0][0])
game = parse(open(game_path, "rb").read(), 0)
meta = load_meta(meta_path)
rl = roles(meta)
gorder = {t["raw"]: i for i, t in enumerate(game["tables"])}
print("%-3s %-5s %-7s %-5s %-30s %9s %9s" % ("#", "save", "role", "parent", "parent name", "meta#", "gameDir#"))
seq = []
for i, t in enumerate(save["tables"]):
    parent, role = rl.get(t["raw"], ("?", "?"))
    a = meta.get(parent, {})
    seq.append((a.get("order", -1), role))
    print("%-3d %-5s %-7s %-5s %-30s %9s %9s" % (i, t["raw"], role, parent, a.get("name", "?"), a.get("order", "?"), gorder.get(parent, "?")))
orders = [o for o, _ in seq]
print("\nparent meta order non-decreasing along the save directory:", orders == sorted(orders))
print("parent game-directory order non-decreasing:", [gorder.get(rl[t['raw']][0]) for t in save['tables']] == sorted(gorder.get(rl[t['raw']][0]) for t in save['tables']))
pat = {}
cur = None
for o, r in seq:
    if o != cur:
        cur = o
        pat.setdefault(o, [])
    pat[o].append(r)
print("role sequence per parent:", sorted(set(tuple(v) for v in pat.values())))
print("meta attribute order on a declaring table (document text):")
import re
x = open(meta_path, encoding="utf-8-sig").read()
m = re.search(r'<table\b[^>]*updateshortname[^>]*>', x)
print("  ", m.group(0)[:300])
