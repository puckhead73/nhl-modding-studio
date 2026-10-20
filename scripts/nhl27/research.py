"""Reproducible, offline NHL 27 ELF indexing and NHL 24 comparison.

Game input is read only. Derived binaries/databases belong in ignored local/.
Run: python scripts/nhl27/research.py index EBOOT OUTPUT
     python scripts/nhl27/research.py survey OUTPUT NHL24_BASE NHL24_UPDATE
"""
import argparse
import hashlib
import json
import re
import sqlite3
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'nhl24'))
from inspect_eboot import Image as SelfImage
import index_refs


class ElfImage(SelfImage):
    """Use program headers, never the stale section table of this supplied dump."""
    def __init__(self, path):
        self.path = Path(path)
        self.data = self.path.read_bytes()
        h = struct.unpack_from('<16sHHIQQQIHHHHHH', self.data)
        if h[0][:6] != b'\x7fELF\x02\x01' or h[2] != 62:
            raise ValueError('Expected little-endian x86-64 ELF')
        self.header = h
        if h[9] != 56 or h[5] + h[9] * h[10] > len(self.data):
            raise ValueError('Invalid program header table')
        self.ph = []
        for i in range(h[10]):
            p = dict(zip(('type', 'flags', 'offset', 'va', 'pa', 'filesz', 'memsz', 'align'),
                         struct.unpack_from('<IIQQQQQQ', self.data, h[5] + i * h[9])))
            if p['offset'] + p['filesz'] > len(self.data):
                raise ValueError(f'Program header {i} exceeds input')
            self.ph.append(p)
        self.segments = [dict(p, self_offset=p['offset'], index=i)
                         for i, p in enumerate(self.ph) if p['type'] in (1, 0x61000010)]
        for p in self.segments:
            if p['filesz'] > p['memsz']:
                raise ValueError('PT_LOAD file size exceeds memory size')


TOPICS = {
    'strategies': r'strateg|forecheck|breakout|neutral.?zone|tactic|line.?bias',
    'franchise': r'franchise|dynasty|morale|contract|negotiat|trade.?protect|coach|conversation',
    'pwhl_rules': r'pwhl|jail.?break|short.?hand|penalty.?release|womens|women.?league',
    'be_a_pro': r'be.?a.?pro|beapro|bap_|bap/|career|archetype|storyline',
    'league_roster': r'league.*(?:max|min|count|size)|(?:max|min).*(?:team|roster|skater|goalie)|roster.*(?:size|valid)|forward.?pass|two.?line',
    'appearance': r'headgeometry|headtexture|portrait|headvariant|player_art',
    'stats_sim': r'giveaway|takeaway|turnover|hit.?count|hit.?stat|simengine|simulation|allstar',
    'ai_ratings': r'pokecheck|deke|pressure|gAttrib|gPlayerAttribute|gGoalieAttribute',
    'ones': r'ones_now|ones_h2h|gOnesCPU|eliminatorones',
}


def index(source, output):
    output.mkdir(parents=True, exist_ok=True)
    im = ElfImage(source)
    h = im.header
    meta = {'path': str(source.resolve()), 'size': len(im.data),
            'sha256': hashlib.sha256(im.data).hexdigest(), 'program_headers': im.ph,
            'section_table': {'offset': h[6], 'entry_size': h[11], 'count': h[12],
                              'in_bounds': h[6] + h[11] * h[12] <= len(im.data)}}
    (output / 'identity.json').write_text(json.dumps(meta, indent=2))
    # Remove only invalid section metadata in the analysis copy; preserve payloads.
    normalized = bytearray(im.data)
    if not meta['section_table']['in_bounds']:
        struct.pack_into('<Q', normalized, 40, 0)
        struct.pack_into('<HHH', normalized, 58, 0, 0, 0)
    (output / 'analysis.elf').write_bytes(normalized)
    meta['analysis_sha256'] = hashlib.sha256(normalized).hexdigest()
    meta['analysis_changed_offsets'] = [i for i in range(64) if normalized[i] != im.data[i]]
    (output / 'identity.json').write_text(json.dumps(meta, indent=2))
    with sqlite3.connect(output / 'index.sqlite') as db:
        db.executescript('DROP TABLE IF EXISTS strings; CREATE TABLE strings(va INTEGER PRIMARY KEY,self_offset INTEGER,text TEXT); DROP TABLE IF EXISTS refs; CREATE TABLE refs(src INTEGER,dst INTEGER,kind TEXT);')
        for s in im.mapped():
            raw = im.data[s['offset']:s['offset'] + s['filesz']]
            db.executemany('INSERT OR IGNORE INTO strings VALUES(?,?,?)',
                ((s['va'] + m.start(), s['offset'] + m.start(), m[0][:-1].decode('ascii'))
                 for m in re.finditer(rb'[\x20-\x7e]{4,}\x00', raw)))
        print('strings', db.execute('SELECT count(*) FROM strings').fetchone()[0], flush=True)
    # Reuse the instruction-candidate indexer, with the raw ELF reader injected.
    index_refs.Image = ElfImage
    index_refs.run(source, output)


def survey(output, base, update):
    db = sqlite3.connect(output / 'index.sqlite')
    old = {}
    for label, root in [('base', base), ('update', update)]:
        with sqlite3.connect(f'file:{(root / "index.sqlite").as_posix()}?mode=ro', uri=True) as d:
            old[label] = {r[0] for r in d.execute('SELECT text FROM strings')}
    rows = db.execute('SELECT va,text FROM strings').fetchall()
    result = {}
    for topic, expression in TOPICS.items():
        pattern = re.compile(expression, re.I)
        found = []
        for va, name in rows:
            if not pattern.search(name):
                continue
            refs = []
            for src, kind in db.execute('SELECT src,kind FROM refs WHERE dst=?', (va,)):
                f = db.execute('SELECT start FROM functions WHERE start<=? ORDER BY start DESC LIMIT 1', (src,)).fetchone()
                refs.append({'instruction': hex(src), 'kind': kind,
                             'preceding_unwind_start': hex(f[0]) if f else None})
            found.append({'va': hex(va), 'text': name, 'in_nhl24_base': name in old['base'],
                          'in_nhl24_update': name in old['update'], 'candidate_refs': refs})
        result[topic] = found
    (output / 'topic-survey.json').write_text(json.dumps(result, indent=2))
    print(json.dumps({k: {'strings': len(v), 'with_candidate_refs': sum(bool(x['candidate_refs']) for x in v),
                         'absent_from_nhl24': sum(not x['in_nhl24_base'] and not x['in_nhl24_update'] for x in v)}
                      for k, v in result.items()}, indent=2))


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    sub = p.add_subparsers(dest='command', required=True)
    i = sub.add_parser('index'); i.add_argument('source', type=Path); i.add_argument('output', type=Path)
    s = sub.add_parser('survey'); s.add_argument('output', type=Path); s.add_argument('base', type=Path); s.add_argument('update', type=Path)
    a = p.parse_args()
    if a.command == 'index': index(a.source, a.output)
    else: survey(a.output, a.base, a.update)
