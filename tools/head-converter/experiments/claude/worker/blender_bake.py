"""Worker Blender stage: surface bake driver (BVH nearest + image I/O in Blender; all arithmetic in src/conversion/bake.py).

blender --background --factory-startup --python-exit-code 1 --python blender_bake.py -- <stage.json>
stage.json: {fitted_native_mesh, source_geometry, ledger (source_shift + TPS), cc5_diffuse, native_diffuse, out_dir, size, policy(optional)}
Writes bake_{A,B,H,F,G}.png, diag_*.png, bake_report.json into out_dir (refuses an existing out_dir).
"""
import bpy, hashlib, json, sys, time
from pathlib import Path
import numpy as np
sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
from src.conversion import bake as BK, geometry as G, exchange as X, cc5_source as C
from mathutils import Vector
from mathutils.bvhtree import BVHTree
cfg = json.loads(Path(sys.argv[sys.argv.index('--') + 1]).read_text())
out = Path(cfg['out_dir'])
if out.exists(): raise SystemExit(f'refusing to overwrite existing output directory {out}')
out.mkdir(parents=True)
t0 = time.time()
fitted = X.load(cfg['fitted_native_mesh']); source = C.load(cfg['source_geometry']); ledger = json.loads(Path(cfg['ledger']).read_text())
head = X.head(fitted); duv = np.array(head['uv']); dtris = np.array(head['triangles'])
dpos = G.native_to_blender(head['positions']); dorig = G.native_to_blender(head['original_positions'])
S0, stris, suv = C.head_arrays(source); shift = np.array(ledger['source_shift']); spos = S0 + shift
anchors = np.array([a for a, b in ledger['pairs_pixels']], float) / 1024; coef = np.array(ledger['coefficients']); nA = len(anchors)


def warp(p):
    r2 = ((p[:, None, :] - anchors[None, :, :]) ** 2).sum(2)
    return (r2 * np.log(np.maximum(r2, 1e-20))) @ coef[:nA] + np.c_[np.ones(len(p)), p] @ coef[nA:]


def load_rgba(path):
    im = bpy.data.images.load(str(path), check_existing=False); im.colorspace_settings.name = 'Non-Color'
    ar = np.empty(len(im.pixels), np.float32); im.pixels.foreach_get(ar)
    return np.flipud(ar.reshape(im.size[1], im.size[0], 4)).copy()


src_img = load_rgba(cfg['cc5_diffuse']); nat_img = load_rgba(cfg['native_diffuse'])
tree = BVHTree.FromPolygons([Vector(p) for p in spos], stris.tolist(), all_triangles=True)


def nearest(p):
    loc, nrm, ti, d = tree.find_nearest(Vector(p)); return np.array(loc), int(ti), float(d)


policy = cfg.get('policy') or BK.DEFAULTS
res = BK.bake(dpos, dorig, dtris, duv, spos, stris, suv, src_img, nat_img, warp, nearest, size=int(cfg.get('size', 1024)), policy=policy, log=print)


def save_png(name, atlas):
    im = bpy.data.images.new(name, width=atlas.shape[1], height=atlas.shape[0], alpha=True); im.colorspace_settings.name = 'Non-Color'
    im.pixels.foreach_set(np.flipud(atlas).ravel()); im.file_format = 'PNG'; im.filepath_raw = str(out / (name + '.png')); im.save()


names = {'A': 'bake_A_tps', 'B': 'bake_B_surface', 'H': 'bake_H_hybrid', 'F': 'bake_F_final', 'G': 'bake_G_boundary'}
for k, atlas in res['atlases'].items(): save_png(names[k], atlas)
for k, img in res['diag'].items(): save_png('diag_' + k, img)
rep = res['report']; rep['candidate_atlas'] = str(out / 'bake_G_boundary.png'); rep['candidate_atlas_sha256'] = hashlib.sha256((out / 'bake_G_boundary.png').read_bytes()).hexdigest()
rep['blender'] = bpy.app.version_string; rep['inputs'] = {k: {'path': cfg[k], 'sha256': hashlib.sha256(Path(cfg[k]).read_bytes()).hexdigest()} for k in ('fitted_native_mesh', 'source_geometry', 'ledger', 'cc5_diffuse', 'native_diffuse')}
rep['total_seconds'] = round(time.time() - t0, 1)
(out / 'bake_report.json').write_text(json.dumps(rep, indent=1))
print('BAKED', rep['candidate_atlas_sha256'], rep['total_seconds'], 's')
