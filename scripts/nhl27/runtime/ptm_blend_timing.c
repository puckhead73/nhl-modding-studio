/* NHL27 PTMBlend's clock runs selection before advancing elapsed time. */
#include "ptm_blend_timing.h"
#include <xmmintrin.h>
PTM_EXPORT float nhl27_ptm_blend_progress(float duration,float elapsed){
    /* Source JBE also takes the nonpositive-duration branch for NaN. */
    if(!(duration>0.0f))return elapsed<0.0f?0.0f:1.0f;
    return _mm_cvtss_f32(_mm_min_ss(_mm_max_ss(_mm_setzero_ps(),
        _mm_set_ss(elapsed/duration)),_mm_set_ss(1.0f)));
}
PTM_EXPORT void nhl27_ptm_blend_rescale(float *duration,float *elapsed,float progress,float remaining){
    /* Original 69fcb0 preserves both fields at progress >=1 or unordered. */
    if(progress<1.0f){*duration=remaining/(1.0f-progress);*elapsed=*duration-remaining;}
}
PTM_EXPORT float nhl27_ptm_blend_begin(struct ptm_blend_clock *s,float nominal){
    float progress=nhl27_ptm_blend_progress(s->duration,s->elapsed);
    if(progress<=0x1p-23f){
        s->nominal=nominal;
        nhl27_ptm_blend_rescale(&s->duration,&s->elapsed,progress,nominal);
        s->blend_in=_mm_cvtss_f32(_mm_min_ss(_mm_set_ss(nominal),_mm_set_ss(s->blend_in)));
        s->blend_out=_mm_cvtss_f32(_mm_min_ss(_mm_set_ss(nominal),_mm_set_ss(s->blend_out)));
        return nominal;
    }
    return s->duration-s->elapsed;
}
PTM_EXPORT float nhl27_ptm_blend_feature_speed(float feature,float remaining){
    return feature>0x1p-16f?feature/remaining:1.0f;
}
PTM_EXPORT float nhl27_ptm_blend_retime(struct ptm_blend_clock *s,float feature,float live_speed){
    float remaining=feature/live_speed;
    s->feature_ticks=feature;
    nhl27_ptm_blend_rescale(&s->duration,&s->elapsed,
        nhl27_ptm_blend_progress(s->duration,s->elapsed),remaining);
    return remaining;
}
PTM_EXPORT void nhl27_ptm_blend_advance(struct ptm_blend_clock *s,float delta){
    s->elapsed=_mm_cvtss_f32(_mm_min_ss(_mm_max_ss(_mm_setzero_ps(),
        _mm_set_ss(delta+s->elapsed)),_mm_set_ss(s->duration)));
}
/* Source 188c50, normal-search branch. The native linked pointers already
 * address Float interfaces (67745308); their getter is vtable+18. Read the
 * configuration again after the maximum getter: game-state callbacks may
 * replace it, and may also modify the speed being clamped. */
PTM_EXPORT void nhl24_ptm_blend_set_speed(void *match,void *context,float speed){
    panic_u8 *m=match,*p;
    typedef float(PTM_GAME_ABI *get_fn)(void*,void*);
    *(float*)(m+0x88)=speed;
    p=*(panic_u8**)(m+0x1100);
    if(!p)return;
    void *maximum=*(void**)(p+0x10);
    float high;
    if(maximum){
        high=((get_fn)(*(panic_address**)maximum)[3])(maximum,context);
        p=*(panic_u8**)(m+0x1100);
    }else high=*(float*)(p+0x20);
    void *minimum=*(void**)(p+0x18);
    float low=minimum?((get_fn)(*(panic_address**)minimum)[3])(minimum,context):*(float*)(p+0x24);
    *(float*)(m+0x88)=_mm_cvtss_f32(_mm_min_ss(
        _mm_max_ss(_mm_set_ss(low),_mm_set_ss(*(float*)(m+0x88))),_mm_set_ss(high)));
}
