"""Package the fixed-size 1.60 motion trial into a private Frostbite file overlay.

Copies the affected CAS, replaces its manifest SHA-1, appends stored CAS frames,
and updates the one existing TOC span. Never writes into an installed game.
Usage: python scripts/nhl27/deke_trial_package.py COMPATIBILITY_ROOT
"""
import hashlib
import json
import shutil
import struct
import sys
from pathlib import Path
from asset_manifest import manifest
from bundle_inventory import bundles
from compatibility import Install, ps4_spans, frames


def stored_frames(data):
    out=bytearray()
    for offset in range(0,len(data),65536):
        block=data[offset:offset+65536];n=len(block)
        out+=struct.pack('>I',n)+bytes([0,0x70|(n>>16)])+struct.pack('>H',n&65535)+block
    return bytes(out)


def run(root):
    dest=root/'deke-trial';reports=json.loads((dest/'bank-trial-report.json').read_text())
    report=next(x for x in reports if x['build']=='nhl24_160')
    if not report['included'] or not report['unchanged_size']:raise ValueError('No fixed-size trial bank')
    bank=Path(report['output']).read_bytes()
    if hashlib.sha256(bank).hexdigest()!=report['output_sha256']:raise ValueError('Trial bank changed')
    original=(root/'ant/nhl24_160/state.bin').read_bytes()
    if hashlib.sha256(original).hexdigest()!=report['source_sha256']:raise ValueError('Original bank changed')
    ins=Install('nhl24_160');rel=Path('Patch/Gen4b/levels/game/game.toc');toc_path=ins.root/rel
    rows,info=bundles(toc_path);toc=bytearray(toc_path.read_bytes())
    wrapper=0x22c if toc[:4]==bytes.fromhex('00d1ce01') else 0
    row=next(x for x in rows if x['name']=='gen4b/levels/game/game')
    start=wrapper+row['record_offset'];record=toc[start:start+row['record_size']]
    spans=ps4_spans(record);ma=bytearray(ins.read(spans[0]));assets=manifest(ma)
    index=next(i for i,a in enumerate(assets) if a['kind']=='res' and a['name'].endswith('game_gen4b_antstate'))
    a=assets[index];span=spans[index+1];ms=spans[0]
    evidence=next(x for x in json.loads((root/'ant-evidence.json').read_text()) if x['build']=='nhl24_160')
    if a['sha1']!=evidence['stored_sha1'] or report['source_sha256']!=evidence['decoded_sha256']:
        raise ValueError('Installed resource is not the audited source bank')
    if any(ms[k]!=span[k] for k in ('layer','catalog','cas')) or span['layer']!=1:
        raise ValueError('Expected manifest and resource in the same patch CAS')
    if a['size']!=len(bank) or hashlib.sha1(ins.read(span)).hexdigest()!=a['sha1']:
        raise ValueError('Installed resource differs from inventory')
    payload=stored_frames(bank)
    if frames(payload)[1]!=bank:raise ValueError('Stored-frame round trip failed')
    sha=hashlib.sha1(payload).digest();ma[4+32+index*20:4+32+(index+1)*20]=sha
    changed=manifest(ma)
    if changed[index]['sha1']!=sha.hex():raise ValueError('Manifest rewrite failed')
    changed[index]['sha1']=a['sha1']
    if changed!=assets:raise ValueError('Unintended manifest edit')
    cas_rel=Path(f"Patch/Gen4b/superbundlelayout/nhl_installpackage_{span['catalog']:02d}/cas_{span['cas']:02d}.cas")
    cas_source=ins.root/cas_rel;append_at=cas_source.stat().st_size
    if append_at+len(payload)>0xffffffff:raise ValueError('CAS offset exceeds PS4 range')
    # Locate exactly the resource offset/size pair, retaining every identifier/flag.
    count=struct.unpack_from('>I',record,12)[0];flags=record[len(record)-count:];p=32;positions=[]
    for flag in flags:
        if flag==1:p+=4
        elif flag!=0:raise ValueError('Unsupported span flag')
        positions.append(p);p+=8
    struct.pack_into('>II',record,positions[index+1],append_at,len(payload))
    rewritten=ps4_spans(record)
    expected=[dict(x) for x in spans];expected[index+1].update(offset=append_at,size=len(payload))
    if rewritten!=expected:raise ValueError('TOC span rewrite failed')
    toc[start:start+len(record)]=record
    package=dest/'trial-package';overlay=package/'game-mods'
    cas_out=overlay/cas_rel;cas_out.parent.mkdir(parents=True,exist_ok=True)
    toc_out=overlay/rel;toc_out.parent.mkdir(parents=True,exist_ok=True)
    shutil.copyfile(cas_source,cas_out)
    with cas_out.open('r+b') as f:
        f.seek(ms['offset']);f.write(ma);f.seek(append_at);f.write(payload)
    toc_out.write_bytes(toc)
    # Re-read the emitted files, rather than relying on buffers used to write them.
    check_rows,_=bundles(toc_out)
    if check_rows!=rows:raise ValueError('TOC bundle directory changed')
    with cas_out.open('rb') as f:
        f.seek(ms['offset']);check=manifest(f.read(ms['size']))
        f.seek(append_at);stored=f.read(len(payload))
    if check[index]['sha1']!=hashlib.sha1(stored).hexdigest() or frames(stored)[1]!=bank:
        raise ValueError('Emitted package round trip failed')
    with cas_out.open('rb') as f:cas_digest=hashlib.file_digest(f,'sha256').hexdigest()
    receipt=dict(status='packaged_motion_only_runtime_unverified',included_clips=report['included'],
        overlay=str(overlay.resolve()),source_toc_sha256=info['sha256'],bank_sha256=report['output_sha256'],
        toc_sha256=hashlib.sha256(toc).hexdigest(),manifest_stored_sha1=sha.hex(),
        cas_source=str(cas_source),cas_source_size=append_at,cas_output_size=cas_out.stat().st_size,
        cas_output_sha256=cas_digest,
        resource_offset=append_at,resource_stored_size=len(payload),
        unchanged_game_executables=True,limitations=report['limitations'][1:])
    (package/'package-receipt.json').write_text(json.dumps(receipt,indent=2))
    print(json.dumps(receipt,indent=2))


if __name__=='__main__':run(Path(sys.argv[1]))
