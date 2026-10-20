"""Private geometry leaves; initializer constants are checked before relocation."""
import panic_math

def geometry(image):
    # Source initializer 24fb1a0: zero XMM0, broadcast one/epsilon, store globals.
    checks={0x24fb1b1:'c5f857c0',0x24fb259:'c5f811050fc6f50c',
            0x24fb238:'c4e2791815e3d89d0b',0x24fb32a:'c5f811154ec5f50c',
            0x24fb489:'c4e279181546d49d0b',0x24fb4b3:'c5f8111515c4f50c'}
    for address,expected in checks.items():
        if image.read(address,len(bytes.fromhex(expected)))!=bytes.fromhex(expected):
            raise ValueError(f'Geometry initializer changed: {address:x}')
    constants={0xf457870:bytes(16),0xf457880:image.read(0xded8b24,4)*4,
               0xf4578d0:image.read(0xded88d8,4)*4}
    chunks=[];receipts={}
    for name,start,size,values in [('nhl27_segment_intersection',0x24f74a0,0x100,constants),
                                   ('nhl27_segment_overlap',0x69bc2c0,0xb0,{}),
                                   ('nhl27_lane_project',0x46f0960,0xc0,{}),
                                   ('nhl27_lane_distance',0x46f2ce0,0x6b0,{})]:
        code,receipt=panic_math.relocate(image,start,size,values)
        chunks.append(panic_math.assembly(code,name));receipts[name]=receipt
    receipts['initializer_constants']={hex(k):v.hex() for k,v in constants.items()}
    return '\n'.join(chunks),receipts
