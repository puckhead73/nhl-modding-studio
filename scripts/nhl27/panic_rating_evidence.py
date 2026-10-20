"""Verify named Skater style-column translations against both private images."""
import json
from pathlib import Path
import sqlite3
from capstone import Cs, CS_ARCH_X86, CS_MODE_64
from capstone.x86 import X86_OP_MEM, X86_OP_IMM, X86_OP_REG
from deke_runtime_evidence import checked_image,SOURCE_HASH,TARGET_HASH
from research import ElfImage,SelfImage


def run():
    src=checked_image(ElfImage,Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin'),SOURCE_HASH)
    dst=checked_image(SelfImage,Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin'),TARGET_HASH)
    sd=sqlite3.connect('file:local/nhl27-eboot/index.sqlite?mode=ro',uri=True)
    td=sqlite3.connect('file:local/nhl24-eboot/update/index.sqlite?mode=ro',uri=True)
    md=Cs(CS_ARCH_X86,CS_MODE_64);md.detail=True
    rows=[]
    for name,source_id,column in [('GeneralSpineAngle',0x11d,0x2d18),
            ('GeneralStrideDistance',0x11f,0x2d78),('GeneralStrideHeight',0x121,0x2dd8),
            ('GeneralStrideGait',0x120,0x2da8)]:
        label='Skater_SkatingStyle_'+name
        sva=sd.execute('select va from strings where text=?',(label,)).fetchone()[0]
        tva=td.execute('select va from strings where text=?',(label,)).fetchone()[0]
        refs=[a for a, in sd.execute('select src from refs where dst=?',(sva,))
              if 0x7c2eda0<=a<0x7c429a0]
        if len(refs)!=1:raise ValueError('Ambiguous named source column')
        source_code=list(md.disasm(src.read(refs[0],256),refs[0]))
        source_column=source_id*0x30+0x18
        stores=[i for i in source_code if i.mnemonic=='mov' and
            i.operands[0].type==X86_OP_MEM and i.operands[0].mem.disp==source_column]
        if len(stores)!=1:raise ValueError('Source named-column offset changed')
        tref=td.execute('select src from refs where dst=?',(tva,)).fetchone()[0]
        target_code=list(md.disasm(dst.read(tref,112),tref))
        hashes=[i.operands[1].imm for i in target_code if i.mnemonic=='mov' and
            i.operands[0].type==X86_OP_REG and md.reg_name(i.operands[0].reg)=='esi' and
            i.operands[1].type==X86_OP_IMM]
        if len(hashes)!=1:raise ValueError('Ambiguous native column hash')
        # Decode the generated constructor from its unwind entry, not raw hits.
        code=list(md.disasm(dst.read(0x8a29a20,0x8a76a60-0x8a29a20),0x8a29a20))
        found=[]
        for n,i in enumerate(code):
            if i.mnemonic!='mov' or len(i.operands)!=2:continue
            if (i.operands[0].type!=X86_OP_REG or md.reg_name(i.operands[0].reg)!='esi'
                or i.operands[1].type!=X86_OP_IMM or i.operands[1].imm!=hashes[0]):continue
            for store in code[n+1:n+45]:
                if (store.mnemonic=='mov' and store.operands[0].type==X86_OP_MEM
                    and store.operands[0].mem.disp==column):found.append((i,store))
        if len(found)!=1:raise ValueError('Native named-column offset changed')
        rows.append(dict(name=label,source_id=hex(source_id),source_column=hex(source_column),
            source_name_reference=hex(refs[0]),source_store=hex(stores[0].address),
            target_column=hex(column),target_hash=hex(hashes[0]),
            target_name_reference=hex(tref),target_hash_reference=hex(found[0][0].address),
            target_store=hex(found[0][1].address)))
    result=dict(source_sha256=SOURCE_HASH,target_sha256=TARGET_HASH,
        source_table='0x104db830',target_table='0xba1a060',columns=rows)
    out=Path('local/nhl27-eboot/compatibility/deke-trial/panic-style-bindings.json')
    out.write_text(json.dumps(result,indent=2)+'\n')
    return result


if __name__=='__main__':print(json.dumps(run(),indent=2))
