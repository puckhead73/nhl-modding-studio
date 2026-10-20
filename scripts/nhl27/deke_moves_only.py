"""Integrate NHL27 move assets through NHL24's native gameplay controllers.

Imported move selection uses isolated context properties populated by the
move-specific adapter. NHL24 retains gesture recognition and gameplay state.
"""
import argparse
import copy
import hashlib
import json
import struct
from collections import deque
from pathlib import Path

from ant_link import link
from native_ant import Bank

SEEDS = ('seq_dk18_toedrag_left', 'pcdb_toedrag', 'pcdb_psk_lpd_bank', 'cho_glidedekespbs')
CHOOSER = 7751865479019598222
POLICY = 7972420970037982113
NODE = 17404573438163858817
ANALOG_ENUM = 0xcb705f7ab922159e
LEFT_STICK = 0x5ba41c0da134cb88
LEFT = 0xd5c8a70e3b28db8f
RIGHT = 0xa06b7d3e4b1c0b02


def read(path):
    return json.loads(path.read_text())


def checked_payload(fragment):
    if 'data' in fragment:
        return fragment
    data = Path(fragment['path']).read_bytes()
    if hashlib.sha256(data).hexdigest() != fragment['sha256']:
        raise ValueError('Staged fragment hash mismatch')
    return dict(fragment, data=data)


def allocated_id(label):
    return int.from_bytes(hashlib.sha256(('nhl27-moves-only/' + label).encode()).digest()[:8], 'little')


# These properties belong only to the imported move selectors. Never publish
# NHL27-only enum values into NHL24's gameplay RI or move owner.
CONTEXT_IDS = {
    0xaaa6c8084b72422e: allocated_id('context/high-level'),
    0x386e433016486095: allocated_id('context/toe-secondary'),
    0x2f0fb778b6452f99: allocated_id('context/toe-type'),
    0x346e8e12411acf19: allocated_id('context/bank-type'),
}
VELOCITY_IDS = [allocated_id('context/bank-initial-velocity-'+axis) for axis in 'xyz']
# Board reversal situations share the complete existing net-reversal sequence.
# The runtime selects native gameplay types 5/6 while preserving private 7/8
# for the board rebound. No private turn-track or velocity-stop edits apply.
# Database assets are ordered by column data, not by their row number: FH is
# bank value 8 and BH is 7. Values 5/6 already use these compound sequences.
BOARD_TURN_ROUTES = {
    0x6b70911b40c2a970: 0x75122e5b19cc2e62,
    0x1d99815707bf3f70: 0xe1b48b0674842d66,
}


def context_properties(bank, enum_id):
    fragments = []
    for original, ident in CONTEXT_IDS.items():
        obj = bank.by_id[original]; at = obj['offset']
        if (obj['size'], obj['type_hash']) != (104, 0x2196d161):
            raise ValueError('Unqualified native enumeration property layout')
        data = bank.blob[at:at+104]
        inherited = [dict(site=s-at, source_site=s) for s in bank.reloc
                     if at <= s < at+104 and not (original == 0xaaa6c8084b72422e and s-at == 96)]
        fragment = dict(data=data, objects=[dict(id=ident, offset=0, size=104,
            type_hash=obj['type_hash'], name='NHL27_MoveContext_'+obj['name'])],
            template_bank_sha256=hashlib.sha256(bank.data).hexdigest(),
            inherited_relocations=inherited, symbolic_relocations=[])
        if original == 0xaaa6c8084b72422e:
            fragment['symbolic_relocations'].append(dict(site=96, target_id=enum_id))
        fragments.append(fragment)
    return fragments


def isolate_context(groups, bank):
    # Clone the source high-level enum, including its children. Existing NHL24
    # enum identities and all their consumers remain untouched.
    enum_group = groups['af/assets/nhl/enum_highlevellpdtype'][0]
    remap = dict(CONTEXT_IDS)
    remap.update({o['id']: allocated_id('context/enum/'+str(o['id']))
                  for o in enum_group['objects']})
    for fragments in groups.values():
        for f in fragments:
            for obj in f['objects']:
                obj['id'] = remap.get(obj['id'], obj['id'])
            if 'root_id' in f:
                f['root_id'] = remap.get(f['root_id'], f['root_id'])
            for ref in f.get('symbolic_relocations', []):
                ref['target_id'] = remap.get(ref['target_id'], ref['target_id'])
    for f in context_properties(bank, remap[0xae0ad0ebd4489066]):
        groups['context/'+str(f['objects'][0]['id'])] = [f]
    # Per-actor scratch words use the already qualified native property layout.
    # The runtime copies IEEE velocity bits here; no enum consumer reads them.
    template=context_properties(bank,remap[0xae0ad0ebd4489066])[-1]
    for axis,ident in zip('xyz',VELOCITY_IDS):
        f=copy.deepcopy(template)
        f['objects'][0].update(id=ident,name='NHL27_BankInitialVelocity_'+axis)
        groups['context/'+str(ident)]=[f]
    return remap


