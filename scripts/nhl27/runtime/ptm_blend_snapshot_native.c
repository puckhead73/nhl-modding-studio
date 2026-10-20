/* Native services for source PTMBlend snapshot generation. The caller supplies
 * its live evaluation context and the translated asset's velocity mode. */
#include "ptm_blend_snapshot.h"
typedef panic_address A;
typedef panic_u8 B;
#define ABI PTM_GAME_ABI
struct snapshot_scope {void *arena;A handle;void *evaluation[3];B flag,padding[7];};
struct snapshot_context {A base;B *blend,*master;void *context,*tls;struct snapshot_scope scope;};
extern void nhl24_ptm_pose_retain(void*);
extern void nhl24_ptm_pose_release(void*);
extern void nhl24_ptm_offset_scope_begin(struct snapshot_scope*,void*,void*,A);
extern void nhl24_ptm_offset_scope_end(struct snapshot_scope*,void*,void(ABI*)(void*));
extern ABI A nhl24_ptm_pose_import_guest(A,void*,void*);
extern ABI A nhl24_ptm_pose_combine_guest(A,void*,void*,A,A);
static void *clone(void *ctx,void *pose){
    struct snapshot_context *s=ctx;
    return ((void*(ABI*)(void*,int))(s->base+0x8bb990))(pose,0);
}
static void retain(void *ctx,void *pose){(void)ctx;nhl24_ptm_pose_retain(pose);}
static void release(void *ctx,void *pose){(void)ctx;nhl24_ptm_pose_release(pose);}
static void begin(void *ctx){
    struct snapshot_context *s=ctx;
    s->tls=((void*(ABI*)(void*))(s->base+0x89cd40))(*(void**)(s->base+0xac89f98));
    nhl24_ptm_offset_scope_begin(&s->scope,s->tls,s->context,*(A*)(s->base+0xbe14630));
}
static void end(void *ctx){
    struct snapshot_context *s=ctx;
    nhl24_ptm_offset_scope_end(&s->scope,s->tls,(void(ABI*)(void*))(s->base+0x89ec10));
}
static A evaluate(void *ctx,float delta){
    struct snapshot_context *s=ctx;
    return ((A(ABI*)(void*,void*,float))(*(A**)s->master)[11])(s->master,s->scope.evaluation,delta);
}
static float time(void *ctx){struct snapshot_context *s=ctx;return *(float*)(s->master+0x1c);}
static void seek(void *ctx,float value){
    struct snapshot_context *s=ctx;void *interface=s->master+0x10;
    ((void(ABI*)(void*,float))(*(A**)interface)[2])(interface,value);
}
static void flush(void *ctx,A handle,void *pose){
    struct snapshot_context *s=ctx;s->scope.handle=handle;
    ((void(ABI*)(void*,void*))(s->base+0x8a0a90))(&s->scope,pose);
}
static A import_pose(void *ctx,void *pose){
    struct snapshot_context *s=ctx;
    return nhl24_ptm_pose_import_guest(s->base,s->scope.evaluation,pose);
}
static A combine(void *ctx,A a,A b){
    struct snapshot_context *s=ctx;
    return nhl24_ptm_pose_combine_guest(s->base,s->scope.evaluation,s->blend,a,b);
}
static float *root(void *ctx,void *pose){
    (void)ctx;B *p=pose,*metadata=*(B**)(p+0x20);
    unsigned offset=*(unsigned short*)(p+0x1c);
    B *data=offset?p+offset:(B*)0;
    return (float*)(data+*(unsigned*)(metadata+0xf4));
}
PTM_EXPORT ABI void nhl24_ptm_blend_snapshots_guest(A base,void *blend,void *evaluation,
    int velocity_mode,B force_update){
    struct snapshot_context context={base,blend,*(B**)((B*)blend+0x2b8),((void**)evaluation)[1],0,{0}};
    struct ptm_blend_snapshot_services services={{&context,clone,retain,release},
        begin,end,evaluate,time,seek,flush,import_pose,combine,root};
    nhl27_ptm_blend_snapshots((void*)((B*)blend+PTM_BLEND_EXTENSION_OFFSET),
        *(void**)((B*)blend+0x30),velocity_mode,(*(B**)((B*)blend+0x2a8))[0x48],force_update,&services);
}
