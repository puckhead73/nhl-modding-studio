# Frozen math/texture helpers adapted from qualified Codex NHL19 writer; no NHL19 anatomical IDs.
import bpy, numpy as np, struct, hashlib
from reference_pack import bc1,tile_offsets
from shading_proof import positive_uniform_scale_translation
sha=lambda b:hashlib.sha256(b).hexdigest()


def normals_tangents(mesh, donor):
    """Regenerate geometry shading, preserving authored normal splits and UV tangent splits.

    Merge normals only for donor-identical XYZ AND f16 normal XYZ, with still-coincident
    fitted positions. Never weld topology, UV records or authored hard normals.
    Degenerate/unused vertices retain normalized donor normals and are counted.
    """
    p = np.asarray(mesh['positions'], dtype=np.float32).astype(float)
    uv = np.asarray(mesh['uv'], float)
    tri = np.asarray(mesh['triangles'], int)
    oldn = np.asarray(donor['normals'], float)
    original = np.asarray(mesh['original_positions'], np.float32)
    face = np.cross(p[tri[:, 1]]-p[tri[:, 0]], p[tri[:, 2]]-p[tri[:, 0]])
    n = np.zeros_like(p)
    for corner in range(3):
        np.add.at(n, tri[:, corner], face)
    groups = {}
    for i in range(len(p)):
        key = (original[i].tobytes(), oldn[i].astype('<f2').tobytes(), p[i].tobytes())
        groups.setdefault(key, []).append(i)
    for ids in groups.values():
        n[ids] = n[ids].sum(axis=0)
    # Keep native shading exactly where no incident face moved. Frozen attachment
    # rings and retained oral geometry should not lose their authored shading.
    moved=np.any(p!=original.astype(float),axis=1)
    active=np.zeros(len(p),bool)
    active[tri[np.any(moved[tri],axis=1)].reshape(-1)]=True
    for ids in groups.values():
        if active[ids].any():active[ids]=True
    eye_proof=None
    if mesh['name'] in ('geo_leyeShape','geo_reyeShape'):
        eye_proof=positive_uniform_scale_translation(original,p)
        if eye_proof['accepted']:
            # Positive uniform scale plus translation leaves authored normal and
            # tangent directions unchanged. Retain exact native shading bytes.
            active[:]=False
    fallback = np.linalg.norm(n, axis=1) < 1e-20
    n[fallback] = oldn[fallback]
    lengths = np.linalg.norm(n, axis=1)
    if (lengths < 1e-20).any():
        raise ValueError('unrecoverable zero normal')
    n /= lengths[:, None]
    n[~active]=oldn[~active]
    t = np.zeros_like(p)
    for a,b,c in tri:
        e1,e2 = p[b]-p[a],p[c]-p[a]
        u,v = uv[b]-uv[a],uv[c]-uv[a]
        den = u[0]*v[1]-u[1]*v[0]
        if abs(den) > 1e-12:
            value = (e1*v[1]-e2*u[1])/den
            t[[a,b,c]] += value
    t -= n*(t*n).sum(axis=1)[:,None]
    tf = np.linalg.norm(t,axis=1)<1e-10
    for i in np.flatnonzero(tf):
        t[i] = np.cross(n[i], (0,1,0) if abs(n[i,1])<.9 else (1,0,0))
    t /= np.linalg.norm(t,axis=1)[:,None]
    assert np.isfinite(n).all() and np.isfinite(t).all()
    return n,t,{'normal_fallback_vertices':int(fallback.sum()),
                 'tangent_fallback_vertices':int(tf.sum()),
                 'shading_regenerated_vertices':int(active.sum()),
                 'native_shading_preserved_vertices':int((~active).sum()),
                 'eye_uniform_scale_translation_proof':eye_proof,
                 'smooth_duplicate_groups':sum(len(ids)>1 for ids in groups.values()),
                 'zero_area_faces':int((np.linalg.norm(face,axis=1)<1e-20).sum())},active

