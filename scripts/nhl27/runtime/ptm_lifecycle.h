#ifndef NHL24_PTM_LIFECYCLE_H
#define NHL24_PTM_LIFECYCLE_H
#include "ptm_native.h"
#define PTM_NATIVE_MATCH_SIZE 0x1740u
#define PTM_EXTENDED_MATCH_SIZE 0x17d0u
#define PTM_EXTENSION_MAGIC 0x37544d5054524f50ull
struct ptm_match_extension {
    panic_address magic;
    struct ptm_transition *transition;
    panic_u8 trajectory_flag,enabled,extra_path_valid,padding[13];
    panic_u8 extra_path[0x70];
};
_Static_assert(sizeof(struct ptm_match_extension)==0x90,"PTM allocation extension");
_Static_assert(__builtin_offsetof(struct ptm_match_extension,extra_path)==0x20,"PTM path alignment");
#endif
