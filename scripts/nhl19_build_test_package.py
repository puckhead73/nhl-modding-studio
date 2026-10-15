"""Build a bounded NHL19 archive test package; never edits the installed game.
Requires NumPy (Blender's bundled Python). Usage: python this.py output_directory
"""
import os,sys,json,struct,hashlib,zlib,math
from pathlib import Path
import numpy as np
root=Path(sys.argv[1])
recipe=json.loads(Path(sys.argv[3]).read_text(encoding='utf-8-sig')) if len(sys.argv)>3 and sys.argv[2]=='--recipe' else {}
texture_only=bool(recipe.get('texture_only',False))
vanilla=Path(recipe.get('vanilla',r'E:\tmp\nhl19-mogilny\vanilla'))
slug=recipe.get('slug','jack_eichel');texture_name=f'head_{slug}_cm.gnf'
donor_paths={k:Path(v) for k,v in recipe.get('donors',{'base':r'E:\tmp\nhl19-mogilny\out\eichel_from_game90.mcd','patch91':r'E:\tmp\nhl19-mogilny\out\eichel_from_p91.mcd'}).items()}
previous=Path(sys.argv[2]) if len(sys.argv)>2 and sys.argv[2]!='--recipe' else None
if recipe.get('previous_package'):previous=Path(recipe['previous_package'])
previous_receipt=json.loads((previous/'installation.json').read_text()) if previous else None
if previous_receipt:assert previous_receipt['status']=='installed'
game=Path(r'H:\Emulators\ShadPS4\Games\CUSA11126\databigs')
sha=lambda b:hashlib.sha256(b).hexdigest()
def filehash(p):
    h=hashlib.sha256()
    with open(p,'rb') as f:
        for b in iter(lambda:f.read(4*1024*1024),b''):h.update(b)
    return h.hexdigest()
with open(root/'native_mesh.json') as f:mesh_input=json.load(f)
mesh_by_name={m['name']:m for m in mesh_input['submeshes']}
def tangents(mesh):
    p=np.array(mesh['positions']);uv=np.array(mesh['uv']);n=np.array(mesh['normals'])
    t=np.zeros_like(p)
    for a,b,c in mesh['triangles']:
        e1=p[b]-p[a];e2=p[c]-p[a];u=uv[b]-uv[a];v=uv[c]-uv[a]
        den=u[0]*v[1]-u[1]*v[0]
        if abs(den)>1e-12:
            value=(e1*v[1]-e2*u[1])/den
            for i in (a,b,c):t[i]+=value
    t-=n*np.sum(t*n,axis=1)[:,None]
    for i in range(len(t)):
        if np.linalg.norm(t[i])<1e-10:
            t[i]=np.cross(n[i],(0,1,0) if abs(n[i,1])<.9 else (1,0,0))
        t[i]/=max(np.linalg.norm(t[i]),1e-20)
    return t

asset_report={'meshes':{},'textures':{}}
for revision,donor_path in ([] if texture_only else donor_paths.items()):
    original=donor_path.read_bytes();data=bytearray((root/(revision+'.positions.mcd')).read_bytes())
    layout=json.loads((root/(revision+'.layout.json')).read_text())
    assert sha(original)==layout['donor_sha256'] and sha(data)==layout['position_sha256']
    # All four actual geometry declarations on these exact donors use n0/g0 4f16.
    for s in layout['streams']:
        m=mesh_by_name[s['name']];ts=tangents(m)
        for i,(n,t) in enumerate(zip(m['normals'],ts)):
            assert abs(np.linalg.norm(n)-1)<.01 and abs(np.linalg.norm(t)-1)<.01
            for off,value in [(s['normal_offset'],n),(s['tangent_offset'],t)]:
                pos=s['data_off']+i*s['stride']+off
                data[pos:pos+6]=struct.pack('<3e',*value)
                assert data[pos+6:pos+8]==original[pos+6:pos+8] # Preserve native fourth-component semantics.
    for i,(a,b) in enumerate(zip(original,data)):
        if a!=b:assert any(lo<=i<hi for lo,hi in layout['allowed_spans'])
    dest=root/(revision+'.mcd');dest.write_bytes(data)
    asset_report['meshes'][revision]={'sha256':sha(data),'size':len(data),'donor_sha256':sha(original),'shading':'regenerated XYZ f16 normals/tangents; original W preserved'}

