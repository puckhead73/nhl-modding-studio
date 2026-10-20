"""Inspect intro/scoreboard bundles and UI controls separately from game manifests.

Usage: python scripts/nhl27/presentation_probe.py COMPATIBILITY_ROOT
Records explicit failures; bundle filters are a sample, not full presentation coverage.
"""
import collections
import hashlib
import json
import re
import sys
from pathlib import Path
from compatibility import Install,frames,Oodle
from riff_evidence import fixup

PATTERN=r'scoreclock|scoreboard|pregame|pre_game|gameintro|game_intro|broadcast'


def run(root):
    dest=root/'presentation';dest.mkdir(parents=True,exist_ok=True);summary=[];errors=[];probes=[];decoder=Oodle()
    for label in ['nhl27','nhl24_base','nhl24_160']:
        ins=Install(label);layers=['Data','Patch'] if label=='nhl27' else ['Patch' if label=='nhl24_160' else 'Data'];seen=set()
        for layer in layers:
            for sb in ['ui','contentlaunchsb','contentsb','worldssb','movies']:
                relative=f'{layer}/{ins.platform}/{sb}.toc'
                if not (ins.root/relative).is_file():continue
                try:inv=ins.inventory(relative,'.*' if sb=='ui' and label=='nhl24_base' else PATTERN)
                except (ValueError,KeyError,FileNotFoundError) as e:errors.append(dict(build=label,toc=relative,error=str(e)));continue
                (dest/f'{label}-{layer}-{sb}.json').write_text(json.dumps(inv))
                assets=[a for b in inv['bundles'] for a in b['assets']]
                summary.append(dict(build=label,toc=relative,bundles=len(inv['bundles']),assets=len(assets),resource_types=dict(collections.Counter(a.get('resource_type',a['kind']) for a in assets))))
                candidates=sorted([a for a in assets if re.search(PATTERN,a['name'],re.I)],key=lambda a:(a['kind'],a['name']))
                for kind in ['ebx','res']:
                    selected=0
                    for a in candidates:
                        if a['kind']!=kind or a['sha1'] in seen:continue
                        seen.add(a['sha1']);selected+=1
                        if selected>12:break
                        row=dict(build=label,toc=relative,name=a['name'],kind=kind,resource_type=a.get('resource_type'),manifest_sha1=a['sha1'])
                        try:
                            raw=ins.read(a['span']);_,b=frames(raw,decoder)
                            if b is None:raise ValueError('Unsupported codec')
                            if len(b)!=a['size'] or hashlib.sha1(raw).hexdigest()!=a['sha1']:raise ValueError('Size/hash mismatch')
                            row.update(size=len(b),header=b[:64].hex(),sha256=hashlib.sha256(b).hexdigest())
                            path=dest/'payloads'/label/(a['sha1']+'.bin');path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(b);row['payload']=str(path)
                            if kind=='ebx':row['fixup']=fixup(b)
                        except (ValueError,KeyError) as e:row['error']=str(e)
                        probes.append(row)
                print(label,relative,'bundles',len(inv['bundles']),'assets',len(assets),flush=True)
    (dest/'summary.json').write_text(json.dumps(dict(inventories=summary,errors=errors),indent=2))
    (dest/'probes.json').write_text(json.dumps(probes,indent=2))
    print('probes',len(probes),'errors',errors)


if __name__=='__main__':run(Path(sys.argv[1]))
