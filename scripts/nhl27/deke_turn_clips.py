"""Qualify and privately stage the two NHL27 turns used by reverse board banks."""
import json,sys,struct,hashlib
from pathlib import Path
sys.path.insert(0,'scripts/nhl27')
from native_ant import Bank
from deke_native_audit import native_id
from compatibility import Install,Oodle,frames
from animation_fields import Fields
from animation_dct import parse,decode
from dct_reorder import permutation,reorder
from deke_closure_stage import unique_resource
from deke_expanded_bank import SCALARS
from deke_new_moves import native_clip_fragment
from ant_tags import compile_tree
from deke_moves_only import allocated_id
p=Path('local/nhl27-eboot/compatibility');t=p/'deke-trial';out=t/'moves-only/turn-audit';out.mkdir(exist_ok=True)
catalog=json.loads((t/'source-catalog.json').read_text());types=json.loads((p/'source-type-descriptors.json').read_text())
b=Bank((p/'ant/nhl24_160/launch.bin').read_bytes());inst=Install('nhl27');dec=Oodle();resources=[]
for path in p.glob('nhl27-*.toc.json'):
 resources.extend(a for bundle in json.loads(path.read_text())['bundles'] for a in bundle['assets'] if a['kind']=='res')
def read(a):
 raw=inst.read(a['span']);assert hashlib.sha1(raw).hexdigest()==a['sha1'];data=frames(raw,dec)[1];assert len(data)==a['size'];return data
def export(part):
 rows={n['asset']['sha1']:n for n in catalog if n['fixup']['partition']==part};assert len(rows)==1
 n=next(iter(rows.values()));return n,Fields(read(n['asset']),types).export()
report=[];fragments=[];remap={};occupied=set(b.by_id)
for ident in [0xb6da816916b1a686,0x81d6fd143908c2c4]:
 matches=[n for n in catalog if native_id(n['fixup']['partition'])==ident];assert matches
 n,f=export(matches[0]['fixup']['partition']);af=next(i['fields'] for i in f['instances'] if 'DctAnimationAsset'in i['type']);cf=next(i['fields'] for i in f['instances'] if 'ClipControllerAsset'in i['type'])
 data=read(unique_resource(resources,n['name'],af['Resource']));_,ctd=export(af['ChannelToDofAsset']['external']['partition']);src=next(i['fields']['0xec5cc5cd'] for i in ctd['instances'] if '0xec5cc5cd'in i['fields'])
 obj,anim,native=b.clip(ident);at=b.pointer(anim['offset']+40);count=struct.unpack_from('<I',b.blob,at+24)[0];dst=list(struct.unpack_from('<'+'I'*count,b.blob,b.pointer(at+16)))
 meta,_=parse(data);converted=reorder(data,permutation(src,dst,*(meta[k] for k in ['quaternions','vectors','float_vectors'])))
 a=decode(converted)[1];z=decode(native)[1]
 row=dict(name=n['name'],id=hex(ident),sample_equal=a==z,controller_equal=struct.pack('<4f6B',*(cf[k] for k in SCALARS))==b.blob[obj['offset']+48:obj['offset']+70],metadata_equal=struct.pack('<ffH',af['TrimOffset'],af['TrimmedDuration'],af['EndFrame'])==b.blob[anim['offset']+24:anim['offset']+34]);report.append(row)
 (out/(hex(ident)+'.resource')).write_bytes(converted);(out/(hex(ident)+'.json')).write_text(json.dumps(f,indent=2))
 private=allocated_id('reverse-turn/'+hex(ident));remap[ident]=private
 assert private not in occupied;occupied.add(private)
 ctd_id=next(i for i,o in b.by_id.items() if o['offset']==at)
 dsl_id=native_id(af['DofSetList']['external']['instance'])
 payload,fragment=native_clip_fragment(converted,f,private,ctd_id,dsl_id)
 for obj in fragment['objects']:
  obj['id']=int(obj['id'],16) if obj['id'] else allocated_id('reverse-turn-animation/'+hex(ident))
  obj['name']='NHL27_ReverseTurn_'+hex(obj['id'])
 for ref in fragment['symbolic_relocations']:ref['target_id']=int(ref['target_id'],16)
 tree=compile_tree(f,cf['TagCollectionSetAsset'],occupied)
 assert not tree.get('compatibility_requirements')
 fragment['symbolic_relocations'].append(dict(site=16,target_id=tree['root_id']))
 fragment.pop('unresolved_dependencies');fragment.pop('qualified_as')
 for label,part,body in [('clip',fragment,payload),('tags',tree,tree.pop('data'))]:
  path=out/(hex(ident)+'.'+label+'.bin');path.write_bytes(body)
  part.update(path=str(path.resolve()),sha256=hashlib.sha256(body).hexdigest());fragments.append(part)
(out/'fragments.json').write_text(json.dumps(dict(remap={str(k):v for k,v in remap.items()},fragments=fragments),indent=2))
(out/'report.json').write_text(json.dumps(report,indent=2));print(json.dumps(report,indent=2))
