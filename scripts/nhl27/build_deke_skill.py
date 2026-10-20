"""Build the moves-only selection/release adapters, preserving original payloads."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess

from build_input_eboot import reconstruct, align
from deke_runtime_evidence import checked_image, TARGET_HASH
from research import SelfImage, ElfImage
import deke_skill_hooks as hooks


def payload(path):
    data=path.read_bytes();h=struct.unpack_from('<16sHHIQQQIHHHHHH',data)
    sections=[struct.unpack_from('<IIQQQQIIQQ',data,h[6]+64*i) for i in range(h[12])]
    allocated=[s for s in sections if s[2]&2 and s[5]]
    if len(allocated)!=1 or allocated[0][1]!=1 or allocated[0][2]&1:
        raise ValueError('Move adapter must contain only read-only code/constants')
    symbols={}
    for s in sections:
        if s[1]!=2:continue
        strings=sections[s[6]];names=data[strings[4]:strings[4]+strings[5]]
        for at in range(s[4],s[4]+s[5],24):
            name,info,other,index,value,size=struct.unpack_from('<IBBHQQ',data,at)
            name=names[name:names.index(0,name)].decode()
            if name and index==0:raise ValueError('Unresolved move adapter symbol: '+name)
            if name:symbols[name]=value
    s=allocated[0]
    return s[3],data[s[4]:s[4]+s[5]],symbols


def run(target,output,compiler):
    image=checked_image(SelfImage,target,TARGET_HASH)
    output=output.resolve();output.mkdir(parents=True,exist_ok=True)
    sources=Path(__file__).resolve().parent/'runtime'
    objects=[]
    for name in ('deke_skill_stick','deke_context_binding'):
        obj=output/(name+'.o');objects.append(str(obj))
        subprocess.run([str(compiler),'--target=x86_64-unknown-freebsd','-O2',
            '-ffreestanding','-fno-stack-protector','-fPIC','-mno-red-zone','-mavx',
            '-ffp-contract=off','-fvisibility=hidden','-c',str(sources/(name+'.c')),
            '-o',str(obj)],check=True,capture_output=True)
    asm=output/'deke-skill-hooks.s';asm.write_text(hooks.assembly())
    obj=asm.with_suffix('.o');objects.append(str(obj))
    subprocess.run([str(compiler),'--target=x86_64-unknown-freebsd','-c',str(asm),
                    '-o',str(obj)],check=True,capture_output=True)
    address=align(max(p['va']+align(p['memsz'],p['align'] or 1)
                      for p in image.ph if p['type'] in (1,0x61000010)))
    script=output/'deke-skill.ld'
    script.write_text('SECTIONS { .port 0x%x : { *(.text*) *(.rodata*) } '
        '/DISCARD/ : { *(.eh_frame*) *(.comment) *(.note*) } }\n'%address)
    linked=output/'deke-skill.elf'
    result=subprocess.run([str(compiler.parent/'ld.lld.exe'),'-m','elf_x86_64',
        '--no-undefined','-T',str(script),'-o',str(linked)]+objects,capture_output=True,text=True)
    if result.returncode:raise RuntimeError(result.stdout+result.stderr)
    va,code,symbols=payload(linked)
    if va!=address:raise ValueError('Payload address changed')
    original=reconstruct(image);candidate=bytearray(original);patches=[]
    for va,before,after in hooks.patches(image,symbols):
        segment=next(s for s in image.segments if s['va']<=va<s['va']+s['filesz'])
        offset=segment['offset']+va-segment['va']
        if candidate[offset:offset+len(before)]!=before:raise ValueError('Reconstruction mismatch')
        candidate[offset:offset+len(before)]=after
        patches.append(dict(va=hex(va),offset=offset,before=before.hex(),after=after.hex()))
    empty=image.ph[11]
    if empty['type']!=4 or any(empty[k] for k in ('filesz','memsz','offset','va')):
        raise ValueError('Expected unused PT_NOTE')
    offset=align(len(candidate));candidate+=bytes(offset-len(candidate))+code
    ph=image.header[5]+11*56
    struct.pack_into('<IIQQQQQQ',candidate,ph,1,5,offset,address,address,len(code),len(code),0x4000)
    restored=bytearray(candidate[:len(original)]);restored[ph:ph+56]=original[ph:ph+56]
    for p in patches:restored[p['offset']:p['offset']+len(bytes.fromhex(p['before']))]=bytes.fromhex(p['before'])
    if restored!=original:raise ValueError('Undeclared original payload change')
    path=output/'eboot.bin';path.write_bytes(candidate)
    reread=ElfImage(path)
    if reread.read(address,len(code))!=code:raise ValueError('Adapter round trip failed')
    report=dict(scope='Skill Stick selectors, net reversal with board rebound or soft drop, handed board pickup query, and NHL27 toe curve',source_sha256=TARGET_HASH,
        path=str(path),sha256=hashlib.sha256(candidate).hexdigest(),patches=patches,
        payload_va=hex(address),payload_bytes=len(code),
        symbols={k:hex(v) for k,v in symbols.items() if k.startswith('deke_')},
        original_payload_check='passed',input_recognizers_changed=False,
        native_gameplay_enum_values_changed=True,gameplay_tested=False)
    report['situational_net_reversal']=dict(native_supported_type_mapping={'1/private7':6,'2/private8':5,'3/private9':5,'4/private10':6},
        private_board_context_preserved=True,pose_writes=False,artificial_entry_stop=False)
    report['pickup_query']=dict(site=hex(hooks.PICKUP_SITE),mapping={'6/private7':1,'5/private8':2},
        scope='Local pickup-filter selector only; native gameplay word and movement remain unchanged')
    report['pickup_wait']=dict(site=hex(hooks.PICKUP_WAIT_SITE),maximum_ticks_by_variant={'7':6,'8':6,'9':6,'10':6},
        variants=[7,8,9,10],scope='Cap local general-pickup fallback wait; preserve native eligibility and shorter waits')
    report['soft_drop']=dict(target_behind_metres=.90,travel_ticks=12,post_clamp_multiplier=1.5,min_speed_per_tick=.1125,max_speed_per_tick=.165,
        incoming_velocity_inherited=False,vertical_velocity=0)
    (output/'runtime-receipt.json').write_text(json.dumps(report,indent=2))
    integration_path=output.parent/'integration-receipt.json'
    integration=json.loads(integration_path.read_text())
    from deke_moves_only import CONTEXT_IDS, VELOCITY_IDS
    if integration['context_properties']!={hex(k):hex(v) for k,v in CONTEXT_IDS.items()}:
        raise ValueError('Move context IDs differ from the runtime build')
    source=(sources/'deke_skill_stick.c').read_text()
    if any(hex(v)+'ULL' not in source for v in CONTEXT_IDS.values()):
        raise ValueError('Runtime binding IDs differ from the asset build')
    if integration['velocity_properties'] != [hex(i) for i in VELOCITY_IDS] or any(
            hex(v)+'ULL' not in source for v in VELOCITY_IDS):
        raise ValueError('Bank velocity storage differs from the asset build')
    integration['executable_patches']=patches
    integration['runtime']=dict(path=str(path),sha256=report['sha256'],
        receipt_path=str(output/'runtime-receipt.json'),
        receipt_sha256=hashlib.sha256((output/'runtime-receipt.json').read_bytes()).hexdigest())
    integration_path.write_text(json.dumps(integration,indent=2))
    print(json.dumps({k:report[k] for k in ('path','sha256','payload_bytes','original_payload_check')}))


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--target',type=Path,default=Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin'))
    p.add_argument('--output',type=Path,default=Path('local/nhl27-eboot/compatibility/deke-trial/moves-only/runtime'))
    p.add_argument('--compiler',type=Path,default=Path(r'C:\Program Files\LLVM\bin\clang.exe'))
    a=p.parse_args();run(a.target,a.output,a.compiler)
