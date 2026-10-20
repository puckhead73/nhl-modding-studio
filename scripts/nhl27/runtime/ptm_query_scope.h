#ifndef NHL24_PTM_QUERY_SCOPE_H
#define NHL24_PTM_QUERY_SCOPE_H
#include "ptm_native.h"
/* Caller-owned scope. close waits for dispatched readers before returning, so
 * snapshots and cache storage may be released immediately after close. */
struct ptm_query_scope {
    struct ptm_query_scope *next;
    void *current_map,*future_map,*third_pose,*third_cache;
    unsigned users;
};
PTM_EXPORT int nhl24_ptm_query_scope_open(struct ptm_query_scope*,void*,void*,void*,void*);
PTM_EXPORT void nhl24_ptm_query_scope_close(struct ptm_query_scope*);
PTM_EXPORT PTM_GAME_ABI int nhl24_ptm_velocity_dispatch_guest(panic_address,void*,void*);
#endif
