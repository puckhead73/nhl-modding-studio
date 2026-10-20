#ifndef NHL24_PTM_FEATURE_POSE_H
#define NHL24_PTM_FEATURE_POSE_H
#include "ptm_native.h"
struct ptm_feature_pose_services {
    void *context;
    void (*transform)(void*,float*,void*,void*,int,panic_u8);
    int (*root_id)(void*,void*);
};
PTM_EXPORT int nhl24_ptm_feature_triplet(void*,void*,void*,void*,int,int,panic_u8,
    float*,panic_u8*,const struct ptm_feature_pose_services*);
PTM_EXPORT PTM_GAME_ABI int nhl24_ptm_feature_triplet_guest(panic_address,void*,void*,void*,void*,
    int,int,panic_u8,float*,panic_u8*);
#endif
