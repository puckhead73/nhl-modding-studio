/* Exact state operations from NHL27 4a51a80/4a51930/4a05fb0. Engine predicates,
 * reactions and event emission remain separate: these routines never substitute
 * a guessed input for an unresolved game service.
 */
#include "panic_state.h"
#if defined(_WIN32)
#define EXPORT __declspec(dllexport)
/* Freestanding Windows test DLL has no CRT floating-point marker. */
int _fltused=0;
#else
#define EXPORT __attribute__((visibility("default")))
#endif

EXPORT float nhl27_panic_attribute(unsigned id, int rating) {
    if (id==0x123) return (float)rating;
    return ((float)rating-50.0f)/(rating<51 ? 50.0f : 49.0f);
}

/* 4a53b70: failures preserve the old stumble byte. All three JA gates are
 * ordered comparisons; NaNs cannot request a stumble through this path. */
EXPORT void nhl27_panic_request_stumble(struct panic_bindings *bindings,
        const struct panic_stumble_input *input) {
    if (!bindings->requested || *bindings->requested!=1) return;
    if (!input->attribute_active) {
        float distance=0x1.fffffep+127f;
        if (input->possessor_present) {
            float x=input->possessor_position[0]-input->position[0];
            float y=input->possessor_position[1]-input->position[1];
            float z=input->possessor_position[2]-input->position[2];
            distance=z*z+(x*x+y*y);
        }
        if (!(input->speed>input->speed_threshold) ||
            !(input->rating_threshold>input->rating/100.0f) ||
            input->possession_team!=(input->actor_team!=1) ||
            !(input->radius*input->radius*0x1.7c87e6p-4f>distance)) return;
    }
    if (bindings->stumble) *bindings->stumble=1;
}

static float source_min(float a,float b) {return a<b ? a : b;}
static float source_max(float a,float b) {return a>b ? a : b;}

/* 69bb3d0 clamps the input, sorts reversed endpoints and preserves the SSE
 * second-operand rule for equal/NaN min/max operands. */
EXPORT float nhl27_panic_blend(float value,float start,float end,float from,float to) {
    if(start-end==0.0f)return value<=start ? from : to;
    float low_value=end<start ? to : from;
    float high_value=end<start ? from : to;
    float low=source_min(end,start),high=source_max(start,end);
    value=source_max(value,low);
    value=source_min(value,high);
    return (high_value-low_value)*((value-low)/(high-low))+low_value;
}

/* 4a52300: the hold phase preserves the current values, including externally
 * modified values. An expired effect leaves values untouched for this frame;
 * cancellation blending starts only when the effect lookup returns absent. */
EXPORT void nhl27_panic_takeoff(struct panic_state *state,const struct panic_takeoff_input *input) {
    state->flag=0;
    if(!input->feature_present)return;
    if(!input->effect_present) {
        for(unsigned n=0;n<4;n++) {
            float value=input->values[n] ? *input->values[n] : 0;
            float baseline=state->attributes[n],difference=value-baseline;
            union {float f;panic_u32 u;} magnitude={difference};
            magnitude.u&=0x7fffffffu;
            if(!(0x1p-23f>=magnitude.f)) {
                if(difference>0)value=source_max(baseline,value-input->cancellation_rate);
                else value=source_min(value+input->cancellation_rate,baseline);
            }
            if(input->values[n])*input->values[n]=value;
        }
        return;
    }
    int age=(int)((panic_u32)input->tick-(panic_u32)input->effect_start);
    int held=(int)((panic_u32)input->blend_in+(panic_u32)input->hold);
    int end=(int)((panic_u32)held+(panic_u32)input->blend_out);
    if(age>end)return;
    state->flag=1;
    float values[4];
    for(unsigned n=0;n<4;n++) {
        if(age<input->blend_in)
            values[n]=nhl27_panic_blend((float)age,0,(float)input->blend_in,
                                       state->attributes[n],input->maximum[n]);
        else if(age<held)values[n]=input->values[n] ? *input->values[n] : 0;
        else values[n]=nhl27_panic_blend((float)age,(float)held,(float)end,
                                        input->maximum[n],state->attributes[n]);
    }
    for(unsigned n=0;n<4;n++)if(input->values[n])*input->values[n]=values[n];
}

