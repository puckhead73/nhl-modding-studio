"""Share the complete net reversal with board and soft-drop situations."""
import json


def adapt(groups, output, allocated_id):
    # deke_moves_only already maps reverse board FH/BH to the net FH/BH roots.
    # Keep the same objects: animation, blends, warpers, events and recovery.
    identities = [0x75122e5b19cc2e62, 0xe1b48b0674842d66,
                  0xcfbf592baad94152, 0xdc7b10363575bca2]
    refs = [r['target_id'] for f in groups['af/assets/nhl/pcdb_psk_lpd_bank']
            for r in f['symbolic_relocations'] if r['target_id'] in identities]
    if refs != identities[:2] * 2 + identities[2:]:
        raise ValueError('Situational board/net reversal row mapping changed')
    routes=dict(zip(identities,identities[:2]*2))
    for f in groups['af/assets/nhl/pcdb_psk_lpd_bank']:
        for ref in f['symbolic_relocations']:
            ref['target_id']=routes.get(ref['target_id'],ref['target_id'])
    report = dict(reason='Full net reversal shared by board rebound and open-ice soft drop; native gameplay types 5/6',
                  gameplay_verified=False, clips=[], sequences=[], blends=[],
                  routes=routes,
                  board_gameplay_types={'7':6,'8':5},
                  open_ice_gameplay_types={'9':5,'10':6},
                  pose_velocity_override=False, animation_edits=False)
    directory = output / 'drop-back-audit'
    directory.mkdir(exist_ok=True)
    (directory / 'report.json').write_text(json.dumps(report, indent=2))
    return report
