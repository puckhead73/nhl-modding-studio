"""Extract local NHL24 ANT controls and compare names, not animation semantics.

Usage: python scripts/nhl27/ant_probe.py COMPATIBILITY_ROOT
Requires compatibility.py inventories. Never treats an unmatched name as a new move.
"""
import hashlib
import json
import re
import sys
from pathlib import Path
from compatibility import Install,frames,Oodle


def run(root):
    decoder=Oodle();results=[];old=set()
    for label,layer in [('nhl24_base','Data'),('nhl24_160','Patch')]:
        inventory=json.loads((root/f'{label}-{layer}_Gen4b_levels_game_game.toc.json').read_text())
        for bundle in inventory['bundles']:
            for a in bundle['assets']:
                if a.get('resource_type')!='0x51a3c853':continue
                raw=Install(label).read(a['span']);blocks,data=frames(raw,decoder)
                if data is None or len(data)!=a['size'] or hashlib.sha1(raw).hexdigest()!=a['sha1']:raise ValueError('ANT identity/size mismatch')
                dest=root/'ant'/label;dest.mkdir(parents=True,exist_ok=True);(dest/'state.bin').write_bytes(data)
                strings=sorted({x[0].decode() for x in re.finditer(rb'[ -~]{5,}',data)})
                (dest/'strings.json').write_text(json.dumps(strings));old.update(s.lower() for s in strings)
                results.append(dict(build=label,name=a['name'],stored_sha1=a['sha1'],decoded_sha256=hashlib.sha256(data).hexdigest(),decoded_size=len(data),expected_size=a['size'],strings=len(strings),header=data[:64].hex()))
    (root/'ant-evidence.json').write_text(json.dumps(results,indent=2))
    rows=[]
    for x in json.loads((root/'comparison.json').read_text())['deke']['items']:
        name=x['name'];variants=[name,name.removeprefix('af/assets/'),name.rsplit('/',1)[-1]]
        rows.append(dict(kind=x['kind'],name=name,ant_name_matches=[v for v in variants if v.lower() in old]))
    (root/'deke-ant-name-comparison.json').write_text(json.dumps(rows,indent=2))
    print('ANT resources',len(results),'source deke records',len(rows),'name matches',sum(bool(x['ant_name_matches']) for x in rows))


if __name__=='__main__':run(Path(sys.argv[1]))
