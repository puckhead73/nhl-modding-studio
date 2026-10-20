#ifndef NHL27_PANIC_STATE_H
#define NHL27_PANIC_STATE_H
typedef unsigned int panic_u32;
typedef unsigned char panic_u8;
typedef __UINTPTR_TYPE__ panic_address;
/* Source 4a54380 allocates 96 bytes. Keep padding untouched on resets. */
struct panic_state {
    panic_address actor, opponent, poke_opponent;
    int duration, active_ticks, cooldown, stamp;
    panic_u8 flag, padding[3];
    float facing, position[4], attributes[4], align_deadline;
    panic_u8 tail[12];
};
struct panic_bindings {
    panic_u8 *ptm_active;
    int *path_lock;
    panic_u8 *stumble, *requested;
    float *closest_distance;
    panic_u8 *between_legs;
    int *type;
    panic_u8 *interrupt_exit, *align_requested;
};
/* Values sampled by the engine adapter immediately before the type-1 handler.
 * Presence is distinct from rank zero and team zero. */
struct panic_skating_input {
    int role, actor_team, possessor_team, own_rank, opponent_rank;
    panic_u8 special_actor, possessor_present, own_rank_present, opponent_rank_present;
    panic_u8 animation_is_panic, session_result;
    float facing, position[4];
};
struct panic_deke_input {
    int role, tick, delay;
    panic_u8 special_actor, between_legs, component_blocked, direction_result, session_result;
    float facing, opponent_facing, speed, angle_threshold, speed_threshold, distance_threshold;
    float position[4], opponent_position[4];
};
struct panic_poke_candidate {
    const struct panic_poke_candidate *next;
    int object_type;
    float facing, position[4];
};
struct panic_poke_input {
    int role;
    panic_u8 special_actor, session_result, animation_is_panic;
    float facing, position[4], radius, facing_threshold, heading_threshold;
    const struct panic_poke_candidate *candidates;
};
struct panic_request_input {
    int role, duration;
    panic_u8 special_actor, session_result, trajectory_present, opposing_team, direction_result;
    float facing, position[4], trajectory_sample[4], angle_threshold, distance_threshold;
};
struct panic_stumble_input {
    int actor_team, possession_team;
    panic_u8 attribute_active, possessor_present;
    float rating, speed, speed_threshold, rating_threshold, radius;
    float position[4], possessor_position[4];
};
struct panic_takeoff_input {
    panic_u8 feature_present, effect_present;
    int tick, effect_start, blend_in, hold, blend_out;
    float cancellation_rate, maximum[4];
    float *values[4];
};
_Static_assert(sizeof(struct panic_state)==96,"source panic state size");
_Static_assert(__builtin_offsetof(struct panic_state,facing)==44,"source cached pose");
_Static_assert(__builtin_offsetof(struct panic_state,attributes)==64,"source attributes");
#endif
