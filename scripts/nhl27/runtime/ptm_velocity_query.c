/* Source 24b1d0 dispatch, retaining callback order and exact boolean gates. */
#include "ptm_velocity_query.h"
extern PTM_GAME_ABI void nhl27_ptm_velocity_math(float*,const float*,const panic_u8*,float);
extern void nhl27_ptm_velocity_output(float*,const float*,const panic_u8*,const float*,int);
PTM_EXPORT void nhl27_ptm_velocity_query(struct ptm_velocity_schema *schema,struct ptm_velocity_query *q){
    int write=!q->override(q->context,schema);
    if(write && q->same_slot_zero==1 && schema->joint_slot==schema->reference_slot &&
        !q->mode(q->context,schema))write=0;
    float velocity[4]={0,0,0,0};
    if(write){
        int joint=q->joint_id(q->context,schema->joint_slot,schema->joint);
        if(joint!=-1){
            int reference=q->joint_id(q->context,schema->reference_slot,schema->reference);
            if(reference!=-1){
                float dt=2.0f,transforms[72];panic_u8 old_flags[2],new_flags[2];
                panic_u8 jf=schema->pose_time-1u<2u,rf=(schema->pose_time&~2u)==1u;
                if(q->previous(q->context,schema->joint_slot,joint,jf,schema->reference_slot,
                    reference,rf,transforms,&dt,old_flags)){
                    /* Source re-reads PoseTime after the previous-pose callback. */
                    jf=schema->pose_time-1u<2u;rf=(schema->pose_time&~2u)==1u;
                    if(q->current(q->context,schema->joint_slot,joint,jf,schema->reference_slot,
                        reference,rf,transforms+36,new_flags)){
                        panic_u8 flags[3]={old_flags[1],new_flags[0],new_flags[1]};
                        nhl27_ptm_velocity_math(velocity,transforms,flags,dt);
                    }
                }
            }
        }
    }
    nhl27_ptm_velocity_output(q->out,velocity,schema->stripped,q->output_transform,write);
}
