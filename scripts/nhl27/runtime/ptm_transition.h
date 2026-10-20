#ifndef NHL27_PTM_TRANSITION_H
#define NHL27_PTM_TRANSITION_H
#include "panic_state.h"
/* Port-owned allocation. Native match/controller layouts are accessed only
 * through adapters; this must never be overlaid on a native match object. */
struct ptm_transition {
    void *match, *controller, *clip, *pose;
    panic_address pose_handle;
    void *trajectory, *match_state;
    panic_address reserved38;
    panic_address begin, end, capacity;
    panic_address allocator, inline_storage;
    panic_address reserved68;
    float inline_vectors[64][4];
    float clip_time;
    panic_u8 mirrored, prepare_offsets, feature_flag, padding477[9];
    float target[4];
    float duration;
    unsigned curve;
    panic_address padding498;
};
_Static_assert(sizeof(struct ptm_transition)==0x4a0,"source transition allocation");
_Static_assert(__builtin_offsetof(struct ptm_transition,clip_time)==0x470,"source transition timing");
struct ptm_transition_services {
    void *context;
    void (*release_pose)(void*,void*);
    void (*destroy_controller)(void*,void*);
    void (*release_vector)(void*,void*,panic_address,panic_address);
    void (*free_transition)(void*,struct ptm_transition*);
};
struct ptm_capture_services {
    void *context;
    panic_address (*sample_clip)(void*,struct ptm_transition*,void*,float);
    panic_address (*evaluate_controller)(void*,void*,void*);
    panic_address (*combine)(void*,void*,void*,panic_address,panic_address);
    void *(*create_pose)(void*,void*);
    void (*retain_pose)(void*,void*);
    void (*release_pose)(void*,void*);
    panic_address (*write_pose)(void*,void*,panic_address,void*);
};
struct ptm_evaluate_services {
    void *context;
    panic_address (*invalid_handle)(void*);
    panic_address (*import_pose)(void*,void*,void*);
    float (*clip_position)(void*,const void*);
    float (*curve)(void*,float,unsigned);
    panic_address (*blend_complete)(void*,void*,void*,panic_address,panic_address);
    panic_address (*blend_partial)(void*,void*,void*,panic_address,panic_address,float);
    panic_address (*apply_trajectory)(void*,void*,void*,panic_address,const float*,float);
    void (*prepare_offsets)(void*,struct ptm_transition*,void*);
};
struct ptm_creation {
    void *match,*controller,*clip,*trajectory,*match_state;
    panic_address invalid_handle,allocator;
    float clip_time,duration;
    const unsigned *curve;
    const panic_u8 *feature_flag;
    panic_u8 mirrored;
    const float *target;
};
struct ptm_creation_services {
    void *context;
    struct ptm_transition *(*allocate)(void*,panic_address);
    void (*destroy_controller)(void*,void*);
    void (*capture)(void*,struct ptm_transition*,void*);
};
#if defined(_WIN32)
#define PTM_EXPORT __declspec(dllexport)
#else
#define PTM_EXPORT __attribute__((visibility("default")))
#endif
PTM_EXPORT void nhl27_ptm_transition_destruct(struct ptm_transition*,const struct ptm_transition_services*);
PTM_EXPORT void nhl27_ptm_transition_clear(struct ptm_transition**,const struct ptm_transition_services*);
PTM_EXPORT const panic_address *nhl27_ptm_transition_offsets(const struct ptm_transition*,int);
PTM_EXPORT void nhl27_ptm_transition_capture(struct ptm_transition*,void*,const struct ptm_capture_services*);
PTM_EXPORT panic_address nhl27_ptm_transition_evaluate(struct ptm_transition*,void*,const void*,
    panic_address,float,const struct ptm_evaluate_services*);
PTM_EXPORT void nhl27_ptm_transition_create(struct ptm_transition**,const struct ptm_creation*,
    void*,const struct ptm_creation_services*);
PTM_EXPORT void nhl27_ptm_transition_initialize(struct ptm_transition*,struct ptm_transition**,
    const struct ptm_creation*,void*,const struct ptm_creation_services*);
#endif
