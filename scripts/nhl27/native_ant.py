"""Bounded version-6 ANT memory-image inspection for the supplied NHL24 banks.

Layout traced through NHL24 1.60 loader 0xad74b0. No runtime pointer fixups
are applied: relocation pairs remain offsets into the serialized data image.
"""
import struct
from animation_dct import take
from animation_stage import bank_ids


class Bank:
    def __init__(self,data):
        bank_ids(data,allow_external=True);self.data=data
        self.header=struct.unpack_from('>I',data)[0]
        h=struct.unpack('<12Q',take(data,self.header,96))
        version,nt,ts,nh,hs,nr,rs,no,os,ne,ni,nb=h
        if version!=6 or nt!=nh or (ts,hs,rs,os)!=(8,4,16,24):
            raise ValueError('Unsupported ANT memory-image layout')
        if no!=struct.unpack_from('>I',data,8)[0]:raise ValueError('Outer/inner object count mismatch')
        p=self.header+96;self.sections={}
        for name,count,width in [('sizes',nt,8),('hashes',nh,4),('relocations',nr,16),
            ('objects',no,24),('external',ne,32),('indices',ni,4),('names',no,8),('blob',nb,1)]:
            self.sections[name]=(p,count,width);take(data,p,count*width);p+=count*width
        if p!=len(data):raise ValueError('Memory image does not end at resource boundary')
        self.blob=self.raw('blob');self.sizes=list(self.values('sizes','<Q'))
        self.hashes=list(self.values('hashes','<I'));self.relocations=list(self.values('relocations','<QQ'))
        self.objects=list(self.values('objects','<QQQ'));self.external=list(self.values('external','<qQQII'))
        self.names=list(self.values('names','<Q'));self.by_id={}
        self.reloc={}
        for src,dst in self.relocations:
            take(self.blob,src,8);take(self.blob,dst,0)
            if src in self.reloc and self.reloc[src]!=dst:raise ValueError('Conflicting relocation')
            self.reloc[src]=dst
        for i,(offset,ident,typ) in enumerate(self.objects):
            if typ>=len(self.sizes):raise ValueError('Object type index outside table')
            take(self.blob,offset,self.sizes[typ])
            if ident in self.by_id:raise ValueError('Duplicate object ID')
            self.by_id[ident]=dict(index=i,offset=offset,type_index=typ,size=self.sizes[typ],
                type_hash=self.hashes[typ],name=self.string(self.names[i]))
        for index in self.values('indices','<I'):
            if index>=len(self.objects):raise ValueError('Object initialization index outside table')
        # The first external field is an optional owner offset; -1 means absent.
        for owner,site,ident,typ,flags in self.external:
            if owner!=-1:take(self.blob,owner,0)
            take(self.blob,site,8)

    def raw(self,name):
        p,n,w=self.sections[name];return take(self.data,p,n*w)

    def values(self,name,fmt):
        for row in struct.iter_unpack(fmt,self.raw(name)):
            yield row[0] if len(row)==1 else row

    def string(self,offset):
        if offset==2**64-1:return None
        take(self.blob,offset,1);end=self.blob.find(b'\0',offset,min(len(self.blob),offset+4096))
        if end<0:raise ValueError('Invalid bank object name')
        return self.blob[offset:end].decode('utf-8')

    def pointer(self,offset):
        if offset not in self.reloc:raise ValueError('Pointer has no serialized relocation')
        return self.reloc[offset]

    def clip(self,ident):
        obj=self.by_id[ident]
        if obj['size']!=72 or obj['type_hash']!=1556467357:raise ValueError('Not an observed 72-byte clip controller')
        anim=self.pointer(obj['offset']+32)
        candidates=[o for o in self.by_id.values() if o['offset']==anim]
        if len(candidates)!=1 or candidates[0]['size']!=104 or candidates[0]['type_hash']!=3842202299:raise ValueError('Not an observed DCT animation object')
        if struct.unpack_from('<I',self.blob,anim+16)[0]!=0x44435420:raise ValueError('Target is not DCT')
        keys,size=struct.unpack_from('<I',self.blob,anim+72)[0],struct.unpack_from('<I',self.blob,anim+88)[0]
        kt=take(self.blob,self.pointer(anim+64),keys*2) if keys else b''
        body=take(self.blob,self.pointer(anim+80),size)
        # Reconstruct only the source-style envelope to exercise the same codec reader.
        out=struct.pack('<5I',keys,32,0,size,32+len(kt))+bytes(12)+kt+body
        out+=bytes((-len(out))%16)+struct.pack('<2I',4,16)
        return obj,candidates[0],out
