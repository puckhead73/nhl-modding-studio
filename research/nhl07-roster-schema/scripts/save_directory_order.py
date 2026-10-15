"""Does the save's table directory follow the meta XML document order of the parent tables, with
insert/delete/update adjacent per parent? (Supports the adjacency resolution of the two
schema-identical AI pairs.) Read-only. Run from the repository root.
"""
import sys, os, re
sys.path.insert(0, os.path.dirname(__file__))
from tdb_scan import parse

x = open('anchors/xbox360/nhl07/databases/NHL07NG-meta.xml', encoding='utf-8-sig').read()
order = [(m.group(2), m.group(1)) for m in re.finditer(r'<table name="([^"]+)" shortname="(\w{4})"', x)]
pos = {sn: i for i, (sn, _) in enumerate(order)}
name = dict(order)
role = {}
for m in re.finditer(r'<table\b([^>]*)>', x):
    a = dict(re.findall(r'(\w+)="([^"]*)"', m.group(1)))
    if a.get('insertshortname'):
        role[a['insertshortname']] = (a['shortname'], 'insert')
    if a.get('deleteshortname'):
        role[a['deleteshortname']] = (a['shortname'], 'delete')
upd = dict(CvSu='cPbu', MVOx='EhOn', HHLk='MqdK', RosT='yvSd', uyGF='yuHm', vozd='ulGe', mASv='oYeE',
           Iblm='bZvB', rcpY='YzeG', zrsU='oDHy', DZfz='dDrd', YvhR='Raym')
for k, v in upd.items():
    role[k] = (v, 'update')
s = parse(open('anchors/xbox360/nhl07/savefiles/ROSTER1/ROSTER1', 'rb').read(), 0x1c)
print("save directory order -> parent (meta document position), role")
prev = -1
monotone = True
for t in s['tables']:
    p, r = role[t['raw']]
    print("  %-5s -> %-5s #%-3d %-28s %s" % (t['raw'], p, pos[p], name[p], r))
    if pos[p] < prev:
        monotone = False
    prev = pos[p]
print("parents appear in meta document order:", monotone)
g = parse(open('anchors/xbox360/nhl07/databases/NHL07NG.db', 'rb').read(), 0)
print("game DB directory order == meta document order:", [t['raw'] for t in g['tables']] == [sn for sn, _ in order])
