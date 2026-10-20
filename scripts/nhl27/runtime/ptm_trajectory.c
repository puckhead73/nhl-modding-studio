/* Source root-transform arithmetic, scheduled through NHL24's pose arena. */
#include "ptm_native.h"
typedef panic_address A;
typedef panic_u8 B;
#define ABI PTM_GAME_ABI
extern ABI void nhl27_ptm_root_math(void *root,const void *parameters);
typedef void *(ABI *get_pose_fn)(void*,unsigned);
typedef A (ABI *finalize_fn)(void*,unsigned,void**);

/* NHL24 2d85a0 qualifies +f4/+f8 metadata and the +48 cache invalidation.
 * The 64-byte payload carries the image base with the job, not in a global. */
PTM_EXPORT void nhl24_ptm_trajectory_execute(void *job,get_pose_fn get_pose) {
    B *j=(B*)job;
    unsigned index=*(unsigned*)(j+0x28);
    B *pool=*(B**)(j+0x20);
    unsigned offset=**(unsigned**)(j+0x10);
    B *parameters=*(B**)(j+0x38)+offset;
    B *pose=get_pose(pool,index);
    B *metadata=*(B**)(pose+0x20);
    if(*(unsigned*)(metadata+0xf8)!=0x80000000u) {
        unsigned short data_offset=*(unsigned short*)(pose+0x1c);
        B *data=data_offset?pose+data_offset:(B*)0;
        nhl27_ptm_root_math(data+*(unsigned*)(metadata+0xf4),parameters);
    }
    *(A*)(pool+0x48+(A)index*16)=0;
}

PTM_EXPORT ABI void nhl24_ptm_trajectory_job(void *job) {
    B *j=(B*)job;
    B *parameters=*(B**)(j+0x38)+**(unsigned**)(j+0x10);
    A base=*(A*)(parameters+0x30);
    nhl24_ptm_trajectory_execute(job,(get_pose_fn)(base+0x89e9b0));
}

PTM_EXPORT A nhl24_ptm_trajectory_enqueue(A base,void *arena,A input,const float *movement,float weight,
                                         A invalid,A operation,finalize_fn finalize,A result_vtable) {
    if(!arena)return invalid;
    B *a=(B*)arena;
    A *cursor=(A*)(a+8);
    *(A*)(a+0x1008)=0;
    if(input!=invalid) {
        *cursor++=3;*cursor++=input;
        *(unsigned*)(a+0x100c)=*((B*)input+3);
    } else a[0x139c]=1;
    A *header=cursor;
    B *payload=(B*)(((A)header+31)&~(A)15);
    header[0]=1;
    header[1]=((A)payload+64-(A)(header+2))/8;
    for(unsigned n=0;n<4;n++)((float*)payload)[n]=movement[n];
    ((float*)payload)[4]=0;((float*)payload)[5]=0;((float*)payload)[6]=0;((float*)payload)[7]=1;
    ((float*)payload)[8]=weight;((float*)payload)[9]=0;
    *(A*)(payload+0x28)=0;*(A*)(payload+0x30)=base;*(A*)(payload+0x38)=0;
    cursor=header+2+header[1];
    *cursor++=5;*cursor++=0xffffffffu;*cursor++=operation;
    *(A**)(a+0x1000)=cursor;
    void *result_properties=0;
    A result=finalize(arena,8,&result_properties);
    *(A*)result_properties=result_vtable;
    return result;
}

PTM_EXPORT ABI A nhl24_ptm_trajectory_guest(A base,void *evaluation,A input,const float *movement,float weight) {
    return nhl24_ptm_trajectory_enqueue(base,*(void**)evaluation,input,movement,weight,
        *(A*)(base+0xbe14630),(A)nhl24_ptm_trajectory_job,(finalize_fn)(base+0x89f810),base+0xa419188);
}
