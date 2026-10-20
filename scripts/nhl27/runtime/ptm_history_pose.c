/* Adapt source 2024f0's past-pose lookup to the native pose-history cache.
 * Native 331a50 qualifies entry+8 (latest map), +38 (sample count), +45d8
 * (interval), +45f0 (hash link), and transform-node+10/+40. */
#include "ptm_native.h"
typedef panic_u8 B;
typedef panic_address A;
PTM_EXPORT int nhl24_ptm_history_pose(void *history,void *owner,int joint,float *out,float *dt){
    out[0]=out[1]=out[2]=out[3]=1.0f;
    out[4]=out[5]=out[6]=0.0f;out[7]=1.0f;
    out[8]=out[9]=out[10]=out[11]=0.0f;*dt=2.0f;
    if(!history)return 0;
    /* Source uses a distinct identity constant when a live history misses. */
    out[3]=0.0f;
    if(!owner)return 0;
    B *h=history;unsigned count=*(unsigned*)(h+0x18);
    if(!count)return 0;
    B **buckets=*(B***)(h+0x10),*sentinel=buckets[count];
    B *entry=buckets[(A)owner%count];
    while(entry && entry!=sentinel && *(void**)entry!=owner)entry=*(B**)(entry+0x45f0);
    if(!entry || entry==sentinel || !*(int*)(entry+0x38))return 0;
    B *map=*(B**)(entry+8);
    unsigned joints=*(unsigned*)(map+0x20);
    if(!joints)return 0;
    B **nodes=*(B***)(map+0x18),*end=nodes[joints];
    B *node=nodes[(A)(__INTPTR_TYPE__)joint%joints];
    while(node && node!=end && *(int*)node!=joint)node=*(B**)(node+0x40);
    if(!node || node==end)return 0;
    for(unsigned lane=0;lane<12;lane++)out[lane]=((float*)(node+0x10))[lane];
    *dt=*(float*)(entry+0x45d8);
    return 1;
}
