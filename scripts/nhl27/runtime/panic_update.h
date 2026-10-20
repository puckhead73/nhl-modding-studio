#ifndef NHL27_PANIC_UPDATE_H
#define NHL27_PANIC_UPDATE_H
#include "panic_state.h"
enum panic_checking_query {
    PANIC_CHECK_HIP_STATE,
    PANIC_CHECK_RALLY_REQUEST,
    PANIC_CHECK_INTERACTING,
    PANIC_CHECK_FAILED_SHOULDER,
    PANIC_CHECK_PUSH_ENABLED
};
struct panic_pending_services {
    void *context;
    void *(*resolve)(void *,void *);
    int (*has_asset)(void *,void *);
    unsigned (*query)(void *,void *,enum panic_checking_query);
};
#if defined(_WIN32)
__declspec(dllexport)
#else
__attribute__((visibility("default")))
#endif
panic_u8 nhl27_panic_pending_check(void *const *,void *const *,const struct panic_pending_services *);
struct panic_allowed_services {
    void *context;
    panic_u8 (*special_animation)(void*,void*);
    int (*state_condition)(void*,void*,panic_u8*);
};
/* Availability queries are live, ordered reads. The adapter must translate
 * actor flags/status explicitly; source status 17 is not a native enum binding.
 * OWNER is captured before condition callbacks, just as in source 455e7b0. */
enum panic_availability_query {
    PANIC_AVAIL_MANAGER_BLOCKED,
    PANIC_AVAIL_FLAG_B0,
    PANIC_AVAIL_FLAG_B8,
    PANIC_AVAIL_ACTOR_BYTE_40,
    PANIC_AVAIL_CONTROL,
    PANIC_AVAIL_CONTROL_MODE,
    PANIC_AVAIL_SOURCE_STATUS,
    PANIC_AVAIL_SPECIAL_ANIMATION,
    PANIC_AVAIL_GLOBAL_BLOCKED,
    PANIC_AVAIL_ROLE_KIND,
    PANIC_AVAIL_FLAG_B4,
    PANIC_AVAIL_OWNER,
    PANIC_AVAIL_STATE_CONDITION,
    PANIC_AVAIL_ACTOR_STATE_BLOCKED,
    PANIC_AVAIL_ANIMATION_BLOCKED,
    PANIC_AVAIL_REQUEST_OPTION,
    PANIC_AVAIL_REQUEST_ALLOWED
};
struct panic_availability_services {
    void *context;
    panic_address (*query)(void *,enum panic_availability_query,void *);
};
#if defined(_WIN32)
__declspec(dllexport)
#else
__attribute__((visibility("default")))
#endif
panic_u8 nhl27_panic_availability_tail(void *,const struct panic_availability_services *);
/* Source arcade request storage. This is not a declaration of a native NHL24
 * object layout. A live adapter must supply the source-compatible records. */
struct panic_arcade_request {
    panic_u32 actor_id, state, kind;
    panic_u8 opaque[0x20];
};
struct panic_request_services {
    void *context;
    void *(*manager)(void *);
    const struct panic_arcade_request *(*records)(void *,void *);
};
#if defined(_WIN32)
__declspec(dllexport)
#else
__attribute__((visibility("default")))
#endif
panic_u8 nhl27_panic_request_available(void *,const struct panic_request_services *);
#if defined(_WIN32)
__declspec(dllexport)
#else
__attribute__((visibility("default")))
#endif
panic_u8 nhl27_panic_request_blocked(void *,panic_u32,const struct panic_request_services *);
/* Services deliberately run at their source call sites. Reactions can mutate
 * active state, type and cooldown; pre-sampling a frame changes behavior. */
