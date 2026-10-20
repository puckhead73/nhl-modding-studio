/* Source 1f9cf0's current-pose triplet with native pose/cache services. */
#include "ptm_feature_pose.h"
typedef panic_address A;
#define ABI PTM_GAME_ABI
static void identity(float *out){
    for(unsigned i=0;i<12;i++)out[i]=0.0f;
    for(unsigned i=0;i<4;i++)out[i]=1.0f;
    out[7]=1.0f;
}
PTM_EXPORT int nhl24_ptm_feature_triplet(void *joint_pose,void *reference_pose,
        void *joint_cache,void *reference_cache,int joint,int reference,panic_u8 mirrored,
        float *out,panic_u8 *root_flags,const struct ptm_feature_pose_services *s){
    if((joint|reference)<0 || !joint_pose || !reference_pose)return 0;
    /* Preserve callback order; metadata can change during evaluation. */
    s->transform(s->context,out,joint_cache,joint_pose,joint,mirrored);
    s->transform(s->context,out+12,reference_cache,reference_pose,reference,mirrored);
    int joint_root=s->root_id(s->context,joint_pose);
    int reference_root=s->root_id(s->context,reference_pose);
    root_flags[0]=(joint==joint_root);
    root_flags[1]=(reference==reference_root);
    if(root_flags[1])identity(out+24);
    else s->transform(s->context,out+24,reference_cache,reference_pose,reference_root,mirrored);
    return 1;
}
static int root_id(void *context,void *pose){
    (void)context;
    return **(int**)(*(panic_u8**)((panic_u8*)pose+0x20)+0x98);
}
static void transform(void *context,float *out,void *cache,void *pose,int joint,panic_u8 mirrored){
    A base=(A)context;float initial[12];identity(initial);
    /* Native 330960 uses the same two readers, with -1 as the stop joint.
     * Source 1f9cf0 admits only byte value 1 to the mirrored branch. */
    if(mirrored==1){
        void *meta=*(void**)((panic_u8*)pose+0x20);
        void *layout=((void*(ABI*)(void*))(base+0x2b4c30))(*(void**)((panic_u8*)meta+0x100));
        ((void(ABI*)(float*,void*,void*,int,int,void*,float*))(base+0x2df8c0))(
            out,cache,pose,joint,-1,layout,initial);
    }else ((void(ABI*)(float*,void*,void*,int,int,float*))(base+0x2dfb90))(
            out,cache,pose,joint,-1,initial);
}
PTM_EXPORT ABI void nhl24_ptm_feature_transform_guest(A base,float *out,void *cache,void *pose,
        int joint,panic_u8 mirrored){
    transform((void*)base,out,cache,pose,joint,mirrored);
}
PTM_EXPORT ABI int nhl24_ptm_feature_triplet_guest(A base,void *joint_pose,void *reference_pose,
        void *joint_cache,void *reference_cache,int joint,int reference,panic_u8 mirrored,
        float *out,panic_u8 *root_flags){
    const struct ptm_feature_pose_services services={(void*)base,transform,root_id};
    return nhl24_ptm_feature_triplet(joint_pose,reference_pose,joint_cache,reference_cache,
        joint,reference,mirrored,out,root_flags,&services);
}
