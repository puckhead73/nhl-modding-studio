#ifndef NHL24_PTM_BLEND_QUERY_H
#define NHL24_PTM_BLEND_QUERY_H
#include "ptm_blend_storage.h"
struct ptm_blend_query_arguments {
    void *current_map,*future_map,*slot,*contexts,*input_state;
    unsigned first,second,third;
    float remaining,delta,scale;
};
struct ptm_blend_query_services {
    void *context;
    void *(*allocate)(void*);
    void (*construct)(void*,void*);
    void (*destroy)(void*,void*);
    void (*release)(void*,void*);
};
PTM_EXPORT int nhl24_ptm_blend_query(void*,const struct ptm_blend_query_arguments*,
    const struct ptm_blend_query_services*);
PTM_EXPORT PTM_GAME_ABI int nhl24_ptm_blend_query_guest(panic_address,void*,
    const struct ptm_blend_query_arguments*);
#endif
