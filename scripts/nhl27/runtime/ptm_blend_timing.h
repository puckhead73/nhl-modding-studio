#ifndef NHL27_PTM_BLEND_TIMING_H
#define NHL27_PTM_BLEND_TIMING_H
#include "ptm_native.h"
struct ptm_blend_clock {
    float duration,elapsed,nominal,blend_in,blend_out,feature_ticks,speed;
};
PTM_EXPORT float nhl27_ptm_blend_progress(float,float);
PTM_EXPORT void nhl27_ptm_blend_rescale(float*,float*,float,float);
PTM_EXPORT float nhl27_ptm_blend_begin(struct ptm_blend_clock*,float);
PTM_EXPORT float nhl27_ptm_blend_feature_speed(float,float);
PTM_EXPORT float nhl27_ptm_blend_retime(struct ptm_blend_clock*,float,float);
PTM_EXPORT void nhl27_ptm_blend_advance(struct ptm_blend_clock*,float);
PTM_EXPORT void nhl24_ptm_blend_set_speed(void*,void*,float);
#endif
