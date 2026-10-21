"""Experimental Legacy SNORM10 XYZ shading; preserve top two bits and all positions."""
import numpy as np
import struct

def unpack(word):
    q=np.array([(word>>s)&1023 for s in (0,10,20)],float)
    return np.where(q>=512,q-1024,q)/511

def pack(vector, original):
    v=np.asarray(vector,float)
    if v.shape!=(3,) or not np.isfinite(v).all():raise ValueError('invalid shading vector')
    # Native Rust encoder rounds half away from zero; preserve opaque upper bits.
    q=np.sign(v)*np.floor(np.abs(np.clip(v,-1,1))*511+.5)
    a=[int(x)&1023 for x in q]
    return (original&0xc0000000)|a[0]|a[1]<<10|a[2]<<20

def regenerate(raw, layout, sub, original_positions):
    p=np.asarray(sub['positions'],float);tri=np.asarray(sub['triangles'],int);uv=np.asarray(sub['uv'],float)
    if p.shape!=(layout['count'],3) or uv.shape!=(len(p),2) or np.asarray(original_positions).shape!=p.shape:
        raise ValueError('record shape mismatch')
    if not np.isfinite(p).all() or not np.isfinite(uv).all():raise ValueError('nonfinite geometry')
    if tri.ndim!=2 or tri.shape[1]!=3 or np.any(tri<0) or np.any(tri>=len(p)):raise ValueError('invalid triangles')
    if layout['start']<0 or layout['start']+len(p)*layout['stride']>len(raw):raise ValueError('buffer range')
    slots=[]
    for tag in ('002a218700030003','002a218700060015'):
        found=[o for o,t in layout['attributes'] if t==tag]
        if len(found)!=1:raise ValueError('expected native normal and tangent declarations')
        slots.append(found[0])
    if slots!=[8,12] or layout['stride']!=32 or layout['position_offset']!=0:
        raise ValueError('unqualified native shading layout')
    words=[[struct.unpack_from('>I',raw,layout['start']+i*layout['stride']+off)[0] for i in range(len(p))] for off in slots]
    old=np.array([unpack(w) for w in words[0]])
    n=np.zeros_like(p);t=np.zeros_like(p)
    for a,b,c in tri:
        e1,e2=p[b]-p[a],p[c]-p[a];face=np.cross(e1,e2)
        n[[a,b,c]]+=face
        u,v=uv[b]-uv[a],uv[c]-uv[a];den=u[0]*v[1]-u[1]*v[0]
        if abs(den)>1e-12:t[[a,b,c]]+=(e1*v[1]-e2*u[1])/den
    groups={}
    for i in range(len(p)):
        key=(tuple(original_positions[i]),words[0][i],tuple(p[i]))
        groups.setdefault(key,[]).append(i)
    for ids in groups.values():n[ids]=n[ids].sum(0)
    lengths=np.linalg.norm(n,axis=1);fallback=lengths<1e-15;n[fallback]=old[fallback]
    n/=np.maximum(np.linalg.norm(n,axis=1)[:,None],1e-15)
    t-=n*np.sum(t*n,axis=1)[:,None]
    tf=np.linalg.norm(t,axis=1)<1e-15
    for i in np.flatnonzero(tf):t[i]=np.cross(n[i],(0,1,0) if abs(n[i,1])<.9 else (1,0,0))
    t/=np.maximum(np.linalg.norm(t,axis=1)[:,None],1e-15)
    moved=np.any(p!=np.asarray(original_positions),axis=1);active=np.zeros(len(p),bool)
    active[tri[np.any(moved[tri],axis=1)].reshape(-1)]=True
    for ids in groups.values():
        if active[ids].any():active[ids]=True
    output=bytearray(raw);allowed=np.zeros(len(raw),bool)
    for slot,vectors,orig in zip(slots,(n,t),words):
        for i in np.flatnonzero(active):
            off=layout['start']+i*layout['stride']+slot
            struct.pack_into('>I',output,off,pack(vectors[i],orig[i]));allowed[off:off+4]=True
    changes=np.frombuffer(raw,np.uint8)!=np.frombuffer(output,np.uint8)
    if np.any(changes&~allowed):raise ValueError('non-shading bytes changed')
    angle=np.degrees(np.arccos(np.clip(np.sum(n*old,axis=1)/np.maximum(np.linalg.norm(old,axis=1),1e-15),-1,1)))
    return bytes(output),{'changed_bytes':int(changes.sum()),'outside_shading_unchanged':True,'active_records':int(active.sum()),'normal_fallback':int(fallback.sum()),'tangent_fallback':int(tf.sum()),'normal_change_degrees_median':float(np.median(angle[active])) if active.any() else 0.,'normal_change_degrees_max':float(angle[active].max()) if active.any() else 0.,'top_bits_values':sorted(set(w>>30 for stream in words for w in stream)), 'limitations':['Mathematical UV tangent; native bitangent/shader convention unqualified','Geometry/lip intersections unchanged']}
