# Adapted from read-only Studio scripts/nhl19_build_test_package.py
import numpy as np

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
