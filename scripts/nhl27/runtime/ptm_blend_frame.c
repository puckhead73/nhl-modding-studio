/* Source query-before-advance setup, adapted to native 369520's frame. */
#include "ptm_blend_storage.h"
#include "ptm_lifecycle.h"
#include <xmmintrin.h>
typedef panic_address A;
typedef panic_u8 B;
#define ABI PTM_GAME_ABI
extern ABI void nhl24_ptm_blend_snapshots_guest(A,void*,void*,int,B);
extern void nhl24_ptm_pose_retain(void*);
extern ABI void nhl24_ptm_blend_warp_guest(A,void*,void*,void*,float);
extern ABI void nhl24_ptm_start_transition_guest(A,struct ptm_transition**,void*,void*,const void*,void*,const float*,float,const B*,void*);
PTM_EXPORT ABI void nhl24_ptm_blend_begin_frame_guest(A base,void *blend,void *evaluation,
        void *native_frame,float delta){
    B *b=blend,*frame=native_frame;
    struct ptm_blend_extension *e=(void*)(b+PTM_BLEND_EXTENSION_OFFSET);
    e->frame_delta=delta;
    /* The sole translated PtmBlendAsset has VelocityMode=0, flag315=0,
     * MinBlendTime=0 and no future-facing/position overrides. */
    if(!e->initialized)nhl24_ptm_blend_snapshots_guest(base,blend,evaluation,0,0);
    float nominal=e->clock.nominal;
    if(nhl27_ptm_blend_progress(e->clock.duration,e->clock.elapsed)<=0x1p-23f){
        void *override=*(void**)(*(B**)(b+0x2a8)+0x30);
        if(override)nominal=((float(ABI*)(void*,void*))(*(A**)override)[3])(override,((void**)evaluation)[1]);
    }
    (void)nhl27_ptm_blend_begin(&e->clock,nominal);
    *(float*)(b+0x2b0)=e->clock.duration;*(float*)(b+0x2b4)=e->clock.elapsed;
    *(float*)(b+0x2c8)=e->clock.blend_in;*(float*)(b+0x2cc)=e->clock.blend_out;
    *(float*)(frame-0x350)=delta;
    *(float*)(frame-0x380)=e->clock.elapsed;
    *(float*)(frame-0x354)=e->clock.duration;
    *(float*)(frame-0x364)=e->clock.elapsed;
    *(void**)(frame-0x340)=((void**)evaluation)[1];
    *(void**)(frame-0x3b0)=evaluation;
    /* Balance the native update's existing per-frame release at its exit. */
    nhl24_ptm_pose_retain(e->pose_b);
    *(void**)(frame-0x360)=e->pose_b;
}

/* The original feature decoder has already produced XMM0 at 36a01e. */
PTM_EXPORT ABI void nhl24_ptm_blend_speed_frame_guest(A base,void *blend,void *native_frame,float feature){
    (void)base;
    B *b=blend,*frame=native_frame;
    struct ptm_blend_extension *e=(void*)(b+PTM_BLEND_EXTENSION_OFFSET);
    void *match=*(void**)(b+0x2c0);
    float remaining=*(float*)(frame-0x380);
    e->clock.feature_ticks=feature;
    e->clock.speed=nhl27_ptm_blend_feature_speed(feature,remaining);
    *(float*)(b+0x2d0)=e->clock.speed;
    nhl24_ptm_blend_set_speed(match,*(void**)(frame-0x340),e->clock.speed);
    if(e->retime==1)remaining=nhl27_ptm_blend_retime(&e->clock,feature,*(float*)((B*)match+0x88));
    *(float*)(frame-0x380)=remaining;
    *(float*)(b+0x2b0)=e->clock.duration;*(float*)(b+0x2b4)=e->clock.elapsed;
}

/* Source copies child timing only after evaluation; advancement and parent
 * remaining-time consumption follow it, with original MINSS operand order. */
PTM_EXPORT ABI void nhl24_ptm_blend_finish_frame_guest(A base,void *blend){
    (void)base;B *b=blend,*child=*(B**)(b+0x50);
    struct ptm_blend_extension *e=(void*)(b+PTM_BLEND_EXTENSION_OFFSET);
    for(unsigned n=0;n<3;n++)((A*)(b+0x18))[n]=((A*)(child+0x18))[n];
    nhl27_ptm_blend_advance(&e->clock,e->frame_delta);
    *(float*)(b+0x2b4)=e->clock.elapsed;
    float consumed=_mm_cvtss_f32(_mm_min_ss(_mm_set_ss(e->clock.duration-e->clock.elapsed),
        _mm_set_ss(*(float*)(b+0x1c))));
    *(float*)(b+0x1c)-=consumed;*(float*)(b+0x24)=consumed+*(float*)(b+0x24);
}

PTM_EXPORT ABI void nhl24_ptm_blend_pre_child_guest(A base,void *blend,void *native_frame){
    B *b=blend,*frame=native_frame,*match=*(B**)(b+0x2c0);
    struct ptm_blend_extension *e=(void*)(b+PTM_BLEND_EXTENSION_OFFSET);
    struct ptm_match_extension *m=(void*)(match+PTM_NATIVE_MATCH_SIZE);
    float remaining=*(float*)(frame-0x380);
    void *extra=m->extra_path_valid?m->extra_path:0;
    nhl24_ptm_blend_warp_guest(base,match,*(void**)(frame-0x340),extra,remaining);
    if(remaining<=e->clock.blend_out&&!m->transition)
        nhl24_ptm_start_transition_guest(base,&m->transition,match,*(void**)(frame-0x3b0),
            match+0x618,*(void**)(b+0x2b8),e->target,remaining,&m->trajectory_flag,extra);
}
