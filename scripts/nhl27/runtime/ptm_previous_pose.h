#ifndef NHL24_PTM_PREVIOUS_POSE_H
#define NHL24_PTM_PREVIOUS_POSE_H
#include "ptm_native.h"
struct ptm_previous_pose_services {
    void *context;
    void (*get)(void*,void*,int,panic_u8,float*,float*);
    int (*root_id)(void*,void*);
};
PTM_EXPORT int nhl27_ptm_previous_triplet(void*,int,panic_u8,void*,int,panic_u8,
    float*,float*,panic_u8*,const struct ptm_previous_pose_services*);
#endif