/* Return bit 0: call request generator; bit 1: dispatch the existing turn type;
 * bit 2: process the active stumble event. The request-generator branch skips
 * dispatch even if it selects a type during this frame.
 */
EXPORT unsigned nhl27_panic_begin_update(struct panic_state *state,
        struct panic_bindings *bindings, panic_u8 animation_is_panic, int cooldown_ticks) {
    if (bindings->interrupt_exit) *bindings->interrupt_exit=0;
    if (!bindings->ptm_active || *bindings->ptm_active!=1 || !animation_is_panic) {
        state->active_ticks=0;
        if (state->duration<1) return state->cooldown<1 ? 1u : 0u;
        --state->duration;
        return 2;
    }
    /* INC wraps on the source CPU; use unsigned arithmetic rather than C UB. */
    state->active_ticks=(int)((panic_u32)state->active_ticks+1u);
    state->duration=state->duration>1 ? state->duration-1 : 0;
    state->cooldown=cooldown_ticks;
    return 2u | (bindings->stumble && *bindings->stumble==1 ? 4u : 0u);
}

/* Called after type handler, stumble request and event emission. The active
 * predicate must be sampled again at this point, as in the source routine.
 */
EXPORT void nhl27_panic_finish_cooldown(struct panic_state *state,
        struct panic_bindings *bindings, panic_u8 animation_is_panic) {
    int old=state->cooldown;
    state->cooldown=old>1 ? old-1 : 0;
    if (old>1 && (!bindings->ptm_active || *bindings->ptm_active!=1 || !animation_is_panic)) {
        if (bindings->requested) *bindings->requested=0;
    }
}

/* Shared reaction cancellation sequence in 4a51a80. This is intentionally not
 * the broader 4a51930 reset: cooldown, stamp, poke opponent, flag and attribute
 * values survive cancellation. Pose validity requires session result == 1.
 */
EXPORT void nhl27_panic_reaction_cancel(struct panic_state *state, struct panic_bindings *bindings,
        panic_u8 session_result, float facing, const float *position) {
    state->duration=state->active_ticks=0;
    state->opponent=0;
    if (bindings->requested) *bindings->requested=0;
    if (bindings->stumble) *bindings->stumble=0;
    if (bindings->between_legs) *bindings->between_legs=0;
    if (bindings->type) *bindings->type=0;
    if (state->actor && session_result==1) {
        state->facing=facing;
        for (unsigned i=0;i<4;++i) state->position[i]=position[i];
    } else {
        state->facing=0;
        for (unsigned i=0;i<4;++i) state->position[i]=0;
    }
}

EXPORT void nhl27_panic_cancel(struct panic_state *state, struct panic_bindings *bindings,
        panic_u8 session_result, float facing, const float *position) {
    nhl27_panic_reaction_cancel(state,bindings,session_result,facing,position);
    if (bindings->interrupt_exit) *bindings->interrupt_exit=1;
}

/* 4a53360: a type-1 turn reacts to puck-race rank and possession. Keep an
 * existing request while its panic animation is active, but do not synthesize
 * a new request in that branch. Reaction cancellation does not set InterruptExit.
 */
EXPORT void nhl27_panic_skating_reaction(struct panic_state *state,
        struct panic_bindings *bindings, const struct panic_skating_input *input) {
    if (input->special_actor || (panic_u32)input->role-5u<=4u) {
        nhl27_panic_reaction_cancel(state,bindings,input->session_result,
                                   input->facing,input->position);
        return;
    }
    if ((!input->own_rank_present || !input->opponent_rank_present ||
         input->own_rank<=input->opponent_rank) &&
        (!input->possessor_present || input->possessor_team==input->actor_team)) {
        if (bindings->ptm_active && *bindings->ptm_active==1 && input->animation_is_panic)
            return;
        if (bindings->requested) *bindings->requested=0;
    } else if (bindings->requested) {
        *bindings->requested=1;
    }
}

