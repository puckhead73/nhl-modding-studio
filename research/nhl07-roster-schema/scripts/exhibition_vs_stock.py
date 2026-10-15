"""Byte-compare the NHL07NG.db 'exhibition*' working tables with their 'stock*'/'default*' twins.
Read-only. Run from the repository root.
"""
import sys, os
sys.path.insert(0, os.path.dirname(__file__))
from tdb_scan import parse

db = open('anchors/xbox360/nhl07/databases/NHL07NG.db', 'rb').read()
p = parse(db, 0)
T = {t['raw']: t for t in p['tables']}


def blk(t):
    return db[t['record_block']:t['record_block'] + t['rec_bytes'] * t['cur']]


pairs = [('cPbu', 'aUhv', 'exhibitionplayerbiotable vs stockplayerbiotable'),
         ('ulGe', 'Smgy', 'exhibitionrostertable vs defaultrostertable'),
         ('yuHm', 'TJKL', 'exhibitiongoalieai vs stockgoalieai'),
         ('yvSd', 'LsXI', 'exhibitionskaterai vs stockskaterai'),
         ('QEoV', 'TrVV', 'exhibitionfreeagents vs defaultfreeagents'),
         ('EhOn', 'BojX', 'exhibitiondownloadplayerbio vs downloadplayerbiotable'),
         ('Raym', 'dZCE', 'exhibitiondownloadskaterai vs downloadskaterai'),
         ('dDrd', 'FovP', 'exhibitiondownloadgoalieai vs downloadgoalieai'),
         ('bZvB', 'AOIh', 'rosterversiontable vs defaultrosterversiontable')]
for a, b, label in pairs:
    ta, tb = T[a], T[b]
    same_schema = [(f['raw'], f['bit'], f['width']) for f in ta['fields']] == [(f['raw'], f['bit'], f['width']) for f in tb['fields']]
    ba, bb = blk(ta), blk(tb)
    diff = sum(1 for i in range(0, min(len(ba), len(bb)), ta['rec_bytes']) if ba[i:i + ta['rec_bytes']] != bb[i:i + ta['rec_bytes']])
    print("%-58s schema identical=%s rows %d/%d record block byte-identical=%s differing records=%d" % (
        label, same_schema, ta['cur'], tb['cur'], ba == bb, diff))
