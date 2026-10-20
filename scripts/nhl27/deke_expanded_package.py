"""Package expanded private banks; verify both resource metadata and CAS round trips."""
import hashlib
import json
import shutil
import struct
import sys
from pathlib import Path
from asset_manifest import manifest
from bundle_inventory import bundles
from compatibility import Install,ps4_spans,frames,Oodle
from deke_trial_package import stored_frames
from native_ant import Bank


def run(root, moves_only=False):
    d=root/'deke-trial'
    if moves_only:d=d/'moves-only'
    package=d/('package' if moves_only else 'expanded-trial');overlay=package/'game-mods'
    ins=Install('nhl24_160');oodle=Oodle();copied={};receipts=[]
    def cas_path(span):
        if span['layer']!=1:raise ValueError('Expanded package requires patch CAS spans')
        return Path(f"Patch/Gen4b/superbundlelayout/nhl_installpackage_{span['catalog']:02d}/cas_{span['cas']:02d}.cas")
    def append(span,payload):
        rel=cas_path(span);out=overlay/rel
        if rel not in copied:
            out.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(ins.root/rel,out);copied[rel]=out
        at=out.stat().st_size
        if at+len(payload)>0xffffffff:raise ValueError('CAS 32-bit offset overflow')
        with out.open('ab') as f:f.write(payload)
        return at,len(payload)
    for name,relative,bundle_name in [('state','Patch/Gen4b/levels/game/game.toc','gen4b/levels/game/game'),
        ('launch','Patch/Gen4b/contentlaunchsb.toc','gen4b/content/common/configs/bundles/launch_sba')]:
        report=json.loads((d/f'{name}-expanded-report.json').read_text());data=Path(report['path']).read_bytes()
        if hashlib.sha256(data).hexdigest()!=report['sha256']:raise ValueError('Candidate changed')
        bank=Bank(data);rows,_=bundles(ins.root/relative);toc=bytearray((ins.root/relative).read_bytes())
        wrapper=0x22c if toc[:4]==bytes.fromhex('00d1ce01') else 0
        row=next(x for x in rows if x['name']==bundle_name);start=wrapper+row['record_offset']
        record=bytearray(toc[start:start+row['record_size']]);spans=ps4_spans(record)
        ma=bytearray(ins.read(spans[0]));assets=manifest(ma)
        candidates=[i for i,a in enumerate(assets) if a['kind']=='res' and a['name'].endswith('_antstate')]
        if len(candidates)!=1:raise ValueError('Ambiguous ANT resource')
        index=candidates[0];asset=assets[index];stored=ins.read(spans[index+1])
        if hashlib.sha1(stored).hexdigest()!=asset['sha1']:raise ValueError('Original resource hash mismatch')
        original=frames(stored,oodle)[1]
        if hashlib.sha256(original).hexdigest()!=report['original_sha256']:raise ValueError('Original decoded bank mismatch')
        old=Bank(original);meta=struct.unpack('<4I',bytes.fromhex(asset['resource_meta']))
        if meta[:3]!=(old.header,old.sections['blob'][0]-old.header,len(old.blob)):
            raise ValueError('Resource metadata allocation fields differ from bank layout')
        payload=stored_frames(data);resource_at,resource_size=append(spans[index+1],payload)
        # Manifest SHA, decoded length, and ANT memory-image allocation sizes.
        magic,total,ne,nr=struct.unpack_from('<4I',ma,4);table=4+32+20*total;res=table+8*ne
        meta_at=res+8*nr+4*nr
        ma[4+32+20*index:4+32+20*(index+1)]=hashlib.sha1(payload).digest()
        struct.pack_into('<I',ma,table+index*8+4,len(data))
        struct.pack_into('<4I',ma,meta_at+16*(index-ne),bank.header,bank.sections['blob'][0]-bank.header,len(bank.blob),meta[3])
        actual=manifest(ma);expected=[dict(x) for x in assets]
        expected[index].update(sha1=hashlib.sha1(payload).hexdigest(),size=len(data),resource_meta=struct.pack('<4I',bank.header,bank.sections['blob'][0]-bank.header,len(bank.blob),meta[3]).hex())
        if actual!=expected:raise ValueError('Unexpected manifest mutation')
        manifest_at,manifest_size=append(spans[0],ma)
        count=len(spans);p=32;positions=[]
        for flag in record[-count:]:
            if flag==1:p+=4
            elif flag!=0:raise ValueError('Unknown TOC flag')
            positions.append(p);p+=8
        struct.pack_into('>II',record,positions[0],manifest_at,manifest_size)
        struct.pack_into('>II',record,positions[index+1],resource_at,resource_size)
        expected_spans=[dict(x) for x in spans];expected_spans[0].update(offset=manifest_at,size=manifest_size)
        expected_spans[index+1].update(offset=resource_at,size=resource_size)
        if ps4_spans(record)!=expected_spans:raise ValueError('Unexpected TOC mutation')
        toc[start:start+len(record)]=record;out=overlay/relative;out.parent.mkdir(parents=True,exist_ok=True);out.write_bytes(toc)
        # Read emitted CAS files back from disk before reporting a usable package.
        with (overlay/cas_path(spans[index+1])).open('rb') as f:f.seek(resource_at);actual_payload=f.read(resource_size)
        with (overlay/cas_path(spans[0])).open('rb') as f:f.seek(manifest_at);actual_manifest=f.read(manifest_size)
        if frames(actual_payload)[1]!=data or actual_manifest!=ma:raise ValueError('Emitted CAS round trip failed')
        receipts.append(dict(bank=name,dct=report['dct'],vbr=report['vbr'],bank_sha256=report['sha256'],toc=relative,
                             toc_sha256=hashlib.sha256(toc).hexdigest(),resource_sha1=hashlib.sha1(payload).hexdigest()))
    files=[]
    for rel,out in copied.items():
        with out.open('rb') as f:digest=hashlib.file_digest(f,'sha256').hexdigest()
        files.append(dict(path=rel.as_posix(),size=out.stat().st_size,sha256=digest))
    receipt=dict(status='animation_metadata_trial_gameplay_unverified',banks=receipts,cas=files,
        limitations=['Unmatched source clip roots and controller graph/executable differences are not ported',
                     'No NHL27 gameplay-parity claim'])
    if moves_only:
        integration=json.loads((d/'integration-receipt.json').read_text())
        if any(integration['banks'][r['bank']]['sha256']!=r['bank_sha256'] for r in receipts):
            raise ValueError('Package differs from integrated move banks')
        runtime=integration['runtime']
        executable=Path(runtime['path']);runtime_receipt=Path(runtime['receipt_path'])
        if hashlib.sha256(executable.read_bytes()).hexdigest()!=runtime['sha256'] or hashlib.sha256(runtime_receipt.read_bytes()).hexdigest()!=runtime['receipt_sha256']:
            raise ValueError('Move-specific runtime changed after integration')
        shutil.copyfile(executable,overlay/'eboot.bin')
        shutil.copyfile(runtime_receipt,package/'runtime-receipt.json')
        receipt.update(status='moves_only_package_verified_gameplay_untested',
            integration_receipt_sha256=hashlib.sha256((d/'integration-receipt.json').read_bytes()).hexdigest(),
            new_move_clips=len(integration['new_move_clips']),
            executable=dict(path='eboot.bin',sha256=runtime['sha256']),
            executable_patches=integration['executable_patches'],
            runtime_receipt_sha256=runtime['receipt_sha256'],
            limitations=['Gameplay testing assigned to user',
                         'Uses NHL24 gameplay; broader NHL27 panic/input/PTM runtime remains separate'])
    (package/'package-receipt.json').write_text(json.dumps(receipt,indent=2));print(json.dumps(receipt,indent=2))


if __name__=='__main__':run(Path(sys.argv[1]), moves_only='--moves-only' in sys.argv[2:])