def node_fragment(bank):
    """Retain native triggers, transitions, lifetime and initialization."""
    obj = bank.by_id[NODE]
    if (obj['size'], obj['type_hash']) != (208, 0x184a86dd):
        raise ValueError('Native glide node layout changed')
    at = obj['offset']
    for field in (136, 144):
        if bank.pointer(at + field) != bank.by_id[0x444a02c81faeacb1]['offset']:
            raise ValueError('Native glide subject changed')
    inherited = [dict(site=s-at, source_site=s) for s in bank.reloc
                 if at <= s < at+208 and s-at not in (136, 144)]
    if any(at <= row[1] < at+208 for row in bank.external):
        raise ValueError('Unexpected external glide node fixup')
    return dict(data=bank.blob[at:at+208], objects=[dict(id=NODE, offset=0,
        size=208, type_hash=obj['type_hash'], name=obj['name'])],
        template_bank_sha256=hashlib.sha256(bank.data).hexdigest(),
        inherited_relocations=inherited,
        symbolic_relocations=[dict(site=s, target_id=CHOOSER) for s in (136, 144)])


def run(root):
    trial = root/'deke-trial'
    output = trial/'moves-only'
    output.mkdir(parents=True, exist_ok=True)
    native = {n: Bank((root/'ant/nhl24_160'/f'{n}.bin').read_bytes()) for n in ('launch', 'state')}
    expanded = {n: read(trial/f'{n}-expanded-report.json') for n in native}
    bases = {}
    for n,r in expanded.items():
        data = Path(r['path']).read_bytes()
        if hashlib.sha256(data).hexdigest()!=r['sha256']:
            raise ValueError('Expanded animation bank changed')
        bases[n] = Bank(data)
    existing = {ident for b in native.values() for ident in b.by_id}
    groups = {}
    for row in read(trial/'controller-fragments/report.json')['partitions']:
        if 'fragment' in row:
            f = row['fragment']
            groups[row['name']] = [copy.deepcopy(f)]
    context_remap = isolate_context(groups, bases['launch'])
    routed=[]
    for f in groups['af/assets/nhl/pcdb_psk_lpd_bank']:
        for ref in f.get('symbolic_relocations',[]):
            if ref['target_id'] in BOARD_TURN_ROUTES:
                routed.append(ref['target_id'])
                ref['target_id']=BOARD_TURN_ROUTES[ref['target_id']]
    if sorted(routed)!=sorted(BOARD_TURN_ROUTES):
        raise ValueError('Board reversal row identities changed')
    tagged = read(trial/'closure-stage/new-moves/tagged-fragments/report.json')['clips']
    for row in tagged:
        if 'fragments' in row:
            groups[row['name']] = row['fragments']
    moves = read(trial/'closure-stage/new-moves/report.json')['clips']
    for row in moves:
        if 'new_binding' in row:
            b = row['new_binding']; ident = int(row['target_ctd_id'], 16)
            groups[f'channel-map/{ident:x}'] = [dict(path=b['path'], sha256=b['sha256'],
                objects=[dict(id=ident, offset=0, size=b['object_size'], type_hash=b['type_hash'])],
                internal_relocations=[b['relocation']], symbolic_relocations=[])]
    turns=read(output/'turn-audit/fragments.json')
    turn_remap={int(k):v for k,v in turns['remap'].items()}
    for fs in groups.values():
        for f in fs:
            for ref in f.get('symbolic_relocations',[]):
                ref['target_id']=turn_remap.get(ref['target_id'],ref['target_id'])
    groups['move-specific-reverse-turns']=turns['fragments']
    from deke_pivot import adapt
    pivot = adapt(groups, output, allocated_id)
    providers = {}
    for name, fs in groups.items():
        for f in fs:
            for obj in f['objects']:
                if obj['id'] in providers:
                    raise ValueError('Ambiguous staged provider')
                providers[obj['id']] = name
    selected = set()
    queue = deque('af/assets/nhl/' + n for n in SEEDS)
    # POLICY is new in NHL24; force inclusion if a future bank contains that ID.
    queue.append(providers[POLICY])
    queue.extend(providers[i] for i in VELOCITY_IDS)
    while queue:
        name = queue.popleft()
        if name in selected:
            continue
        selected.add(name)
        for f in groups[name]:
            if f.get('compatibility_requirements') or f.get('unresolved_dependencies'):
                raise ValueError(f'Unqualified move dependency: {name}')
            for ref in f.get('symbolic_relocations', []):
                if ref['target_id'] not in existing:
                    queue.append(providers[ref['target_id']])
    expected = {r['name'] for r in read(trial/'new-moves/report.json')['clips']
                if '/ptm' not in r['name']}
    # User replaced these open-ice clips with the proven full net turn.
    superseded={'af/assets/dk25_betweenlegs_bh_reverse_nopickup',
                'af/assets/dk25_betweenlegs_fh_reverse_nopickup'}
    expected-=superseded
    if not expected <= selected:
        raise ValueError(f'Unreachable move clips: {sorted(expected-selected)}')
    if any('panic' in n or '/ptm' in n for n in selected):
        raise ValueError('Broader runtime graph leaked into moves-only closure')
    fragments = [checked_payload(f) for n in sorted(selected) for f in groups[n]]
    definitions = {o['id'] for f in fragments for o in f['objects']}
    # Keep existing state identities in their own bank; no dependency on loading
    # a level-specific state bank before the launch bank is introduced.
    state_replacements = definitions & set(native['state'].by_id)
    if state_replacements:
        raise ValueError(f'State replacements need explicit split: {state_replacements}')
    reports = {}
    for name in ('launch', 'state'):
        previous = read(trial/f'{name}-expanded-report.json')
        data = Path(previous['path']).read_bytes()
        if hashlib.sha256(data).hexdigest() != previous['sha256']:
            raise ValueError('Expanded animation bank changed')
        base = Bank(data)
        if name == 'launch':
            fragments.append(node_fragment(base))
            definitions.add(NODE)
            # NHL24's registration path calls strlen on memory-image names.
            # Anonymous EBX children still need a non-null native debug name;
            # the memory-image sentinel is parseable but crashes this loader.
            for fragment in fragments:
                for obj in fragment['objects']:
                    if obj.get('name') is None:
                        obj['name'] = f'NHL27_Move_Internal_{obj["id"]:016x}'
            # Only native, already registered layouts are accepted by the linker.
            data = link(base, fragments, replace_ids=definitions & set(base.by_id),
                registered_types=set(zip(native['state'].hashes, native['state'].sizes)))
        path = output/f'{name}.bank'; path.write_bytes(data)
        report = dict(previous, path=str(path.resolve()), sha256=hashlib.sha256(data).hexdigest())
        report['new_move_clips'] = len(expected) if name == 'launch' else 0
        (output/f'{name}-expanded-report.json').write_text(json.dumps(report, indent=2))
        reports[name] = report
    linked = Bank((output/'launch.bank').read_bytes())
    # Verify actual emitted clip bytes against audited conversions.
    for move in moves:
        if move['name'] in expected:
            clip_id = next(o['id'] for f in groups[move['name']] for o in f['objects']
                           if o['type_hash'] == 0x5cc5ce9d)
            if linked.clip(clip_id)[2] != Path(move['path']).read_bytes():
                raise ValueError(f'Linked clip mismatch: {move["name"]}')
    receipt = dict(scope='NHL27 moves within NHL24 gameplay', installed=False,
        gameplay_tested=False, executable_patches=[], new_move_clips=sorted(expected),
        superseded_clips=sorted(superseded),
        controller_groups=sorted(selected), native_replacements=[hex(i) for i in sorted(definitions & set(native['launch'].by_id))],
        adapters=['Isolated move context properties: source toe/bank databases and glide chooser',
                  'Native glide node: subject pointers only; triggers and transitions retained'],
        context_properties={hex(k):hex(v) for k,v in CONTEXT_IDS.items()},
        velocity_properties=[hex(i) for i in VELOCITY_IDS],
        private_turn_clips={hex(k):hex(v) for k,v in turn_remap.items()},
        board_turn_routes={hex(k):hex(v) for k,v in BOARD_TURN_ROUTES.items()},
        stationary_pivot=pivot,
        runtime_required='deke_skill_stick',
        excluded='Panic/PTM reactions, source input runtime, source eligibility and interruption systems',
        banks={n: dict(path=r['path'], sha256=r['sha256']) for n,r in reports.items()})
    (output/'integration-receipt.json').write_text(json.dumps(receipt, indent=2))
    print(json.dumps(dict(new_clips=len(expected), groups=len(selected), replacements=receipt['native_replacements'])))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path)
    run(parser.parse_args().root)