def bc1(rgb):
    h,w,_=rgb.shape;pw=(w+3)//4*4;ph=(h+3)//4*4
    rgb=np.pad(rgb,((0,ph-h),(0,pw-w),(0,0)),mode='edge')
    blocks=rgb.reshape(ph//4,4,pw//4,4,3).transpose(0,2,1,3,4).reshape(-1,16,3).astype(float)
    centred=blocks-blocks.mean(1)[:,None,:]
    _,vectors=np.linalg.eigh(np.einsum('nvi,nvj->nij',centred,centred))
    axis=vectors[:,:,-1];project=np.einsum('nvi,ni->nv',centred,axis)
    ends=np.stack([blocks[np.arange(len(blocks)),project.argmax(1)],blocks[np.arange(len(blocks)),project.argmin(1)]],axis=1)
    q=np.rint(ends*np.array([31,63,31])/255).astype(np.uint16)
    codes=(q[:,:,0]<<11)|(q[:,:,1]<<5)|q[:,:,2]
    c0=codes.max(1);c1=codes.min(1)
    same=c0==c1;c0=np.where(same & (c0<65535),c0+1,c0);c1=np.where(c0==c1,c1-1,c1)
    def unpack(c):return np.stack([(c>>11)*255/31,((c>>5)&63)*255/63,(c&31)*255/31],axis=1)
    a=unpack(c0);b=unpack(c1);pal=np.stack([a,b,(2*a+b)/3,(a+2*b)/3],axis=1)
    nearest=((blocks[:,:,None,:]-pal[:,None,:,:])**2).sum(3).argmin(2).astype(np.uint32)
    bits=np.sum(nearest<<(np.arange(16,dtype=np.uint32)*2),axis=1,dtype=np.uint32)
    packed=np.empty((len(blocks),8),np.uint8)
    packed[:,:2]=c0.astype('<u2').view(np.uint8).reshape(-1,2);packed[:,2:4]=c1.astype('<u2').view(np.uint8).reshape(-1,2)
    packed[:,4:]=bits.astype('<u4').view(np.uint8).reshape(-1,4)
    return packed.tobytes()
def tile_offsets(w,h):
    bx=(w+3)//4;by=(h+3)//4;tx=(bx+7)//8
    for y in range(by):
        for x in range(bx):
            xx=x%8;yy=y%8;m=sum(((xx>>k)&1)<<(2*k)|((yy>>k)&1)<<(2*k+1) for k in range(3))
            yield (y*bx+x)*8,(((y//8)*tx+x//8)*64+m)*8
original=(vanilla/texture_name).read_bytes();gnf=bytearray(original)
d1,d2,d3=struct.unpack_from('<III',gnf,0x14)
assert (d1>>20)&63==35 and (d3>>20)&31==13
assert (d2&16383)+1==1024 and ((d2>>14)&16383)+1==1024 and (d3>>16)&15==10
rgb=np.load(root/'source_rgba.npy')[:,:,:3]
height,width,_=rgb.shape
assert width==height and width in (1024,2048) and np.isfinite(rgb).all()
levels=width.bit_length()
if width!=1024:
    # Resize only measured size-dependent fields in the real NHL19 descriptor.
    # 512/1024 vanilla maps corroborate pitch, file size, mip count and data size.
    total=256+sum(((max(1,width>>i)+31)//32)**2*512 for i in range(levels))
    gnf=bytearray(original[:256])+bytearray(total-256)
    assert struct.unpack_from('<I',original,12)[0]==len(original)
    assert struct.unpack_from('<I',original,44)[0]==len(original)-256
    pitch=struct.unpack_from('<I',original,32)[0]
    assert ((pitch>>13)&16383)+1==1024
    struct.pack_into('<I',gnf,12,total)
    struct.pack_into('<I',gnf,24,(d2&0xf0000000)|(width-1)|((height-1)<<14))
    struct.pack_into('<I',gnf,28,(d3&~(15<<16))|((levels-1)<<16))
    struct.pack_into('<I',gnf,32,(pitch&~(16383<<13))|((width-1)<<13))
    struct.pack_into('<I',gnf,44,total-256)
    allowed=set(range(12,16))|set(range(24,36))|set(range(44,48))
    assert all(a==b or i in allowed for i,(a,b) in enumerate(zip(original[:256],gnf[:256])))
linear=np.where(rgb<=.04045,rgb/12.92,((rgb+.055)/1.055)**2.4)
offset=256;mips=[]
for level in range(levels):
    h,w,_=linear.shape
    srgb=np.where(linear<=.0031308,linear*12.92,1.055*linear**(1/2.4)-.055)
    encoded=bc1(np.rint(np.clip(srgb,0,1)*255))
    size=((w+31)//32)*((h+31)//32)*512
    assert offset+size<=len(gnf)
    for src,dst in tile_offsets(w,h):gnf[offset+dst:offset+dst+8]=encoded[src:src+8]
    # Independent untile equality across every encoded mip; padding/header untouched.
    recovered=b''.join(gnf[offset+dst:offset+dst+8] for _,dst in tile_offsets(w,h));assert recovered==encoded
    mips.append({'level':level,'width':w,'height':h,'offset':offset,'tiled_size':size,'linear_sha256':sha(encoded)})
    (root/f'diffuse_mip_{level}.bc1').write_bytes(encoded)
    offset+=size
    if w>1:linear=linear.reshape(h//2,2,w//2,2,3).mean((1,3))
assert offset==len(gnf)
if width==1024:assert gnf[:256]==original[:256]
(root/texture_name).write_bytes(gnf)
asset_report['textures']={'diffuse_sha256':sha(gnf),'width':width,'height':height,'mips':mips,'normal_map':'original Eichel BC5 retained; no source normal map provided','resized_header_runtime_status':'2K requires in-game validation' if width==2048 else 'previously tested 1K layout'}
asset_report['textures']['diffuse_file']=texture_name
if recipe:asset_report['textures']['normal_map']=recipe['normal_map']

def table(path):
    with open(path,'rb') as f:
        header=f.read(48);assert header[:4]==b'EB\0\3' and struct.unpack_from('>I',header,8)[0]==0x110400
        count,noff=struct.unpack_from('>I',header,4)[0],struct.unpack_from('>I',header,12)[0]
        ns,ds,dc=header[20],header[21],struct.unpack_from('>H',header,22)[0]
        doff=(noff+count*ns+15)&~15;f.seek(0);prefix=f.read(doff+dc*ds)
    entries=[]
    for i in range(count):
        n=prefix[noff+i*ns:noff+(i+1)*ns];di=struct.unpack_from('>H',n)[0]
        directory=prefix[doff+di*ds:doff+(di+1)*ds].split(b'\0')[0].decode()
        name=n[2:].split(b'\0')[0].decode();row=prefix[48+20*i:68+20*i]
        block,_,size,_,_=struct.unpack('>5I',row)
        entries.append({'index':i,'path':directory+'/'+name,'offset':block*16,'size':size,'row':row.hex()})
    assert struct.unpack_from('>Q',header,24)[0]==path.stat().st_size
    return header,entries
def decode(raw):
    assert raw[:8]==b'chunkzip';full,chunk,n=struct.unpack_from('>III',raw,12);offset=40;out=b''
    for i in range(n):
        size=struct.unpack_from('>I',raw,offset)[0];out+=zlib.decompress(raw[offset+8:offset+8+size],-15)
        offset=((offset+8+size-8+15)&~15)+8
    assert len(out)==full
    return out
def compress_like(raw,data):
    full,chunk,old_n=struct.unpack_from('>III',raw,12)
    assert chunk==131072 and old_n==(full+chunk-1)//chunk
    # Check every original per-chunk flag before extending the chain.
    offset=40;gaps=[]
    for index in range(old_n):
        oldsize=struct.unpack_from('>I',raw,offset)[0];gaps.append(raw[offset+4:offset+8])
        offset=((offset+8+oldsize-8+15)&~15)+8
    assert len(set(gaps))==1
    n=(len(data)+chunk-1)//chunk
    header=bytearray(raw[:40]);struct.pack_into('>III',header,12,len(data),chunk,n)
    out=bytearray(header)
    for index in range(n):
        gap=gaps[0]
        compressed=zlib.compress(data[index*chunk:(index+1)*chunk],9,wbits=-15)
        out+=struct.pack('>I',len(compressed))+gap+compressed
        if index<n-1:out+=b'\0'*((((len(out)-8+15)&~15)+8)-len(out))
    assert decode(out)==data
    return bytes(out)
manifest={'game_root':str(game.parent),'status':'prepared; runtime loading/animation unverified','archives':[],'assets':asset_report}
if recipe:manifest['recipe']=recipe
if previous:manifest['rollback_returns_to']=str(previous)
if texture_only:manifest['test_variable']='Diffuse texture only; no mesh or patch archive changes'
archive_jobs=[('game_90-008.big','base',True)] if texture_only else [('game_90-008.big','base',True),('patch_game_91.big','patch91',False),('patch_game_93.big','base',False)]
for archive_name,revision,with_texture in archive_jobs:
    path=game/archive_name;header,entries=table(path);oldsize=path.stat().st_size
    arc={'path':str(path),'original_size':oldsize,'original_sha256':filehash(path),'original_header':header.hex(),'changes':[]}
    if recipe:assert arc['original_sha256']==recipe['baseline_sha256'][archive_name],'Current archive differs from accepted baseline'
    if previous:
        prior=next(a for a in previous_receipt['archives'] if a['plan']['path']==str(path))
        assert arc['original_sha256']==prior['installed_sha256'],'live previous installation changed'
    cursor=oldsize
    for name,replacement in ([] if texture_only else [(slug+'.mcd',root/(revision+'.mcd'))])+([(texture_name,root/texture_name)] if with_texture else []):
        found=[e for e in entries if e['path']=='nhl/data/kettle/rendering/characters/player/'+name];assert len(found)==1
        entry=found[0]
        with open(path,'rb') as f:f.seek(entry['offset']);raw=f.read(entry['size'])
        decoded=decode(raw)
        expected=donor_paths[revision] if name.endswith('.mcd') else vanilla/name
        if previous:
            prior_change=next(c for c in prior['plan']['changes'] if c['path']==entry['path'])
            expected=previous/prior_change['decoded_file']
            assert sha(decoded)==prior_change['decoded_sha256']
        assert decoded==expected.read_bytes()
        data=replacement.read_bytes();stored=compress_like(raw,data)
        stored_name=archive_name+'__'+name+'.chunkzip';(root/stored_name).write_bytes(stored)
        cursor=(cursor+15)&~15;row=bytearray.fromhex(entry['row']);struct.pack_into('>I',row,0,cursor//16);struct.pack_into('>I',row,8,len(stored))
        arc['changes'].append({**entry,'row_offset':48+20*entry['index'],'new_row':row.hex(),'new_offset':cursor,'stored_file':stored_name,'stored_sha256':sha(stored),'decoded_sha256':sha(data),'original_decoded_sha256':sha(decoded),'decoded_file':replacement.name})
        cursor+=len(stored)
    arc['new_size']=cursor;nh=bytearray(header);struct.pack_into('>Q',nh,24,cursor);arc['new_header']=nh.hex()
    manifest['archives'].append(arc)
    print('PREPARED',archive_name,oldsize,'->',cursor,flush=True)
(root/'package.json').write_text(json.dumps(manifest,indent=2))
print('PACKAGE_COMPLETE',root)
