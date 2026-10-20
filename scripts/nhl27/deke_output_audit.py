"""Audit reused move-controller records against staged NHL27 native layouts.

Read-only: pointer addresses are normalized through the serialized relocation
graph. Differences are evidence for investigation, not permission to overwrite
shared gameplay assets. Arrays are compared only within mapped source regions.
"""
import json
from pathlib import Path
from collections import deque
from native_ant import Bank
from deke_moves_only import CONTEXT_IDS, SEEDS, allocated_id, BOARD_TURN_ROUTES


def compare(fragment, bank, remap):
    data = Path(fragment['path']).read_bytes()
    objects = fragment['objects']
    mapping = {o['offset']: bank.by_id[remap.get(o['id'],o['id'])]['offset'] for o in objects
               if remap.get(o['id'],o['id']) in bank.by_id}
    internal = fragment.get('internal_relocations', [])
    symbolic = fragment.get('symbolic_relocations', [])
    # Object/array starts delimit allocations in the emitted source fragment.
    starts = sorted({0, len(data)} | set(mapping) |
                    {r['destination'] for r in internal} |
                    {o['offset'] for o in objects})
    def native(site):
        at = max((s for s in starts if s <= site), default=0)
        return mapping.get(at, -10**12) + site-at
    for _ in range(len(internal)+1):
        changed = False
        for r in internal:
            at = native(r['site'])
            if at in bank.reloc and r['destination'] not in mapping:
                mapping[r['destination']] = bank.reloc[at]; changed = True
        if not changed: break
    pointers = {r['site'] for r in internal+symbolic}
    differences = []
    for o in objects:
        ident=remap.get(o['id'],o['id'])
        if ident not in bank.by_id: continue
        dst = bank.by_id[ident]
        if (o['size'], o['type_hash']) != (dst['size'], dst['type_hash']):
            differences.append(dict(id=hex(o['id']), kind='layout')); continue
        for off in range(0, o['size'], 4):
            site = o['offset']+off
            if any(p <= site < p+8 for p in pointers): continue
            a=data[site:site+4]; b=bank.blob[dst['offset']+off:dst['offset']+off+4]
            if a != b:
                differences.append(dict(id=hex(o['id']), name=dst['name'],
                    kind='scalar', field=off, source=a.hex(), installed=b.hex()))
    for r in symbolic:
        at=native(r['site']); target=remap.get(r['target_id'],r['target_id'])
        if at < 0 or target not in bank.by_id: continue
        if bank.reloc.get(at) != bank.by_id[target]['offset']:
            differences.append(dict(kind='reference', site=r['site'], target=hex(target)))
    # Serialized dynamic arrays live outside object records. Compare their
    # scalar payload too, stopping at the next emitted allocation boundary.
    for start,end in zip(starts,starts[1:]):
        if start not in mapping or any(o['offset']<=start<o['offset']+o['size'] for o in objects):
            continue
        for site in range(start,end):
            if any(p<=site<p+8 for p in pointers):continue
            at=native(site)
            if data[site:site+1]!=bank.blob[at:at+1]:
                differences.append(dict(kind='array-byte',site=site,source=data[site],installed=bank.blob[at]))
    return differences


def run(root):
    trial=root/'deke-trial'; out=trial/'moves-only'
    bank=Bank((out/'launch.bank').read_bytes())
    rows=json.loads((trial/'controller-fragments/report.json').read_text())['partitions']
    groups={r['name']:r['fragment'] for r in rows if 'fragment' in r}
    remap=dict(CONTEXT_IDS)
    remap.update(BOARD_TURN_ROUTES)
    pivot=json.loads((out/'drop-back-audit/report.json').read_text())
    routes={int(k):v for k,v in pivot['routes'].items()}
    turns=json.loads((out/'turn-audit/fragments.json').read_text())
    remap.update({int(k):v for k,v in turns['remap'].items()})
    remap.update({o['id']:allocated_id('context/enum/'+str(o['id']))
        for o in groups['af/assets/nhl/enum_highlevellpdtype']['objects']})
    providers={o['id']:n for n,f in groups.items() for o in f['objects']}
    queue=deque('af/assets/nhl/'+n for n in SEEDS); seen=set(); reports=[]
    while queue:
        name=queue.popleft()
        if name in seen:continue
        seen.add(name);f=groups[name]
        local=dict(remap)
        if name=='af/assets/nhl/pcdb_psk_lpd_bank':
            local.update({k:routes[v] for k,v in BOARD_TURN_ROUTES.items()})
            local.update({i:routes[i] for i in (0xcfbf592baad94152,0xdc7b10363575bca2)})
        reports.append(dict(name=name, requirements=f.get('compatibility_requirements',[]),
            differences=compare(f,bank,local)))
        for r in f.get('symbolic_relocations',[]):
            target=r['target_id']
            if name=='af/assets/nhl/pcdb_psk_lpd_bank':target=local.get(target,target)
            if target in providers:queue.append(providers[target])
    for sequence in pivot['sequences']:
        reports.append(dict(name='drop-back/'+hex(sequence['private']),requirements=[],
            differences=compare(sequence['compiled_fragment'],bank,{}),
            intentional_source_differences=compare(sequence['source_fragment'],bank,
                {int(k):v for k,v in sequence['audit_remap'].items()})))
    result=dict(scope='Controller records only; not runtime movement parity',groups=reports,
        stationary_pivot_adapters=pivot,
        intentional_reference_adapters={hex(k):hex(v) for k,v in BOARD_TURN_ROUTES.items()},
        adapter_reason='Board and open-ice reversals share the complete net controller; runtime chooses rebound or soft drop')
    (out/'output-controller-audit.json').write_text(json.dumps(result,indent=2))
    print(json.dumps([dict(name=r['name'],differences=len(r['differences']),
        requirements=len(r['requirements'])) for r in reports if r['differences'] or r['requirements']],indent=2))


if __name__=='__main__':
    import argparse
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('root',type=Path)
    run(p.parse_args().root)
