"""Replay the preserved Demitra CC5 recipe for Turgeon over Bouwmeester.

Run with Blender --background --python-exit-code 1 --python this.py -- STAGE.
Stages: probe, fit, finalize, repair, diffuse2k. Derived scripts and substitutions
are recorded in scratch; the successful Demitra fixtures are never overwritten.
"""
import hashlib
import json
import sys
from pathlib import Path
import numpy as np

root = Path(r'H:\TEMP\nhl19-turgeon')
reference = Path(r'H:\TEMP\nhl19-head-import-reference-2026-09-11')
stage = sys.argv[sys.argv.index('--')+1]
names = {'probe': 'nhl19_demitra_uv_probe.py', 'fit': 'nhl19_demitra_fit.py',
         'finalize': 'nhl19_demitra_finalize.py', 'repair': 'nhl19_demitra_repair.py',
         'diffuse2k': 'nhl19_demitra_diffuse_2k.py', 'qc':'nhl19_demitra_qc.py'}
name = names[stage]
relative = 'scripts/blender/' + name
original = reference / relative
manifest = json.loads((reference/'manifest.json').read_text(encoding='utf-8-sig'))
expected = next(f['sha256'] for f in manifest['files'] if f['path'] == relative)
assert hashlib.sha256(original.read_bytes()).hexdigest() == expected
code = original.read_text()
replacements = [
    ('nhl19-demitra', 'nhl19-turgeon'),
    ('marian_gaborik', 'jay_bouwmeester'), ('gaborik', 'bouwmeester'),
    ('game_90-008#69297', 'game_90-008#63246'),
    ('Mogilny Test\\Demitra', 'Mogilny Test\\Turgeon'),
    ('Demitra.fbm', 'TurgeonTest.fbm'), ('Demitra_Source', 'TurgeonTest_Source'),
    ('Demitra', 'Turgeon'),
]
for before, after in replacements:
    code = code.replace(before, after)
if stage == 'fit':
    code=code.replace('place_head(dp,sampled,1.66,1.74)','place_head(dp,sampled,1.66,1.695)')
    start=code.index('neck_tris=')
    end=code.index('# Bouwmeester embeds',start) if '# Bouwmeester embeds' in code[start:] else code.index('# Gaborik embeds',start)
    code=code[:start]+'# Neck attachment is handled by the measured final repair band.\n'+code[end:]
if stage == 'qc':
    code=code.replace('package/native_mesh.json','revision2/native_mesh.json').replace('package/source_fit_qc.json','revision2/source_fit_qc.json')
    code=code.replace('dist=[];opposed=[]','dist=[];opposed=[];samples=[]').replace('dist.append(d*1000)','dist.append(d*1000);samples.append({"triangle":i,"uv":uv.tolist(),"distance_mm":d*1000,"position":list(p)})')
    code=code.replace("print('QC'", "(root/'revision2/worst_surface_samples.json').write_text(json.dumps(sorted(samples,key=lambda s:s['distance_mm'],reverse=True)[:25],indent=2))\nprint('QC'")
if stage == 'repair':
    # Bouwmeester's lower jaw extends below Gaborik's fixed-neck cutoff.
    code=code.replace('1.735','1.695').replace('neck_top = 1.775 + .045 * rear_weight','neck_top = 1.755 + .060 * rear_weight')
    # Different donor proportions need a continuous transition OUTSIDE the exact
    # ear lock too; a hard boundary produced narrow scalp creases on this donor.
    marker = "rear_weight[ear_ids] = 0"
    assert marker in code
    code = code.replace(marker, marker + "\near_margin = np.linalg.norm(old[:, None, :] - old[ear_ids][None, :, :], axis=2).min(axis=1)\nrear_weight *= smooth(ear_margin / .035)")
    marker = "# Restore the crown exactly after the posterior correspondence adjustment."
    code = code.replace(marker, """# Remove chart-transition dimples on the convex upper scalp only.
for i in np.flatnonzero((old[:,1]>1.855) & (old[:,2]<.065)):
    p = Vector((old[i,0],-old[i,2],old[i,1]))
    hit,_,_,_=tree.ray_cast(centre,(p-centre).normalized(),.5)
    if hit is None: hit,_,_,_=tree.find_nearest(p)
    w=smooth((old[i,1]-1.855)/.015)
    result[i]=result[i]*(1-w)+np.array((hit.x,hit.z,-hit.y))*w
""" + marker)
    marker="ear_ids = np.array(sorted(ear_ids), dtype=int)"
    # Keep the full two-ring ear guard. Surface relaxation happens outside it.
    marker='result[ear_ids] = fitted[ear_ids]'
    code=code.replace(marker, marker+"""
# Relax only the transition surface on welded adjacency; retain native records.
adj=[set() for _ in result]
for triangle in head['triangles']:
    for i in triangle: adj[i].update(j for j in triangle if j!=i)
for ids in groups.values():
    neighbours=set().union(*(adj[i] for i in ids))-set(ids)
    for i in ids: adj[i]=neighbours
relax=smooth((old[:,1]-1.735)/.035)*(1-smooth((old[:,1]-1.925)/.015))*(1-smooth((old[:,2]-.035)/.045))
relax[ear_ids]=0;relax[ledger['oral_ids']]=0
for _ in range(12):
    next_result=result.copy()
    for i in np.flatnonzero(relax>0):
        if adj[i]: next_result[i]=result[i]*(1-.35*relax[i])+result[list(adj[i])].mean(axis=0)*(.35*relax[i])
        if old[i,1]>1.845:
            p=Vector((next_result[i,0],-next_result[i,2],next_result[i,1]))
            hit,_,_,_=tree.find_nearest(p)
            w=smooth((old[i,1]-1.845)/.02)
            next_result[i]=next_result[i]*(1-w)+np.array((hit.x,hit.z,-hit.y))*w
    result=next_result
for ids in groups.values():
    if len(ids)>1: result[ids]=result[ids].mean(axis=0)
result[ear_ids]=fitted[ear_ids]
""")
derived = root/'recipe_sources'
derived.mkdir(exist_ok=True)
for helper in ['head_scalp_placement.py']:
    (derived/helper).write_bytes((reference/'scripts/blender'/helper).read_bytes())
path = derived/name.replace('demitra','turgeon')
path.write_text(code)
(derived/(stage+'_provenance.json')).write_text(json.dumps({
    'reference': str(original), 'sha256': expected, 'substitutions': replacements,
    'derived_sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
}, indent=2))
if stage == 'fit':
    prior_root = Path(r'H:\TEMP\nhl19-demitra')
    def head_at(p):
        return next(m for m in json.loads(p.read_text())['submeshes'] if m['name']=='geo_headShape')
    donor = head_at(root/'vanilla/bouwmeester_native.json')
    prior = head_at(prior_root/'vanilla/gaborik_native.json')
    source = next(m for m in json.loads((root/'source/source_geometry.json').read_text()) if m['name']=='0')
    source_prior = next(m for m in json.loads((prior_root/'source/source_geometry.json').read_text()) if m['name']=='0')
    checks = {'native_uv_equal': donor['uv']==prior['uv'],
              'native_triangles_equal': donor['triangles']==prior['triangles'],
              'source_uv_equal': source['uv']==source_prior['uv'],
              'source_triangles_equal': source['triangles']==source_prior['triangles']}
    (root/'template_parity.json').write_text(json.dumps(checks,indent=2))
    assert all(checks.values()), 'UV/template drift: recalibration required'
sys.path.insert(0, str(derived))
exec(compile(code, str(path), 'exec'), {'__file__':str(path), '__name__':'__main__'})
