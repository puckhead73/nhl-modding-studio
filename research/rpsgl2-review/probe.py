"""Read-only Legacy PS3 sample probe. All exports go to a disposable directory."""
import sys,json,tempfile,hashlib
from pathlib import Path
import numpy as np
repo=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(repo/'RPSGL2.zip_pass_123/RPSGL2'))
from rpsgl import RPSGL,Mesh,export_obj,import_obj
from fbxio import export_fbx,import_fbx
root=Path(sys.argv[1])
paths=[root/'player'/f'player_{i}.rpsgl' for i in (66,67,71,76,80,88,89)]
for folder,prefix in [('skate','playerskate_'),('helmet','helmet_'),('glove','glove_'),('playerstick','playerstick_'),('trapper','trapper_'),('goaliehelmet','cage_'),('stadium','stadium_')]:
 candidates=sorted(p for p in (root/folder).glob(prefix+'*.rpsgl') if not p.stem.endswith(('_cm','_nm','_am','_tm','_sm','_dm','_lm')))
 if candidates: paths.append(candidates[0])
results=[]
with tempfile.TemporaryDirectory() as tmp:
 tmp=Path(tmp)
 for path in paths:
  row={'file':path.relative_to(root).as_posix()}; results.append(row)
  try:
   original=path.read_bytes(); row['sha256']=hashlib.sha256(original).hexdigest()
   doc=RPSGL(original,path); row['kind']=doc.kind
   saved=tmp/'unchanged.rpsgl'; doc.save(saved); assert saved.read_bytes()==original
   row['unchanged_save_exact']=True
   mesh=doc.contents.asset(None).mesh if doc.kind=='container' else doc.mesh
   row.update(vertices=len(mesh.positions),triangles=len(mesh.faces),edge_parts=len(getattr(doc,'equipment_parts',[])))
   for ext,writer,reader in [('obj',export_obj,import_obj),('fbx',export_fbx,import_fbx)]:
    dest=tmp/('mesh.'+ext); writer(mesh,dest); imported=reader(dest)
    np.testing.assert_array_equal(imported.faces,mesh.faces)
    # OBJ uses ten significant digits; arena coordinates magnify decimal rounding.
    row[ext+'_max_position_error']=float(np.max(np.abs(imported.positions-mesh.positions)))
    np.testing.assert_allclose(imported.positions,mesh.positions,atol=1e-8,rtol=5.1e-10)
    if doc.kind=='model':
     copy=RPSGL(original,path); copy.import_mesh(imported); assert bytes(copy.data)==original
    row[ext+'_roundtrip']=True
   if getattr(doc,'is_equipment',False):
    changed=Mesh(mesh.positions.copy(),mesh.uv.copy(),mesh.normals.copy(),mesh.faces.copy())
    changed.positions[0,0]+=1/16384
    doc.import_mesh(changed)
    np.testing.assert_allclose(doc.mesh.positions,changed.positions,atol=1/16384,rtol=0)
    assert bytes(doc.data)!=original and len(doc.data)==len(original)
    allowed=np.zeros(len(original),dtype=bool)
    for part in doc.equipment_parts:
     for attr in part['stream'].attributes:
      for i in range(len(part['mesh'].positions)):
       offset=part['vertex']+i*part['stream'].stride+attr.offset
       allowed[offset:offset+attr.size]=True
    delta=np.frombuffer(original,np.uint8)!=np.frombuffer(doc.data,np.uint8)
    assert not np.any(delta & ~allowed)
    saved=tmp/'edited.rpsgl'; doc.save(saved)
    reopened=RPSGL.open(saved)
    np.testing.assert_allclose(reopened.mesh.positions,changed.positions,atol=1/16384,rtol=0)
    row['edit_reopen_and_opaque_bytes_preserved']=True
    for rejection in ('topology','range'):
     copy=RPSGL(original,path)
     bad=Mesh(mesh.positions.copy(),mesh.uv.copy(),mesh.normals.copy(),mesh.faces.copy())
     if rejection=='topology': bad.faces=bad.faces[::-1].copy()
     else: bad.positions+=100
     try: copy.import_mesh(bad)
     except ValueError: pass
     else: raise AssertionError(rejection+' accepted')
     assert bytes(copy.data)==original
     row[rejection+'_rejected_atomically']=True
   assert path.read_bytes()==original
   row['status']='passed'
  except Exception as e: row.update(status='failed',error=str(e))
  print(json.dumps(row),flush=True)
(repo/'research/rpsgl2-review/results.json').write_text(json.dumps(results,indent=2),encoding='utf-8')
