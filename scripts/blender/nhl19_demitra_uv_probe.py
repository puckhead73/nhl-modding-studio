"""Export rest geometry and UV-boundary evidence for CC5-to-NHL correspondence."""
import bpy,json,sys,collections
from pathlib import Path
root=Path(r'H:\TEMP\nhl19-demitra')
bpy.ops.wm.open_mainfile(filepath=str(root/'source/Demitra_Source.blend'))
result=[]
for ob in bpy.data.objects:
 if ob.type!='MESH':continue
 original=[list(ob.matrix_world@v.co) for v in ob.data.vertices]
 ps=[];uv=[];lookup={};loop_ids={}
 for loop in ob.data.loops:
  t=tuple(ob.data.uv_layers.active.data[loop.index].uv)
  key=(loop.vertex_index,*t)
  if key not in lookup:lookup[key]=len(ps);ps.append(original[loop.vertex_index]);uv.append(list(t))
  loop_ids[loop.index]=lookup[key]
 ob.data.calc_loop_triangles()
 tris=[[loop_ids[l] for l in t.loops] for t in ob.data.loop_triangles]
 result.append({'name':ob.name,'positions':ps,'uv':uv,'triangles':tris,'triangle_materials':[t.material_index for t in ob.data.loop_triangles]})
(root/'source/source_geometry.json').write_text(json.dumps(result))
donor=json.loads((root/'vanilla/gaborik_native.json').read_text())
head=next(m for m in donor['submeshes'] if m['name']=='geo_headShape')
head={'name':'NHL19','positions':[[p[0],-p[2],p[1]] for p in head['positions']], 'uv':[[u,1-v] for u,v in head['uv']],'triangles':head['triangles']}
reports=[]
for m in [head,next(m for m in result if m['name']=='0')]:
 edges=collections.Counter(tuple(sorted(e)) for t in m['triangles'] for e in [(t[0],t[1]),(t[1],t[2]),(t[2],t[0])])
 graph=collections.defaultdict(set)
 for (a,b),count in edges.items():
  if count==1:graph[a].add(b);graph[b].add(a)
 pending=set(graph);groups=[]
 while pending:
  start=pending.pop();group={start};q=[start]
  while q:
   for n in graph[q.pop()]:
    if n in pending:pending.remove(n);group.add(n);q.append(n)
  ids=sorted(group);uv=[m['uv'][i] for i in ids]
  groups.append({'ids':ids,'count':len(ids),'uv_bounds':[[min(p[k] for p in uv) for k in range(2)],[max(p[k] for p in uv) for k in range(2)]]})
 reports.append({'name':m['name'],'boundary_groups':groups})
(root/'uv_boundary_report.json').write_text(json.dumps(reports,indent=2))
print(json.dumps([{'name':r['name'],'boundaries':[{'count':g['count'],'uv_bounds':g['uv_bounds']} for g in r['boundary_groups']]} for r in reports]))
