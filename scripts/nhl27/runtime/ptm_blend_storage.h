#ifndef NHL24_PTM_BLEND_STORAGE_H
#define NHL24_PTM_BLEND_STORAGE_H
#include "ptm_blend_timing.h"
/* Separate from the native 2d8-byte object; allocation hooks must reserve it. */
#define PTM_BLEND_EXTENSION_OFFSET 0x2e0u
#define PTM_BLEND_EXTENDED_SIZE 0x330u
#define PTM_BLEND_MAGIC 0x37444e454c425450ull
struct ptm_blend_extension {
    panic_address magic;
    void *pose_a,*pose_b;
    float frame_delta;
    unsigned reserved;
    float target[4];
    struct ptm_blend_clock clock;
    panic_u8 initialized,enabled,retime,padding;
};
struct ptm_blend_pose_services {
    void *context;
    void *(*clone)(void*,void*);
    void (*retain)(void*,void*);
    void (*release)(void*,void*);
};
_Static_assert(sizeof(struct ptm_blend_extension)==0x50,"Blend allocation extension");
PTM_EXPORT void nhl24_ptm_blend_storage_init(struct ptm_blend_extension*,void*,float,float,float,
    const struct ptm_blend_pose_services*);
PTM_EXPORT void nhl24_ptm_blend_storage_capture_a(struct ptm_blend_extension*,void*,
    const struct ptm_blend_pose_services*);
PTM_EXPORT void nhl24_ptm_blend_storage_destroy(struct ptm_blend_extension*,
    const struct ptm_blend_pose_services*);
#endif
