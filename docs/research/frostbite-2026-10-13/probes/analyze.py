"""Summarise an inventory .jsonl: folder tree counts per superbundle, res types, chunk residency,
and keyword probes for the content families Studio edits. Read-only; prints to stdout.

Usage: python -I analyze.py <tag.jsonl> [depth]
"""
import collections
import json
import re
import sys

path = sys.argv[1]
depth = int(sys.argv[2]) if len(sys.argv) > 2 else 2

ebx = collections.Counter()
ebx_by_sb = collections.defaultdict(collections.Counter)
res_types = collections.Counter()
res_type_example = {}
names = set()
res_names = collections.defaultdict(set)
bundle_count = collections.Counter()
toc_chunks = collections.Counter()
cas_spread = collections.Counter()
for line in open(path, encoding="utf-8"):
    r = json.loads(line)
    if r["k"] == "tocchunk":
        toc_chunks[r["sb"]] += 1
        continue
    bundle_count[r["sb"]] += 1
    for cid, _o, _s in r["spans"]:
        cas_spread[f"{cid:#06x}"] += 1
    for n, _s in r["ebx"]:
        names.add(n)
        key = "/".join(n.split("/")[:depth])
        ebx[key] += 1
        ebx_by_sb[r["sb"]]["/".join(n.split("/")[:1])] += 1
    for n, _s, t in r["res"]:
        res_types[f"{t:#010x}"] += 1
        res_type_example.setdefault(f"{t:#010x}", n)
        res_names[f"{t:#010x}"].add(n)

print(f"# {path}: {len(names)} distinct ebx names, {sum(bundle_count.values())} bundles")
print("\n## ebx top-level folders per superbundle")
for sb, c in sorted(ebx_by_sb.items()):
    print(f"  {sb}: " + ", ".join(f"{k}={v}" for k, v in c.most_common(14)))
print(f"\n## ebx folders (depth {depth}), top 80")
for k, v in ebx.most_common(80):
    print(f"  {v:7d}  {k}")
print("\n## res types (count, example)")
for t, v in res_types.most_common(40):
    print(f"  {t} {v:7d}  e.g. {res_type_example[t]}")
print("\n## toc-level chunks per superbundle:", dict(toc_chunks))
probes = {
    "database/tdb": r"(^|/)(db|database|databases|tdb)(/|$)|\.db$|nhl_?game|roster",
    "localization": r"loc(ali[sz]ation)?/|strings?/|stringtable|/eng?_?us|language",
    "heads/faces": r"(^|/)(head|heads|face|faces|headshape|player[s]?/head)",
    "hair": r"hair",
    "kits/jerseys": r"jersey|uniform|kit[s]?/|socks|pants",
    "arena/ice": r"arena|rink|/ice/|centerice|centreice|venue|stadium",
    "boards": r"board|dasher|glass",
    "logos": r"logo|crest|emblem",
    "portraits": r"portrait|headshot|playerimage|mugshot",
    "equipment": r"stick|glove|helmet|skate|goalie|pad[s]?/|blocker|mask",
    "scorebug/overlay": r"scorebug|scoreboard|overlay|broadcast|hud",
    "audio": r"^sound/|audio|harmony|music|commentary|pbp|chant|horn",
    "fonts": r"font",
    "ui": r"^ui/|frontend|menu",
}
print("\n## family probes (distinct ebx names; 8 examples)")
for fam, pat in probes.items():
    rx = re.compile(pat, re.I)
    hit = sorted(n for n in names if rx.search(n))
    tops = collections.Counter("/".join(n.split("/")[:2]) for n in hit)
    print(f"  {fam}: {len(hit)} | top folders: {', '.join(f'{k}={v}' for k, v in tops.most_common(6))}")
    for n in hit[:: max(1, len(hit) // 8)][:8]:
        print(f"      {n}")
