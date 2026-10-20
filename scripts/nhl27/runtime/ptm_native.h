#ifndef NHL24_PTM_NATIVE_H
#define NHL24_PTM_NATIVE_H
#include "ptm_transition.h"
#if defined(_WIN32)
#define PTM_GAME_ABI __attribute__((sysv_abi))
#else
#define PTM_GAME_ABI
#endif
struct ptm_mirror_scope {
    unsigned *word;
    unsigned saved;
    panic_u8 reset_count;
    panic_u8 padding[3];
};
struct ptm_native_clip_services {
    void (PTM_GAME_ABI *begin_mirror)(struct ptm_mirror_scope*,void*,panic_u8);
    void *(PTM_GAME_ABI *create_mirror)(void*,void*,void*);
};
PTM_EXPORT panic_address nhl24_ptm_sample_clip(void*,void*,void*,float,const panic_u8*,
                                             const struct ptm_native_clip_services*);
PTM_EXPORT void nhl24_ptm_auto_create(void*,void*,const void*,struct ptm_transition**,
    panic_address,panic_address,const struct ptm_creation_services*);
PTM_EXPORT int nhl24_ptm_auto_due(void*,const void*,float);
#endif
