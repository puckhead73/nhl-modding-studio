/* Source 1f9ab0 uses the reference pose's interval and rig root for both
 * root flags. Current-pose queries deliberately use different root lookup. */
#include "ptm_previous_pose.h"
PTM_EXPORT int nhl27_ptm_previous_triplet(void *joint_slot,int joint,panic_u8 joint_future,
    void *reference_slot,int reference,panic_u8 reference_future,float *out,float *dt,
    panic_u8 *root_flags,const struct ptm_previous_pose_services *s){
    float ignored=0;
    s->get(s->context,joint_slot,joint,joint_future,out,&ignored);
    s->get(s->context,reference_slot,reference,reference_future,out+12,dt);
    if(!(*dt>=0x1p-23f))return 0;
    int root=s->root_id(s->context,reference_slot);
    root_flags[0]=joint==root;root_flags[1]=reference==root;
    if(reference==root){
        for(unsigned i=0;i<12;i++)out[24+i]=0;
        out[24]=out[25]=out[26]=out[27]=out[31]=1.0f;
    }else s->get(s->context,reference_slot,root,reference_future,out+24,&ignored);
    return 1;
}
