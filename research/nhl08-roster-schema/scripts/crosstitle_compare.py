"""NHL 07 vs NHL 08 Xbox 360: what the roster difflist model shares and what differs. Read-only.
  python crosstitle_compare.py   (run from the repository root)
Also re-checks the NHL 07 report's update-table inferences against the NHL 07 meta's own
updateshortname declarations, which that report did not use.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from tdblib import parse, scan, load_meta, roles, keys_of, update_fields_of

T = {
    "nhl07": dict(meta="anchors/xbox360/nhl07/databases/NHL07NG-meta.xml", db="anchors/xbox360/nhl07/databases/NHL07NG.db",
                  roster="anchors/xbox360/nhl07/savefiles/ROSTER1/ROSTER1"),
    "nhl08": dict(meta="anchors/xbox360/nhl08/databases/nhlng-meta.xml", db="anchors/xbox360/nhl08/databases/nhlng.db",
                  roster="anchors/xbox360/nhl08/savefiles/ROSTER1 20260718064658/ROSTER1 20260718064658"),
}
M, R, G = {}, {}, {}
for k, v in T.items():
    M[k] = load_meta(v["meta"])
    rb = open(v["roster"], "rb").read()
    R[k] = (rb, parse(rb, scan(rb)[0][0]))
    G[k] = parse(open(v["db"], "rb").read(), 0)

print("== containers ==")
for k in T:
    rb, p = R[k]
    print("  %s roster: file %d bytes, TDB @%#x db_size %d tables %d, tail %d; game DB tables %d" % (
        k, len(rb), p["base"], p["db_size"], p["table_count"], len(rb) - p["base"] - p["db_size"], G[k]["table_count"]))

print("\n== meta vocabulary ==")
for k in T:
    m = M[k]
    print("  %s: tables %d, insert ids %d, delete ids %d, update ids %d, difflistproperty %s, savegroups %s" % (
        k, len(m), sum("insertshortname" in a for a in m.values()), sum("deleteshortname" in a for a in m.values()),
        sum("updateshortname" in a for a in m.values()), sorted({a.get("difflistproperty") for a in m.values()}),
        sorted({g for a in m.values() for g in a.get("savegroups", "").split(",") if g})))

print("\n== NHL 07 report's inferred update parents vs the NHL 07 meta's declared updateshortname ==")
NHL07_REPORT = dict(CvSu="cPbu", MVOx="EhOn", HHLk="MqdK", RosT="yvSd", uyGF="yuHm", vozd="ulGe",
                    mASv="oYeE", Iblm="bZvB", rcpY="YzeG", zrsU="oDHy", DZfz="dDrd", YvhR="Raym")
r7 = roles(M["nhl07"])
ok = 0
for sid, parent in NHL07_REPORT.items():
    d = r7.get(sid)
    verdict = "DECLARED same" if d == (parent, "update") else "declared %s" % (d,)
    ok += d == (parent, "update")
    print("  %s -> %s (report) ; meta: %s" % (sid, parent, verdict))
print("  agreement: %d/%d" % (ok, len(NHL07_REPORT)))

print("\n== declared difflist ids for parents present in both metas ==")
same = diff = 0
for sn in sorted(set(M["nhl07"]) & set(M["nhl08"])):
    a, c = M["nhl07"][sn], M["nhl08"][sn]
    ids7 = (a.get("insertshortname"), a.get("deleteshortname"), a.get("updateshortname"))
    ids8 = (c.get("insertshortname"), c.get("deleteshortname"), c.get("updateshortname"))
    if any(ids7) or any(ids8):
        eq = ids7 == ids8
        same += eq
        diff += not eq
        if not eq:
            print("  %-5s %-30s / %-30s 07=%s 08=%s" % (sn, a["name"], c["name"], ids7, ids8))
print("  identical (ins,del,upd) triples: %d ; differing: %d" % (same, diff))
print("  parents only in NHL 07: %s" % sorted(M["nhl07"][x]["name"] for x in set(M["nhl07"]) - set(M["nhl08"])))
print("  parents only in NHL 08: %s" % sorted(M["nhl08"][x]["name"] for x in set(M["nhl08"]) - set(M["nhl07"])))

print("\n== rosters-group parents: schema drift ==")
g7 = {sn for sn, a in M["nhl07"].items() if "rosters" in a.get("savegroups", "").split(",")}
g8 = {sn for sn, a in M["nhl08"].items() if "rosters" in a.get("savegroups", "").split(",")}
print("  rosters group: NHL 07 %d tables, NHL 08 %d tables; common %d" % (len(g7), len(g8), len(g7 & g8)))
print("  only NHL 07: %s" % sorted(M["nhl07"][x]["name"] for x in g7 - g8))
print("  only NHL 08: %s" % sorted(M["nhl08"][x]["name"] for x in g8 - g7))
gt7 = {t["raw"]: t for t in G["nhl07"]["tables"]}
gt8 = {t["raw"]: t for t in G["nhl08"]["tables"]}
for sn in sorted(g7 & g8, key=lambda s: M["nhl08"][s]["name"]):
    a, c = M["nhl07"][sn], M["nhl08"][sn]
    f7 = {f["shortname"]: f for f in a["fields"]}
    f8 = {f["shortname"]: f for f in c["fields"]}
    common = set(f7) & set(f8)
    depth_diff = [f7[x]["name"] + ":%s->%s" % (f7[x]["depth"], f8[x]["depth"]) for x in common if f7[x]["depth"] != f8[x]["depth"]]
    upd_diff = [f7[x]["name"] for x in common if f7[x].get("update") != f8[x].get("update")]
    key_diff = [x for x in common if f7[x].get("key") != f8[x].get("key")]
    print("  %-5s %-28s fields %2d->%2d recB %3s->%3s rows %4s->%4s; only07=%s only08=%s depth-diff=%s update-flag-diff=%s key-diff=%s keys %s->%s" % (
        sn, c["name"], len(f7), len(f8), gt7[sn]["rec_bytes"] if sn in gt7 else "-", gt8[sn]["rec_bytes"] if sn in gt8 else "-",
        gt7[sn]["cur"] if sn in gt7 else "-", gt8[sn]["cur"] if sn in gt8 else "-",
        sorted(f7[x]["name"] for x in set(f7) - set(f8)), sorted(f8[x]["name"] for x in set(f8) - set(f7)), depth_diff, upd_diff, key_diff, keys_of(a), keys_of(c)))

print("\n== save tables: ids shared by both roster saves ==")
s7 = {t["raw"]: t for t in R["nhl07"][1]["tables"]}
s8 = {t["raw"]: t for t in R["nhl08"][1]["tables"]}
r8 = roles(M["nhl08"])
print("  shared save ids: %d of 07=%d / 08=%d" % (len(set(s7) & set(s8)), len(s7), len(s8)))
for sid in sorted(set(s7) & set(s8)):
    print("    %s 07:%s(%s) nf %d recB %d | 08:%s(%s) nf %d recB %d" % (
        sid, r7.get(sid, ("?", "?"))[0], r7.get(sid, ("?", "?"))[1], s7[sid]["nfields"], s7[sid]["rec_bytes"],
        r8.get(sid, ("?", "?"))[0], r8.get(sid, ("?", "?"))[1], s8[sid]["nfields"], s8[sid]["rec_bytes"]))
print("  only NHL 07 save: %s" % sorted("%s=%s:%s" % (x, r7[x][0], r7[x][1]) for x in set(s7) - set(s8) if x in r7))
print("  only NHL 08 save: %s" % sorted("%s=%s:%s" % (x, r8[x][0], r8[x][1]) for x in set(s8) - set(s7) if x in r8))
