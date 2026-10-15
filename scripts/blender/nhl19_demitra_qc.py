"""Measure exterior fit against the supplied CC5 rest surface; no asset mutation."""
import json,sys
import numpy as np
from pathlib import Path
from mathutils import Vector
from mathutils.bvhtree import BVHTree
root=Path(r'H:\TEMP\nhl19-demitra')
native=json.loads((root/'package/native_mesh.json').read_text())
m=next(m for m in native['submeshes'] if m['name']=='geo_headShape')
cc=next(m for m in json.loads((root/'source/source_geometry.json').read_text()) if m['name']=='0')
r=json.loads((root/'fit/uv_warp.json').read_text());shift=Vector(r['source_shift'])
source=[Vector(p)+shift for p in cc['positions']]
tree=BVHTree.FromPolygons(source,cc['triangles'],all_triangles=True)
ps=[Vector((p[0],-p[2],p[1])) for p in m['positions']]
dist=[];opposed=[]
for i,t in enumerate(m['triangles']):
 uv=np.mean([m['uv'][j] for j in t],axis=0)
 if not (.30<uv[0]<.70 and .15<uv[1]<.60):continue
 if .326<uv[1]<.330:continue # Native lash strips carry their own offset.
 a,b,c=[ps[j] for j in t];n=(b-a).cross(c-a)
 if n.length<1e-10:continue
 p=(a+b+c)/3;hit,sn,_,d=tree.find_nearest(p);dist.append(d*1000)
 if n.normalized().dot(sn)<0:opposed.append({'triangle':i,'uv':uv.tolist(),'distance_mm':d*1000})
anchors=np.array([a for a,b in r['pairs_pixels']])/1024;coef=np.array(r['coefficients']);n=len(anchors)
def warp(p):
 rr=((p[:,None]-anchors)**2).sum(2)
 return (rr*np.log(np.maximum(rr,1e-20)))@coef[:n]+np.c_[np.ones(len(p)),p]@coef[n:]
yy,xx=np.mgrid[.1:.8:100j,.15:.85:100j];p=np.c_[xx.ravel(),yy.ravel()];q=warp(p)
dx=(warp(p+[1e-5,0])-q)/1e-5;dy=(warp(p+[0,1e-5])-q)/1e-5
det=dx[:,0]*dy[:,1]-dx[:,1]*dy[:,0]
assert det.min()>0,'UV registration folds the facial atlas'
report={'uv_jacobian_min':float(det.min()),'uv_grid_samples':len(det),'front_surface_triangles':len(dist),'front_surface_error_mm':{'median':float(np.median(dist)),'p95':float(np.percentile(dist,95)),'max':max(dist)},'opposed_source_normals':opposed}
(root/'package/source_fit_qc.json').write_text(json.dumps(report,indent=2))
print('QC',json.dumps({**report,'opposed_source_normals':len(opposed)}))
