"""Rebake the CC5 source at 2K using the successful Demitra UV calibration.

Texture-only fixture: does not refit geometry or write game archives.
"""
import hashlib
import json
from pathlib import Path
import bpy
import numpy as np

root = Path(r'H:\TEMP\nhl19-demitra')
out = root / 'diffuse2k'
out.mkdir(exist_ok=True)
assert not (out / 'installation.json').exists(), 'Preserve installed test packages'
ledger = json.loads((root / 'fit/uv_warp.json').read_text())
source_path = Path(r'S:\Downloads\Mogilny Test\Demitra\Demitra.fbm\Std_Skin_Head_Diffuse.png')

def pixels(path):
    im = bpy.data.images.load(str(path), check_existing=False)
    im.colorspace_settings.name = 'Non-Color'
    values = np.empty(len(im.pixels), np.float32)
    im.pixels.foreach_get(values)
    return np.flipud(values.reshape(im.size[1], im.size[0], 4)).copy()

def sample(im, q):
    xy = np.clip(q * np.array([im.shape[1], im.shape[0]]) - .5, 0,
                 np.array([im.shape[1] - 1.001, im.shape[0] - 1.001]))
    x, y = xy.astype(int).T
    f = xy - np.c_[x, y]
    return ((im[y, x] * (1-f[:, 0, None]) + im[y, x+1] * f[:, 0, None]) * (1-f[:, 1, None])
            + (im[y+1, x] * (1-f[:, 0, None]) + im[y+1, x+1] * f[:, 0, None]) * f[:, 1, None])

src = pixels(source_path)
assert src.shape[:2] == (4096, 4096)
oral = pixels(root / 'vanilla/decoded/game_90-008#69297_head_marian_gaborik_cm.png')
anchors = np.array([a for a, b in ledger['pairs_pixels']], float) / 1024
coef = np.array(ledger['coefficients'])
size = 2048
atlas = np.empty((size, size, 4), np.float32)
for y0 in range(0, size, 16):
    yy, xx = np.mgrid[y0:y0+16, 0:size]
    q = np.c_[(xx.ravel()+.5)/size, (yy.ravel()+.5)/size]
    r2 = ((q[:, None, :] - anchors[None, :, :]) ** 2).sum(2)
    mapped = (r2*np.log(np.maximum(r2, 1e-20))) @ coef[:len(anchors)] + np.c_[np.ones(len(q)), q] @ coef[len(anchors):]
    values = sample(src, mapped)
    # Same native oral-region rule as the successful 1K bake, scaled to 2K.
    mask = (yy.ravel()/size > .84) & ((xx.ravel()/size < .405) | (xx.ravel()/size > .595))
    values[mask] = sample(oral, q[mask])
    atlas[y0:y0+16] = values.reshape(16, size, 4)
atlas[:, :, 3] = 1
assert np.isfinite(atlas).all()
np.save(out / 'source_rgba.npy', atlas)
im = bpy.data.images.new('Demitra_Diffuse_2K', width=size, height=size, alpha=True)
im.colorspace_settings.name = 'Non-Color'
im.pixels.foreach_set(np.flipud(atlas).ravel())
im.file_format = 'PNG'
im.filepath_raw = str(out / 'Demitra_Diffuse_2K.png')
im.save()
(out / 'bake_report.json').write_text(json.dumps({
    'source': str(source_path), 'source_sha256': hashlib.sha256(source_path.read_bytes()).hexdigest(),
    'source_resolution': [4096, 4096], 'output_resolution': [size, size],
    'mapping_sha256': hashlib.sha256((root / 'fit/uv_warp.json').read_bytes()).hexdigest(),
    'method': 'Resample original 4K CC5 source through unchanged UV calibration; native oral islands retained',
    'mesh_baseline': str(root / 'revision2'),
}, indent=2))
print('BAKED', out / 'Demitra_Diffuse_2K.png')
