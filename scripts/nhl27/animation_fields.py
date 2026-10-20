"""Read named source EBX animation fields using exact recovered type signatures.

Unknown field kinds remain explicit; this reader is not an EBX serializer.
"""
import struct
from animation_dct import take
from riff_evidence import fixup


class Fields:
    def __init__(self,data,types):
        self.data=data;self.fixup=fixup(data,True);self.types=types
        self.by_guid={x['guid']:x for x in types};self.local=list(self.fixup['types'])
        if data[12:16]!=b'EBXD':raise ValueError('Unsupported EBXD placement')
        self.end=20+self.fixup['chunk_sizes']['EBXD'];self.nodes={}
        for off in self.fixup['instance_offsets']:
            at=32+off;idx=self.num(at,'H');guid=self.local[idx];t=self.by_guid[guid]
            if t['signature']!=self.fixup['types'][guid]:raise ValueError('Shared schema mismatch')
            self.nodes[at]=t

    def read(self,at,n):
        if at<32 or at+n>self.end:raise ValueError('Field outside EBXD')
        return take(self.data,at,n)

    def num(self,at,fmt):return struct.unpack('<'+fmt,self.read(at,struct.calcsize('<'+fmt)))[0]

    def pointer(self,at):
        raw=self.num(at,'Q')
        if not raw:return None
        # EBX pointer slots are eight bytes, but internal relative offsets are
        # signed 32-bit values stored with a zero upper word (including negatives).
        if raw>>32:raise ValueError('Unsupported nonzero EBX pointer upper word')
        if raw&1:
            i=(raw&0xffffffff)>>1
            if i>=len(self.fixup['imports']):raise ValueError('Import index outside table')
            return dict(external=self.fixup['imports'][i])
        relative=raw-(1<<32) if raw&0x80000000 else raw
        target=at+relative
        if target not in self.nodes:raise ValueError('Internal pointer does not reference an instance')
        return dict(internal_offset=target-32)

    def value(self,at,flags,ref,depth):
        kind=(flags>>5)&31
        formats={8:'i',10:'?',11:'b',12:'B',13:'h',14:'H',15:'i',16:'I',17:'q',18:'Q',19:'f',20:'d',23:'Q'}
        if kind in formats:return self.num(at,formats[kind]),struct.calcsize(formats[kind])
        if kind==3:return self.pointer(at),8
        if kind==7:
            off=self.num(at,'q')
            if off==-1:return '',8
            p=at+off;self.read(p,1);end=self.data.find(b'\0',p,min(self.end,p+4096))
            if end<0:raise ValueError('Unterminated string')
            return self.read(p,end-p).decode('utf-8'),8
        if kind==2 and ref!=65535:
            t=self.types[ref];return self.object(at,t,depth+1),t['size']
        return dict(unsupported_kind=kind,offset=at-32),0

    def object(self,at,t,depth=0):
        if depth>24:raise ValueError('Type inheritance/structure recursion limit')
        self.read(at,t['size']);out={}
        for f in t['fields']:
            flags=int(f['flags'],16);p=at+f['offset'];ref=f['type_ref']
            name=f['name_candidates'][0] if len(f['name_candidates'])==1 else f['name_hash']
            if flags==0 and ref!=65535:
                out.update(self.object(at,self.types[ref],depth+1));continue
            if flags&8:
                start=p+self.num(p,'i');count=self.num(start-4,'I')
                if count>100000:raise ValueError('Array count exceeds bound')
                values=[]
                for _ in range(count):
                    v,size=self.value(start,flags&~8,ref,depth)
                    if size==0:raise ValueError('Unknown array stride')
                    values.append(v);start+=size
                out[name]=values
            else:out[name]=self.value(p,flags,ref,depth)[0]
        return out

    def export(self):
        return dict(partition=self.fixup['partition'],instances=[dict(offset=at-32,
            type=t['name_candidates'],guid=t['guid'],fields=self.object(at,t)) for at,t in self.nodes.items()])

    def raw_array(self, instance_offset, name):
        """Preserve packed array bits, including signaling NaN payloads."""
        at = 32 + instance_offset

        def find(t, depth=0):
            if depth > 24:
                raise ValueError('Type inheritance recursion limit')
            for field in t['fields']:
                flags = int(field['flags'], 16)
                ref = field['type_ref']
                if flags == 0 and ref != 65535:
                    result = find(self.types[ref], depth + 1)
                    if result is not None:
                        return result
                label = field['name_candidates'][0] if len(field['name_candidates']) == 1 else field['name_hash']
                if label == name:
                    if not flags & 8:
                        raise ValueError('Requested field is not an array')
                    p = at + field['offset']
                    start = p + self.num(p, 'i')
                    count = self.num(start - 4, 'I')
                    if count > 100000:
                        raise ValueError('Array count exceeds bound')
                    kind = ((flags & ~8) >> 5) & 31
                    widths = {8: 4, 10: 1, 11: 1, 12: 1, 13: 2, 14: 2, 15: 4,
                              16: 4, 17: 8, 18: 8, 19: 4, 20: 8, 23: 8}
                    width = self.types[ref]['size'] if kind == 2 and ref != 65535 else widths.get(kind)
                    if width is None:
                        raise ValueError('Raw array has unsupported or pointer-bearing element kind')
                    return dict(count=count, stride=width, data=self.read(start, count * width))
            return None

        result = find(self.nodes[at])
        if result is None:
            raise KeyError(name)
        return result
