"""Compile missing checking resources used by source panic eligibility.

Retains exported identities and native dependencies. Production of mutable
rally/failed-check signals remains an explicit installation requirement.
"""
import argparse
import hashlib
import json
from pathlib import Path
from ant_tags import compile_tree
from deke_gesture_audit import Source, selected
from deke_native_audit import native_id
from native_ant import Bank

ROOT_PARTITION='5e0b0834-4b68-d6a2-5e0b-08344b68d6a2'
FIELDS=('RallyShoulderCheckRequestState','IsInteracting','IsInaFaildedShoulderCheckNode')

def run(root):
    source=Source(root)
    banks=[Bank((root/'ant/nhl24_160'/f'{name}.bin').read_bytes()) for name in ('state','launch')]
    native=set().union(*(b.by_id for b in banks))
    occupied=set(native)
    checking=source.partition(ROOT_PARTITION)['instances'][0]['fields']
    pending=[checking[name] for name in FIELDS]
    documents={}
    def references(value):
        if isinstance(value,dict):
            if 'external' in value:yield value
            else:
                for v in value.values():yield from references(v)
        elif isinstance(value,list):
            for v in value:yield from references(v)
    while pending:
        ref=pending.pop();ident=native_id(ref['external']['instance'])
        if ident in native or ident in documents:continue
        doc=source.follow(ref);documents[ident]=doc
        occupied.update(native_id(guid) for guid in doc['exported_instance_guids'])
        pending.extend(references(selected(doc)['fields']))
    directory=root/'deke-trial/panic-checking-fragments';directory.mkdir(exist_ok=True)
    fragments=[]
    for ident,doc in sorted(documents.items()):
        ids={offset:native_id(guid) for guid,offset in doc['exported_instance_guids'].items()}
        fragment=compile_tree(doc,{'internal_offset':doc['selected_offset']},occupied,ids)
        data=fragment.pop('data');path=directory/f'{ident:016x}.bin';path.write_bytes(data)
        fragment.update(path=str(path),sha256=hashlib.sha256(data).hexdigest())
        name=selected(doc)['fields']['Name']
        if ident==native_id(checking['RallyShoulderCheckRequestState']['external']['instance']):
            fragment['compatibility_requirements'].append({'kind':'checking_rally_producer_required','source_id':hex(ident)})
        if ident==native_id(checking['IsInaFaildedShoulderCheckNode']['external']['instance']):
            fragment['compatibility_requirements'].append({'kind':'checking_failed_node_producer_required','source_id':hex(ident)})
        fragments.append({'name':name,'fragment':fragment})
    available=native|{o['id'] for r in fragments for o in r['fragment']['objects']}
    missing=[r for row in fragments for r in row['fragment']['symbolic_relocations'] if r['target_id'] not in available]
    if missing:raise ValueError(f'Unresolved checking asset imports: {missing}')
    report={'installed':False,'status':'staged_producer_integration_required','fragments':fragments,
            'source_evidence':source.evidence,'unresolved_imports':missing}
    (directory/'report.json').write_text(json.dumps(report,indent=2))
    return report

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('root',type=Path)
    result=run(parser.parse_args().root)
    print(json.dumps({'status':result['status'],'fragments':len(result['fragments'])}))
