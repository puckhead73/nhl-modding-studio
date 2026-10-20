"""Bounded offline DbObject reader, matching Studio's tdb-frostbite/db.rs types.

Usage: python scripts/nhl27/dbobject.py INPUT OUTPUT_JSON
Opaque blob contents are summarized, not treated as decoded files.
"""
import hashlib
import json
import struct
import sys
from pathlib import Path


class Reader:
    def __init__(self,b,keep_blobs=False):self.b=b;self.p=0;self.keep_blobs=keep_blobs
    def take(self,n):
        if n<0 or self.p+n>len(self.b):raise ValueError('DbObject range exceeds input')
        b=self.b[self.p:self.p+n];self.p+=n;return b
    def length(self):
        n=0
        for shift in range(0,35,7):
            v=self.take(1)[0]
            if shift==28 and v>15:raise ValueError('LEB128 overflow')
            n|=(v&127)<<shift
            if not v&128:return n
        raise ValueError('Unterminated LEB128')
    def entry(self,depth=0):
        if depth>64:raise ValueError('Nesting limit')
        t=self.take(1)[0]
        if t==0:return None
        name=''
        if not t&128:
            end=self.b.find(b'\0',self.p)
            if end<self.p:raise ValueError('Unterminated key')
            name=self.take(end-self.p+1)[:-1].decode('utf8')
        kind=t&31
        if kind in (1,2):
            sub=Reader(self.take(self.length()),self.keep_blobs);v=[] if kind==1 else {};term=False
            while sub.p<len(sub.b):
                entry=sub.entry(depth+1)
                if entry is None:term=True;break
                k,item=entry
                if kind==1:v.append(item)
                else:
                    if k in v:raise ValueError('Duplicate key')
                    v[k]=item
            if not term or sub.p!=len(sub.b):raise ValueError('Container consumption mismatch')
        elif kind==6:
            n=self.take(1)[0]
            if n>1:raise ValueError('Invalid bool')
            v=bool(n)
        elif kind==7:
            b=self.take(self.length())
            if not b.endswith(b'\0'):raise ValueError('Unterminated string')
            v=b[:-1].decode('utf8')
        elif kind in (8,9,11,12):
            fmt={8:'<i',9:'<q',11:'<d',12:'<d'}[kind];v=struct.unpack(fmt,self.take(struct.calcsize(fmt)))[0]
        elif kind in (15,16):v=self.take(16 if kind==15 else 20).hex()
        elif kind==19:
            b=self.take(self.length());v=b if self.keep_blobs else {'blob_size':len(b),'sha256':hashlib.sha256(b).hexdigest()}
        else:raise ValueError(f'Unsupported DbObject type {kind}')
        return name,v


def parse(path,keep_blobs=False):
    b=Path(path).read_bytes()
    if b[:4]==bytes.fromhex('00d1ce01'):b=b[0x22c:]
    elif b[:3]==bytes.fromhex('00d1ce'):raise ValueError('Unsupported wrapper')
    r=Reader(b,keep_blobs);v=r.entry()
    if not v or r.p!=len(b):raise ValueError('Trailing or empty DbObject')
    return v[1]


if __name__=='__main__':Path(sys.argv[2]).write_text(json.dumps(parse(sys.argv[1]),indent=2))
