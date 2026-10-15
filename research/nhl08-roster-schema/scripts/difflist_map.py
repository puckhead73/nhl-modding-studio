"""Complete save-table -> parent-table mapping for a difflist save, with descriptor verification.
  python difflist_map.py <save> <game.db> <meta.xml> [--group rosters]
For every save table: role (insert/delete/update) and parent from the meta's own
insertshortname / deleteshortname / updateshortname declarations; independent cross-check by
exact field-set identity against every table of the meta; key field(s); capacity; then a
per-field descriptor comparison with the parent (type kind, bit offset, width, order).
"""
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from tdblib import parse, scan, load_meta, roles, keys_of, update_fields_of, field_names, capacity

save_path, game_path, meta_path = sys.argv[1:4]
group = sys.argv[sys.argv.index("--group") + 1] if "--group" in sys.argv else "rosters"
b = open(save_path, "rb").read()
base = scan(b)[0][0]
save = parse(b, base)
game = parse(open(game_path, "rb").read(), 0)
gt = {t["raw"]: t for t in game["tables"]}
meta = load_meta(meta_path)
rl = roles(meta)

print("meta: %d tables; declared insert ids %d, delete ids %d, update ids %d" % (
    len(meta), sum("insertshortname" in a for a in meta.values()), sum("deleteshortname" in a for a in meta.values()),
    sum("updateshortname" in a for a in meta.values())))
groups = {}
for sn, a in meta.items():
    for g in a.get("savegroups", "").split(","):
        if g:
            groups.setdefault(g, []).append(sn)
print("savegroups:", {g: len(v) for g, v in groups.items()})
print("save: %d tables at %#x" % (save["table_count"], base))

# independent field-set cross-check: which meta tables have (all fields) / (update+key) / (key) == save field set
def fieldset_matches(fs):
    hits = []
    for sn, a in meta.items():
        allf = frozenset(f["shortname"] for f in a["fields"])
        upd = frozenset(update_fields_of(a)) | frozenset(keys_of(a))
        key = frozenset(keys_of(a))
        # the three labels are tested independently: a parent whose every non-key field is marked
        # update="True" legitimately matches both "all-fields" and "update+key".
        if fs == allf:
            hits.append((sn, "all-fields"))
        if fs == upd:
            hits.append((sn, "update+key"))
        if fs == key:
            hits.append((sn, "key-only"))
    return hits

print("\n== mapping ==")
print("%-5s %-7s %-5s %-30s %-6s %4s %5s %-14s %8s %-24s %s" % ("save", "role", "parent", "parent name", "inGame", "nf", "recB", "key(s)", "capacity", "capacity source", "field-set cross-check"))
rows = []
for t in save["tables"]:
    fs = frozenset(f["raw"] for f in t["fields"])
    xc = fieldset_matches(fs)
    r = rl.get(t["raw"])
    if not r:
        print("%-5s UNDECLARED  field-set matches: %s" % (t["raw"], xc))
        continue
    parent, role = r
    a = meta[parent]
    pt = gt.get(parent)
    cap, src = capacity(a, role, pt)
    expect = {"insert": "all-fields", "delete": "key-only", "update": "update+key"}[role]
    agree = [sn for sn, kind in xc if kind == expect]
    verdict = ("unique" if agree == [parent] else "parent+%d other(s) %s" % (len(agree) - 1, [s for s in agree if s != parent]) if parent in agree else "DISAGREES %s" % xc)
    rows.append((t, parent, role, a, pt, cap))
    print("%-5s %-7s %-5s %-30s %-6s %4d %5d %-14s %8s %-24s %s" % (
        t["raw"], role, parent, a["name"], "yes" if pt else "NO", t["nfields"], t["rec_bytes"], ",".join(keys_of(a)), cap, src, verdict))

print("\nparents in save group %r:" % group)
for sn in groups.get(group, []):
    have = [t["raw"] + ":" + rl[t["raw"]][1] for t in save["tables"] if rl.get(t["raw"], ("",))[0] == sn]
    a = meta[sn]
    print("  %-5s %-30s declared ins=%-5s del=%-5s upd=%-5s in save: %s" % (sn, a["name"], a.get("insertshortname"), a.get("deleteshortname"), a.get("updateshortname"), have or "NONE"))
other = [t["raw"] for t in save["tables"] if rl.get(t["raw"]) and group not in meta[rl[t["raw"]][0]].get("savegroups", "")]
print("save tables whose parent is NOT in group %r: %s" % (group, other or "none"))

print("\n== descriptor comparison with the parent (binary descriptors of the game DB) ==")
for t, parent, role, a, pt, cap in rows:
    if not pt:
        print("%s: parent %s absent from game DB" % (t["raw"], parent))
        continue
    pf = {f["raw"]: f for f in pt["fields"]}
    names = field_names(a)
    problems = []
    same_offsets = True
    for i, f in enumerate(t["fields"]):
        q = pf.get(f["raw"])
        if not q:
            problems.append("field %s not in parent" % f["raw"])
            continue
        if (f["kind"], f["width"]) != (q["kind"], q["width"]):
            problems.append("%s kind/width %s/%s vs parent %s/%s" % (f["raw"], f["kind"], f["width"], q["kind"], q["width"]))
        if f["bit"] != q["bit"]:
            same_offsets = False
    order_same = [f["raw"] for f in t["fields"]] == [f["raw"] for f in pt["fields"] if f["raw"] in {x["raw"] for x in t["fields"]}]
    # is the save table a dense re-pack in parent descriptor order?
    packed = True
    pos = 0
    for f in sorted(t["fields"], key=lambda f: f["bit"]):
        if f["bit"] != pos:
            packed = False
        pos += f["width"]
    expect_set = {"insert": set(pf), "delete": set(keys_of(a)), "update": set(update_fields_of(a)) | set(keys_of(a))}[role]
    have = {f["raw"] for f in t["fields"]}
    setok = have == expect_set
    rb = "recB %d vs parent %d" % (t["rec_bytes"], pt["rec_bytes"])
    print("%-5s %-7s -> %-5s %-28s fields %2d/%2d set=%s offsets=%s order=%s dense-pack=%s %s %s" % (
        t["raw"], role, parent, a["name"], t["nfields"], pt["nfields"], "expected" if setok else "UNEXPECTED(+%s -%s)" % (sorted(have - expect_set), sorted(expect_set - have)),
        "parent" if same_offsets else "re-packed", "parent" if order_same else "DIFFERENT", packed, rb, "; ".join(problems) or "descriptors ok"))
    if role == "update" and not same_offsets:
        dropped = [names.get(f["raw"], f["raw"]) + ":%d@%d" % (f["width"], f["bit"]) for f in pt["fields"] if f["raw"] not in have]
        print("        parent fields absent from the update table (name:width@parent_bit): %s" % dropped)
        shown = ["%s@%d->%d" % (names.get(f["raw"], f["raw"]), pf[f["raw"]]["bit"], f["bit"]) for f in t["fields"] if f["bit"] != pf[f["raw"]]["bit"]][:6]
        print("        first shifted fields (parent_bit->save_bit): %s" % shown)
