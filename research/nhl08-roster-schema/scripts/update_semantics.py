"""Metadata update semantics: the exact update="True" marking vs the save's update tables vs
Studio's editability reading (tdb-defs: updatable = attribute != "False"). Read-only.
  python update_semantics.py <save> <game.db> <meta.xml> [--group rosters]
"""
import os
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(__file__))
from tdblib import parse, scan, load_meta, roles, keys_of, update_fields_of

save_path, game_path, meta_path = sys.argv[1:4]
group = sys.argv[sys.argv.index("--group") + 1] if "--group" in sys.argv else "rosters"
b = open(save_path, "rb").read()
save = parse(b, scan(b)[0][0])
st = {t["raw"]: t for t in save["tables"]}
meta = load_meta(meta_path)
rl = roles(meta)

vals = Counter()
for a in meta.values():
    for f in a["fields"]:
        vals[f.get("update", "<absent>")] += 1
print("update attribute values over all %d fields of the meta: %s" % (sum(vals.values()), dict(vals)))
print("fields tdb-defs would treat as updatable (attribute != 'False'): %d ; fields explicitly marked update='True': %d" % (
    sum(v for k, v in vals.items() if k != "False"), vals.get("True", 0)))
kv = Counter(f.get("key", "<absent>") for a in meta.values() for f in a["fields"])
print("key attribute values:", dict(kv))
both = sum(1 for a in meta.values() for f in a["fields"] if f.get("key") == "True" and f.get("update") == "True")
print("fields that are both key and update=True: %d" % both)

print("\nper table in group %r: fields / update=True / unmarked / key ; unmarked field names ; update-table field set check" % group)
for sn, a in meta.items():
    if group not in a.get("savegroups", "").split(","):
        continue
    upd = update_fields_of(a)
    keys = keys_of(a)
    unmarked = [f["name"] for f in a["fields"] if f.get("update") != "True"]
    uid = a.get("updateshortname")
    check = "no update table declared"
    if uid and uid in st:
        have = {f["raw"] for f in st[uid]["fields"]}
        exp = set(upd) | set(keys)
        check = "update table %s == update+key: %s" % (uid, have == exp) if have == exp else "update table %s DIFFERS: +%s -%s" % (uid, sorted(have - exp), sorted(exp - have))
    elif uid:
        check = "update table %s declared but absent from this save" % uid
    print("  %-5s %-30s %3d / %3d / %2d / %-16s unmarked=%s ; %s" % (sn, a["name"], len(a["fields"]), len(upd), len(unmarked), ",".join(keys), unmarked, check))

print("\nfields marked update='True' across the whole meta whose table has NO updateshortname (marking without a write path):")
cnt = 0
for sn, a in meta.items():
    if update_fields_of(a) and not a.get("updateshortname"):
        cnt += 1
        print("  %-5s %-30s update=True fields %d groups=%s" % (sn, a["name"], len(update_fields_of(a)), a.get("savegroups", "")))
print("  tables: %d" % cnt)
print("\ntables with updateshortname whose maxupdate != -1:", [(a["name"], a.get("maxupdate")) for a in meta.values() if a.get("updateshortname") and a.get("maxupdate") != "-1"] or "none")
print("tables with insertshortname but no deleteshortname:", [a["name"] for a in meta.values() if a.get("insertshortname") and not a.get("deleteshortname")])
print("tables with deleteshortname but no insertshortname:", [a["name"] for a in meta.values() if a.get("deleteshortname") and not a.get("insertshortname")])
print("tables with updateshortname but neither insert nor delete:", [a["name"] for a in meta.values() if a.get("updateshortname") and not a.get("insertshortname") and not a.get("deleteshortname")])
print("difflistproperty values:", dict(Counter(a.get("difflistproperty", "<absent>") for a in meta.values())))
print("tables with savegroups but no difflist ids at all:", [(a["name"], a.get("savegroups")) for a in meta.values() if a.get("savegroups") and not any(a.get(k) for k in ("insertshortname", "deleteshortname", "updateshortname"))])
