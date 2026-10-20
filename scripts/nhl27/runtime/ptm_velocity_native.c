/* Query adapter for the six translated deke velocity schemas. Their drivers,
 * update overrides and slot selectors are null; reject other schema shapes.
 * The caller owns and initializes third_cache (native size 16c0), supplies the
 * retained previous snapshot, and keeps both alive through query completion. */
#include "ptm_velocity_query.h"
#include "ptm_previous_pose.h"
#include "ptm_feature_pose.h"
typedef panic_address A;
typedef panic_u8 B;
#define ABI PTM_GAME_ABI
extern int nhl24_ptm_history_pose(void*,void*,int,float*,float*);
extern ABI void nhl24_ptm_feature_transform_guest(A,float*,void*,void*,int,B);
struct velocity_context {A base;B *schema,*query;void *third_pose,*third_cache,*history;};
static void *record_zero(void *map,unsigned stride){
    B *m=map;int index=*(int*)m;
    if(index==-1)return 0;
    B *record=*(B**)(m+0x10)+(A)index*stride;
    return record==*(B**)(m+0x18)?0:record;
}
static int override(void *ctx,struct ptm_velocity_schema *schema){(void)ctx;(void)schema;return 0;}
static int mode(void *ctx,struct ptm_velocity_schema *schema){
    (void)schema;struct velocity_context *c=ctx;
    return ((int(ABI*)(void*))(*(A**)c->schema)[13])(c->schema);
}
static int joint_id(void *ctx,void *slot,void *joint){
    struct velocity_context *c=ctx;
    return ((int(ABI*)(void*,void*,void*))(c->base+0x2c8410))(*(void**)(c->query+0x70),slot,joint);
}
static void previous_get(void *ctx,void *slot,int joint,B future,float *out,float *dt){
    (void)slot;struct velocity_context *c=ctx;
    if(future){
        for(unsigned i=0;i<12;i++)out[i]=0;
        out[0]=out[1]=out[2]=out[3]=out[7]=1;*dt=2;
        if(c->third_pose)nhl24_ptm_feature_transform_guest(c->base,out,c->third_cache,
            c->third_pose,joint,c->query[0x38]);
    }else{
        B *record=record_zero(*(void**)(c->query+0x30),16);
        void *owner=record?*(void**)(record+8):0;
        if(owner)while(*(void**)((B*)owner+8))owner=*(void**)((B*)owner+8);
        nhl24_ptm_history_pose(c->history,owner,joint,out,dt);
    }
}
static int previous_root(void *ctx,void *slot){
    (void)slot;struct velocity_context *c=ctx;
    B *record=record_zero(*(void**)(c->query+0x70),0x8a0);
    if(!record)return -1;
    return **(int**)(*(B**)(record+0x890)+0x98);
}
static int previous(void *ctx,void *slot,int joint,B jf,void *ref,int reference,B rf,
        float *out,float *dt,B *flags){
    struct velocity_context *c=ctx;
    void *property=*(void**)(*(B**)(c->query+0x40)+8);
    B *records=*(B**)(*(B**)(c->query+0x30)+0x10);
    void **history=((void**(ABI*)(void*,void*))(*(A**)property)[15])(property,*(void**)(records+8));
    c->history=*history;
    if(c->history)c->history=((void*(ABI*)(void*,unsigned))(*(A**)c->history)[2])(c->history,0xacfb5852);
    struct ptm_previous_pose_services services={ctx,previous_get,previous_root};
    return nhl27_ptm_previous_triplet(slot,joint,jf,ref,reference,rf,out,dt,flags,&services);
}
static void *current_pose(struct velocity_context *c,B future){
    B *record=record_zero(*(void**)(c->query+0x60+(future?8:0)),16);
    return record?*(void**)(record+8):0;
}
static int current(void *ctx,void *slot,int joint,B jf,void *ref,int reference,B rf,float *out,B *flags){
    struct velocity_context *c=ctx;void *a=current_pose(c,jf),*b=current_pose(c,rf);
    if((joint|reference)<0 || !a || !b)return 0;
    typedef void*(ABI *cache_fn)(void*,void*);
    cache_fn cache=(cache_fn)(c->base+0x330f80);
    void *ac=cache(c->query+(jf?0x5bc0:0x78),slot);
    void *bc=cache(c->query+(rf?0x5bc0:0x78),ref);
    return nhl24_ptm_feature_triplet_guest(c->base,a,b,ac,bc,joint,reference,c->query[0x38],out,flags);
}
PTM_EXPORT ABI int nhl24_ptm_velocity_query_guest(A base,void *primary,void *query,
        void *third_pose,void *third_cache){
    B *p=primary,*q=query;
    if(*(void**)(p+0x18)||*(void**)(p+0x20)||*(void**)(p+0x30)||*(void**)(p+0x40))return 0;
    struct velocity_context ctx={base,p,q,third_pose,third_cache,0};
    struct ptm_velocity_schema schema={*(void**)(p+0x10),0,*(void**)(p+0x38),0,
        *(unsigned*)(p+0x60),{p[0x28],p[0x29],p[0x2a]}};
    struct ptm_velocity_query request={&ctx,*(float**)(q+0x50),*(float**)(q+0x58),q[0x39],
        override,mode,joint_id,previous,current};
    nhl27_ptm_velocity_query(&schema,&request);
    return 1;
}