struct panic_update_services {
    void *context;
    struct panic_bindings *(*bindings)(void *);
    panic_u8 (*animation_is_panic)(void *);
    int (*cooldown_ticks)(void *);
    void (*active_stumble)(void *,struct panic_state *);
    void (*generate_request)(void *,struct panic_state *);
    void (*skating)(void *,struct panic_state *);
    void (*deke)(void *,struct panic_state *);
    void (*poke)(void *,struct panic_state *);
    void (*request_stumble)(void *,struct panic_state *);
    void (*emit_event)(void *,struct panic_state *);
    void (*cancel_gates)(void *,struct panic_state *);
};
struct panic_cancel_services {
    void *context;
    struct panic_bindings *(*bindings)(void *);
    const panic_u8 *(*interrupt_requested)(void *);
    panic_u8 (*collision)(void *);
    panic_u8 (*session_present)(void *);
    panic_u8 (*session_state)(void *);
    void (*pose)(void *,float *,float *);
};
struct panic_trajectory_view { panic_address begin, end; };
struct panic_reset_services {
    void *context;
    struct panic_bindings *(*bindings)(void *);
    panic_u8 (*session_state)(void *);
    void (*pose)(void *,panic_address,float *,float *);
    void (*clear_preconditions)(void *,panic_address);
    void (*reset_alignment)(void *,struct panic_state *);
    panic_u8 *(*feature_active)(void *,panic_address);
};
#if defined(_WIN32)
__declspec(dllexport)
#else
__attribute__((visibility("default")))
#endif
void nhl27_panic_reset_full(struct panic_state *,const struct panic_reset_services *);
struct panic_closest_services {
    void *context;
    struct panic_bindings *(*bindings)(void *);
    panic_u8 (*motion_active)(void *,panic_address);
    void *(*first_candidate)(void *,panic_address);
    void *(*second_candidate)(void *,panic_address);
    float (*cached_distance)(void *,panic_address);
    float (*candidate_distance)(void *,void *,panic_address);
    float (*third_distance)(void *,panic_address);
};
#if defined(_WIN32)
__declspec(dllexport)
#else
__attribute__((visibility("default")))
#endif
void nhl27_panic_update_closest(struct panic_state *,const struct panic_closest_services *);
struct panic_frame_services {
    void *context;
    panic_u8 (*cached_active)(void *,panic_address);
    void (*reactions)(void *,struct panic_state *);
    void (*preconditions)(void *,struct panic_state *);
    void (*feature)(void *,struct panic_state *);
    void (*closest)(void *,struct panic_state *);
    void (*takeoff)(void *,struct panic_state *);
    void (*alignment)(void *,struct panic_state *);
    struct panic_bindings *(*bindings)(void *);
    int (*tick)(void *);
    const struct panic_trajectory_view *(*trajectory)(void *,panic_address);
    void *(*query_path_tracker)(void *,panic_address);
    const float *(*tracker_position)(void *,void *);
    float (*tolerance)(void *);
    void (*reset)(void *,struct panic_state *);
    void (*reset_alignment)(void *,struct panic_state *);
};
#if defined(_WIN32)
__declspec(dllexport)
#else
__attribute__((visibility("default")))
#endif
void nhl27_panic_update_frame(struct panic_state *,const struct panic_frame_services *);
#if defined(_WIN32)
__declspec(dllexport)
#else
__attribute__((visibility("default")))
#endif
void nhl27_panic_update_cancel(struct panic_state *,const struct panic_cancel_services *);
#if defined(_WIN32)
__declspec(dllexport)
#else
__attribute__((visibility("default")))
#endif
void nhl27_panic_update_reactions(struct panic_state *,const struct panic_update_services *);
/* Source call identities remain explicit while native adapters are qualified.
 * Pointer-valued queries use panic_address, preserving both full pointers and
 * low-byte boolean return values from the original ABI. */
enum panic_feature_query {
    PANIC_FEATURE_INITIALIZED=0x3fddb0,
    PANIC_SPECIAL_SESSION=0x4846d50,
    PANIC_SESSION_ACTOR=0x4715010,
    PANIC_SAME_ACTOR=0x44c7fb0,
    PANIC_ACTOR_ALLOWED=0x439bdc0,
    PANIC_ACTOR_AVAILABLE=0x455ec20,
    PANIC_ANIMATION_REACTION=0x4a19c80,
    PANIC_ACTOR_STATE_BLOCKED=0x453cd50,
    PANIC_GAME_BLOCKED=0x4795920,
    PANIC_SESSION_OBJECT=0x452dc70,
    PANIC_SESSION_BLOCKED=0x3f0560,
    PANIC_ANIMATION_BLOCKED=0x4a19aa0,
    PANIC_GLOBAL_BLOCKED=0x453a9b0,
    PANIC_ACTOR_BLOCKED=0x453aa00,
    PANIC_REQUEST_BLOCKED=0x453e080,
    PANIC_COMPONENT_BLOCKED=0x4533ae0,
    PANIC_ANIMATION_FLAG_691=1,
    PANIC_ACTOR_ROLE_KIND=2,
    PANIC_SESSION_ACTOR_TYPE=3
};
struct panic_feature_services {
    void *context;
    void *(*feature)(void *,panic_address);
    panic_address (*query)(void *,enum panic_feature_query,panic_address,panic_address);
};
#if defined(_WIN32)
__declspec(dllexport)
#else
__attribute__((visibility("default")))
#endif
void nhl27_panic_update_feature(struct panic_state *,const struct panic_feature_services *);
#endif