/* ROUNDSS imm9 without a CRT dependency. Preserve signed zero and return
 * infinities/NaNs unchanged, just as the source's round-down operation does. */
static float floor_scalar(float value) {
    union { float f; panic_u32 u; } v={value};
    unsigned exponent=(v.u>>23)&255u;
    if (exponent>=150) return value;
    if (exponent<127) {
        if (!(v.u&0x7fffffffu)) return value;
        return v.u>>31 ? -1.0f : 0.0f;
    }
    panic_u32 mask=(1u<<(150-exponent))-1u;
    if (!(v.u&mask)) return value;
    if (v.u>>31) v.u+=mask;
    v.u&=~mask;
    return v.f;
}

static float wrap_angle(float value) {
    if (value<0.0f || value>=0x1.921fb6p+2f) {
        float fraction=(value+-0x1.921fb6p+1f)*0x1.45f306p-3f;
        fraction=fraction-floor_scalar(fraction);
        if (0.5f<fraction) fraction=fraction+-1.0f;
        value=fraction*0x1.921fb6p+2f+0x1.921fb6p+1f;
    }
    return value;
}

static float folded_angle(float value) {
    value=wrap_angle(value);
    if (0x1.921fb6p+1f<value)
        value=wrap_angle(0x1.921fb6p+2f-wrap_angle(value));
    return value;
}

/* Source atan approximation uses SSE min/max operand ordering, including
 * returning the second operand on unordered comparisons and equal zeros. */
static float displacement_heading(float x, float y, float z) {
    if (x==0 && y==0 && z==0) return 0;
    union { float f; panic_u32 u; } ux={x},uz={z},ax,az,result;
    ax.u=ux.u&0x7fffffffu;az.u=uz.u&0x7fffffffu;
    float heading=0;
    if (!(0x1p-16f>=ax.f) || !(0x1p-16f>=az.f)) {
        float denominator=az.f>ax.f ? az.f : ax.f;
        denominator=denominator>0x1p-16f ? denominator : 0x1p-16f;
        float numerator=az.f<ax.f ? az.f : ax.f;
        float t=numerator*(1.0f/denominator),q=t*t;
        heading=(((((q*-0x1.b9ba60p-7f+0x1.d6daaap-5f)*q-0x1.f09862p-4f)*q+
                     0x1.90a992p-3f)*q-0x1.54fc8ap-2f)*q+0x1.ffff6ep-1f)*t;
        if (az.f<ax.f) heading=0x1.921fb6p+0f-heading;
        if (uz.u>>31) heading=0x1.921fb6p+1f-heading;
        result.f=heading;
        if (ux.u>>31) result.u^=0x80000000u;
        heading=result.f;
    }
    return wrap_angle(heading);
}

/* 4a52cd0: inspect the actor's ordered nearby-object list. Object type 1 is
 * excluded independently of team; a successful candidate ends the traversal. */
EXPORT void nhl27_panic_poke_reaction(struct panic_state *state,
        struct panic_bindings *bindings, const struct panic_poke_input *input) {
    if (!state->poke_opponent || input->special_actor || (panic_u32)input->role-5u<=4u) {
        nhl27_panic_reaction_cancel(state,bindings,input->session_result,input->facing,input->position);
        return;
    }
    for (const struct panic_poke_candidate *c=input->candidates;c;c=c->next) {
        if (c->object_type==1) continue;
        float x=c->position[0]-input->position[0],y=c->position[1]-input->position[1];
        float z=c->position[2]-input->position[2];
        float distance=(y*y+x*x)+z*z;
        /* JA rejects only ordered greater; JBE accepts unordered too. */
        if (!(distance>input->radius*input->radius*0x1.7c87e6p-4f) &&
            (!(input->facing_threshold>folded_angle(input->facing-c->facing)*0x1.ca5dc2p+5f) ||
             (bindings->requested && *bindings->requested==1))) {
            float heading=displacement_heading(x,y,z);
            if (!(input->heading_threshold>folded_angle(input->facing-heading)*0x1.ca5dc2p+5f)) {
                if (bindings->requested) *bindings->requested=1;
                return;
            }
        }
    }
    if (bindings->ptm_active && *bindings->ptm_active==1 && input->animation_is_panic) return;
    if (bindings->requested) *bindings->requested=0;
}

