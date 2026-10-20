#ifndef NHL27_PTM_VELOCITY_QUERY_H
#define NHL27_PTM_VELOCITY_QUERY_H
#include "ptm_native.h"
struct ptm_velocity_schema {
    void *joint,*joint_slot,*reference,*reference_slot;
    unsigned pose_time;
    panic_u8 stripped[3];
};
struct ptm_velocity_query {
    void *context;
    float *out;
    const float *output_transform;
    panic_u8 same_slot_zero;
    int (*override)(void*,struct ptm_velocity_schema*);
    int (*mode)(void*,struct ptm_velocity_schema*);
    int (*joint_id)(void*,void*,void*);
    int (*previous)(void*,void*,int,panic_u8,void*,int,panic_u8,float*,float*,panic_u8*);
    int (*current)(void*,void*,int,panic_u8,void*,int,panic_u8,float*,panic_u8*);
};
PTM_EXPORT void nhl27_ptm_velocity_query(struct ptm_velocity_schema*,struct ptm_velocity_query*);
#endif
