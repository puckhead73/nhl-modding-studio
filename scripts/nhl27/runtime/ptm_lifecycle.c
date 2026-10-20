/* Paired allocation/lifecycle hooks for port-owned match-controller storage. */
#include "ptm_lifecycle.h"
typedef panic_address A;
#define ABI PTM_GAME_ABI
typedef void(ABI *path_fn)(void*);
typedef void(ABI *clear_fn)(A,struct ptm_transition**);
extern ABI void nhl24_ptm_transition_clear_guest(A,struct ptm_transition**);
static struct ptm_match_extension *extension(void *match){
    return (struct ptm_match_extension*)((panic_u8*)match+PTM_NATIVE_MATCH_SIZE);
}

PTM_EXPORT void nhl24_ptm_extension_init(void *match,path_fn construct){
    struct ptm_match_extension *e=extension(match);
    for(unsigned n=0;n<sizeof(*e);n++)((panic_u8*)e)[n]=0;
    construct(e->extra_path);
    e->magic=PTM_EXTENSION_MAGIC;
}
PTM_EXPORT void nhl24_ptm_extension_reset(A base,void *match,clear_fn clear){
    struct ptm_match_extension *e=extension(match);
    if(e->magic==PTM_EXTENSION_MAGIC)clear(base,&e->transition);
}
PTM_EXPORT void nhl24_ptm_extension_destroy(A base,void *match,clear_fn clear,path_fn destroy){
    struct ptm_match_extension *e=extension(match);
    if(e->magic!=PTM_EXTENSION_MAGIC)return;
    clear(base,&e->transition);
    destroy(e->extra_path);
    e->magic=0;
}
PTM_EXPORT ABI void nhl24_ptm_match_initialize_guest(A base,void *match){
    nhl24_ptm_extension_init(match,(path_fn)(base+0x2b8840));
}
PTM_EXPORT ABI void nhl24_ptm_match_reset_guest(A base,void *match){
    nhl24_ptm_extension_reset(base,match,nhl24_ptm_transition_clear_guest);
}
PTM_EXPORT ABI void nhl24_ptm_match_destroy_guest(A base,void *match){
    nhl24_ptm_extension_destroy(base,match,nhl24_ptm_transition_clear_guest,(path_fn)(base+0x2a4e10));
}