def decode_bc1(encoded, w, h):
    """Independent BC1 block decoding for quantization receipts and visual previews."""
    raw = np.frombuffer(encoded, dtype=np.uint8).reshape(-1,8)
    ends = raw[:,:4].copy().view('<u2').reshape(-1,2)
    def rgb(c):
        r=(c>>11)&31; g=(c>>5)&63; b=c&31
        return np.stack([(r<<3)|(r>>2),(g<<2)|(g>>4),(b<<3)|(b>>2)],axis=1).astype(float)
    a,b = rgb(ends[:,0]),rgb(ends[:,1])
    palette=np.stack([a,b,np.floor((2*a+b)/3),np.floor((a+2*b)/3)],axis=1)
    three_color=ends[:,0]<=ends[:,1]
    palette[three_color,2]=np.floor((a[three_color]+b[three_color])/2)
    palette[three_color,3]=0
    bits=raw[:,4:].copy().view('<u4').reshape(-1)
    ids=(bits[:,None]>>(2*np.arange(16,dtype=np.uint32)))&3
    pixels=palette[np.arange(len(raw))[:,None],ids]
    bw=(w+3)//4;bh=(h+3)//4
    return pixels.reshape(bh,bw,4,4,3).transpose(0,2,1,3,4).reshape(bh*4,bw*4,3)[:h,:w]

def save_png(path, rgb):
    h,w,_=rgb.shape
    image=bpy.data.images.new(path.stem,width=w,height=h,alpha=True)
    image.colorspace_settings.name='Non-Color'
    rgba=np.ones((h,w,4),np.float32);rgba[:,:,:3]=rgb/255
    image.pixels.foreach_set(rgba[::-1].reshape(-1))
    image.filepath_raw=str(path);image.file_format='PNG';image.save()
    bpy.data.images.remove(image)

def texture_package(original, atlas_path, out):
    assert original[:4]==b'GNF ' and len(original)==701696
    d1,d2,d3=struct.unpack_from('<III',original,0x14)
    assert (d1>>20)&63==35 and (d3>>20)&31==13
    assert (d2&16383)+1==1024 and ((d2>>14)&16383)+1==1024 and (d3>>16)&15==10
    image=bpy.data.images.load(str(atlas_path),check_existing=False)
    image.colorspace_settings.name='Non-Color'
    assert tuple(image.size)==(1024,1024)
    rgba=np.empty(1024*1024*4,np.float32);image.pixels.foreach_get(rgba)
    rgb=rgba.reshape(1024,1024,4)[::-1,:,:3].copy()
    bpy.data.images.remove(image)
    assert np.isfinite(rgb).all() and rgb.min()>=0 and rgb.max()<=1
    linear=np.where(rgb<=.04045,rgb/12.92,((rgb+.055)/1.055)**2.4)
    gnf=bytearray(original);allowed=np.zeros(len(gnf),bool);offset=256;receipt=[]
    for level in range(11):
        h,w,_=linear.shape
        srgb=np.where(linear<=.0031308,linear*12.92,1.055*linear**(1/2.4)-.055)
        target=np.rint(np.clip(srgb,0,1)*255)
        encoded=bc1(target)
        size=((w+31)//32)*((h+31)//32)*512
        for src,dst in tile_offsets(w,h):
            gnf[offset+dst:offset+dst+8]=encoded[src:src+8]
            allowed[offset+dst:offset+dst+8]=True
        # Independent coordinate formulation avoids calling encoder's tile_offsets again.
        recovered=bytearray()
        for y in range((h+3)//4):
            for x in range((w+3)//4):
                morton=(x&1)|((y&1)<<1)|((x&2)<<1)|((y&2)<<2)|((x&4)<<2)|((y&4)<<3)
                tiled=((y//8)*(((w+3)//4+7)//8)+x//8)*512+morton*8
                recovered.extend(gnf[offset+tiled:offset+tiled+8])
        assert recovered==encoded
        decoded=decode_bc1(encoded,w,h)
        error=np.abs(decoded-target)
        (out/f'diffuse_mip_{level}.bc1').write_bytes(encoded)
        save_png(out/f'diffuse_mip_{level}_decoded.png',decoded)
        receipt.append({'level':level,'width':w,'height':h,'offset':offset,'tiled_size':size,
                        'linear_sha256':sha(encoded),'mean_absolute_rgb8_error':float(error.mean()),
                        'p95_absolute_rgb8_error':float(np.percentile(error,95)),
                        'max_absolute_rgb8_error':float(error.max())})
        offset+=size
        if w>1:linear=linear.reshape(h//2,2,w//2,2,3).mean((1,3))
    assert offset==len(gnf)
    a=np.frombuffer(original,np.uint8);b=np.frombuffer(gnf,np.uint8)
    assert np.array_equal(a[~allowed],b[~allowed])
    path=out/'head_aaron_ekblad_cm.gnf';path.write_bytes(gnf)
    return path,{'sha256':sha(gnf),'size':len(gnf),'mips':receipt,
                 'header_and_padding_unchanged':True,'original_sha256':sha(original)}
