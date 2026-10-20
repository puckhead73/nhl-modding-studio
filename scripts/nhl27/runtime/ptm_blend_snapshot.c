/* Source 283e60: preserve the two-tick velocity sample and master seek-back. */
#include "ptm_blend_snapshot.h"
#include <xmmintrin.h>
PTM_EXPORT void nhl27_ptm_blend_snapshots(struct ptm_blend_extension *e,void *pose,
    int velocity_mode,panic_u8 update_master,panic_u8 force_update,
    const struct ptm_blend_snapshot_services *s){
    void *ctx=s->poses.context;
    s->begin(ctx);
    int backward=0,advance=0;
    if(velocity_mode!=1){
        if(!force_update){
            if(update_master){
                if(e->clock.nominal>=2.0f)advance=update_master==1;
                else{(void)s->evaluate(ctx,e->clock.nominal);backward=1;}
            }else backward=1;
        }else if(e->clock.nominal>=2.0f)advance=1;
        else{(void)s->evaluate(ctx,e->clock.nominal);backward=1;}
    }else advance=force_update||update_master==1;
    if(backward){
        float time=s->time(ctx);
        panic_address handle=s->evaluate(ctx,0.0f);
        s->flush(ctx,handle,e->pose_b);
        panic_address prior=s->import_pose(ctx,e->pose_b);
        handle=s->evaluate(ctx,2.0f);
        handle=s->combine(ctx,handle,prior);
        handle=s->combine(ctx,prior,handle);
        nhl24_ptm_blend_storage_capture_a(e,pose,&s->poses);
        s->flush(ctx,handle,e->pose_a);
        float *a=s->root(ctx,e->pose_a),*b=s->root(ctx,e->pose_b);
        __m128 zero=_mm_setzero_ps();
        __m128 sign=_mm_castsi128_ps(_mm_set_epi32(0,(int)0x80000000u,
            (int)0x80000000u,(int)0x80000000u));
        /* Source negates with SUBPS, then XORs the first two vectors. */
        _mm_storeu_ps(b,_mm_xor_ps(_mm_sub_ps(zero,_mm_loadu_ps(a)),sign));
        _mm_storeu_ps(b+4,_mm_xor_ps(_mm_sub_ps(zero,_mm_loadu_ps(a+4)),sign));
        _mm_storeu_ps(b+12,_mm_sub_ps(zero,_mm_loadu_ps(a+12)));
        _mm_storeu_ps(b+16,_mm_sub_ps(zero,_mm_loadu_ps(a+16)));
        _mm_storeu_ps(b+20,_mm_sub_ps(zero,_mm_loadu_ps(a+20)));
        _mm_storeu_ps(e->target,_mm_mul_ps(_mm_sub_ps(zero,_mm_loadu_ps(a+12)),_mm_set1_ps(0.5f)));
        s->seek(ctx,time);
    }else{
        if(advance)(void)s->evaluate(ctx,e->clock.nominal-2.0f);
        panic_address handle=s->evaluate(ctx,0.0f);
        nhl24_ptm_blend_storage_capture_a(e,pose,&s->poses);
        s->flush(ctx,handle,e->pose_a);
        handle=s->evaluate(ctx,2.0f);
        s->flush(ctx,handle,e->pose_b);
        float *root=s->root(ctx,e->pose_b);
        _mm_storeu_ps(e->target,_mm_mul_ps(_mm_set1_ps(0.5f),_mm_loadu_ps(root+12)));
    }
    e->initialized=1;
    s->end(ctx);
}
