#ifndef NHL24_PTM_BLEND_SNAPSHOT_H
#define NHL24_PTM_BLEND_SNAPSHOT_H
#include "ptm_blend_storage.h"
/* Query scope is owned by the caller; operation handles remain in that scope. */
struct ptm_blend_snapshot_services {
    struct ptm_blend_pose_services poses;
    void (*begin)(void*);
    void (*end)(void*);
    panic_address (*evaluate)(void*,float);
    float (*time)(void*);
    void (*seek)(void*,float);
    void (*flush)(void*,panic_address,void*);
    panic_address (*import_pose)(void*,void*);
    panic_address (*combine)(void*,panic_address,panic_address);
    float *(*root)(void*,void*);
};
PTM_EXPORT void nhl27_ptm_blend_snapshots(struct ptm_blend_extension*,void*,int,panic_u8,panic_u8,
    const struct ptm_blend_snapshot_services*);
#endif
