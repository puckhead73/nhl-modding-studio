"""Export a disabled shad XML experiment after exact executable validation.

Never modifies the executable or installs a patch. Output creation is exclusive.
Usage: python scripts/nhl24/export_experiment.py EBOOT FEATURE OUTPUT.xml
"""
import argparse
import hashlib
import json
import xml.etree.ElementTree as ET
from pathlib import Path
from inspect_eboot import Image

MANIFEST = Path(__file__).with_name('patch_candidates.json')


def export(eboot, feature, output):
    im=Image(eboot)
    digest=hashlib.sha256(im.data).hexdigest()
    registry=json.loads(MANIFEST.read_text())
    build=next((b for b in registry['builds'] if b['sha256']==digest),None)
    if build is None:raise ValueError('Unknown or modified executable: exact SHA-256 match required')
    spec=next((p for p in build['experiments'] if p['id']==feature),None)
    if spec is None:raise ValueError('Experiment is unavailable for this build')
    if output.suffix.lower()!='.xml' or output.resolve()==Path(eboot).resolve():
        raise ValueError('Output must be a separate .xml file')
    root=ET.Element('Patch')
    root.append(ET.Comment('RESEARCH EXPERIMENT: disabled; game verification pending. '+spec['limitation']))
    meta=ET.SubElement(root,'Metadata',Name=spec['name'],isEnabled='false',AppVer=build['version'],AppElf='eboot.bin',Author='NHL Modding Studio research')
    lines=ET.SubElement(meta,'PatchList')
    ranges=[]
    for edit in spec['edits']:
        va=int(edit['va'],16);expected=bytes.fromhex(edit['expected']);replacement=bytes.fromhex(edit['replacement'])
        if not expected or len(expected)!=len(replacement):raise ValueError('Invalid edit width')
        if im.read(va,len(expected))!=expected:raise ValueError(f'Original bytes differ at {va:#x}')
        if any(va<b and a<va+len(expected) for a,b in ranges):raise ValueError('Overlapping edits')
        ranges.append((va,va+len(expected)))
        lines.append(ET.Comment(f'image VA {va:#x}; SELF offset {im.offset(va):#x}; original {expected.hex()}'))
        ET.SubElement(lines,'Line',Type='bytes',Address=f'0x{va+0x400000:08X}',Value=replacement.hex().upper())
    ET.indent(root)
    payload=ET.tostring(root,encoding='utf-8',xml_declaration=True)
    with output.open('xb') as f:f.write(payload)
    return {'build':build['version'],'sha256':digest,'feature':feature,'output':str(output.resolve()),'enabled':False,'status':spec['status']}


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('eboot',type=Path);p.add_argument('feature');p.add_argument('output',type=Path)
    a=p.parse_args()
    print(json.dumps(export(a.eboot,a.feature,a.output),indent=2))
