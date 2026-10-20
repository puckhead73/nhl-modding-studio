/* Own the extra native transform cache across a complete match query. */
#include "ptm_blend_query.h"
#include "ptm_query_scope.h"
typedef panic_address A;
#define ABI PTM_GAME_ABI
extern void nhl24_ptm_pose_retain(void*);
extern void nhl24_ptm_pose_release(void*);
PTM_EXPORT int nhl24_ptm_blend_query(void *blend,const struct ptm_blend_query_arguments *a,
        const struct ptm_blend_query_services *s){
    struct ptm_blend_extension *e=(void*)((panic_u8*)blend+PTM_BLEND_EXTENSION_OFFSET);
    if(e->magic!=PTM_BLEND_MAGIC || e->enabled!=1 || !e->initialized)return 0;
    typedef void(ABI *query_fn)(void*,void*,void*,void*,void*,void*,unsigned,unsigned,unsigned,float,float,float);
    void *cache=s->allocate(s->context);
    if(!cache)return 0;
    s->construct(s->context,cache);
    void *pose=e->pose_a;nhl24_ptm_pose_retain(pose);
    struct ptm_query_scope scope;
    int opened=nhl24_ptm_query_scope_open(&scope,a->current_map,a->future_map,pose,cache);
    if(opened){
        void *match=*(void**)((panic_u8*)blend+0x2c0);
        ((query_fn)(*(A**)match)[27])(match,a->current_map,a->future_map,a->slot,a->contexts,
            a->input_state,a->first,a->second,a->third,a->remaining,a->delta,a->scale);
        nhl24_ptm_query_scope_close(&scope);
    }
    nhl24_ptm_pose_release(pose);
    s->destroy(s->context,cache);
    s->release(s->context,cache);
    return opened;
}
struct native_services {A base;void *allocator;};
static void *allocate(void *ctx){
    struct native_services *s=ctx;
    typedef void*(ABI *fn)(void*,A,const char*,unsigned,unsigned,unsigned);
    return ((fn)(*(A**)s->allocator)[3])(s->allocator,0x16c0,"PTM source velocity scope",1,16,0);
}
static void construct(void *ctx,void *cache){
    struct native_services *s=ctx;((void(ABI*)(void*))(s->base+0x2df360))(cache);
}
static void destroy(void *ctx,void *cache){
    struct native_services *s=ctx;((void(ABI*)(void*))(s->base+0x2df470))(cache);
}
static void release(void *ctx,void *cache){
    struct native_services *s=ctx;
    ((void(ABI*)(void*,void*,A))(*(A**)s->allocator)[4])(s->allocator,cache,0x16c0);
}
PTM_EXPORT ABI int nhl24_ptm_blend_query_guest(A base,void *blend,
        const struct ptm_blend_query_arguments *a){
    struct native_services context={base,*(void**)(base+0xaa3dbb0)};
    struct ptm_blend_query_services services={&context,allocate,construct,destroy,release};
    return nhl24_ptm_blend_query(blend,a,&services);
}