/* 4a53570: the trajectory sample's fourth lane is heading, not homogeneous W.
 * A valid sample always refreshes the cached pose, even if no turn is requested. */
EXPORT void nhl27_panic_generate_request(struct panic_state *state,
        struct panic_bindings *bindings,const struct panic_request_input *input) {
    state->duration=state->active_ticks=0;
    state->opponent=0;
    if(bindings->requested)*bindings->requested=0;
    if(bindings->stumble)*bindings->stumble=0;
    if(bindings->between_legs)*bindings->between_legs=0;
    if(bindings->type)*bindings->type=0;
    if(!input->trajectory_present || (bindings->path_lock && *bindings->path_lock!=0) ||
       (panic_u32)input->role-5u<5u || input->special_actor) {
        nhl27_panic_reaction_cancel(state,bindings,input->session_result,input->facing,input->position);
        return;
    }
    float facing=wrap_angle(wrap_angle(input->trajectory_sample[3]+0x1.921fb6p+0f));
    float difference=folded_angle(facing-state->facing)*0x1.ca5dc2p+5f;
    if(180.0f-difference<input->angle_threshold && input->opposing_team && (input->direction_result&1u)) {
        float x=input->trajectory_sample[0]-state->position[0];
        float y=input->trajectory_sample[1]-state->position[1];
        float z=input->trajectory_sample[2]-state->position[2];
        float distance=(z*z+x*x)+y*y;
        __asm__("sqrtss %0,%0":"+x"(distance));
        if(input->distance_threshold<distance) {
            if(bindings->requested)*bindings->requested=1;
            state->duration=input->duration;
            if(bindings->type)*bindings->type=1;
        }
    }
    state->facing=facing;
    for(unsigned n=0;n<4;n++)state->position[n]=input->trajectory_sample[n];
}

/* 4a52840: between-legs recognition can request a turn before angle/speed
 * tests; failing those tests must not clear the request on a valid actor. */
EXPORT void nhl27_panic_deke_reaction(struct panic_state *state,
        struct panic_bindings *bindings, const struct panic_deke_input *input) {
    if (!state->opponent || input->special_actor || (panic_u32)input->role-5u<=4u) {
        nhl27_panic_reaction_cancel(state,bindings,input->session_result,
                                   input->facing,input->position);
        return;
    }
    int deadline=(int)((panic_u32)state->stamp+(panic_u32)input->delay);
    if (input->tick<deadline) return;
    float difference=wrap_angle(input->facing-input->opponent_facing);
    if (0x1.921fb6p+1f<difference)
        difference=wrap_angle(0x1.921fb6p+2f-wrap_angle(difference));
    float degrees=difference*0x1.ca5dc2p+5f;
    if (input->between_legs) {
        if (bindings->between_legs) *bindings->between_legs=1;
        if (bindings->requested) *bindings->requested=1;
    }
    if (degrees>input->angle_threshold && !input->component_blocked &&
        ((input->between_legs&1u) || input->speed>input->speed_threshold)) {
        float x=input->opponent_position[0]-input->position[0];
        float y=input->opponent_position[1]-input->position[1];
        float z=input->opponent_position[2]-input->position[2];
        float distance=z*z+(x*x+y*y);
        if (input->direction_result==1 && distance>input->distance_threshold)
            if (bindings->requested) *bindings->requested=1;
    }
}
