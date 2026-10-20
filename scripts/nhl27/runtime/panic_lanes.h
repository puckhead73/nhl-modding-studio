#ifndef NHL27_PANIC_LANES_H
#define NHL27_PANIC_LANES_H
#include "panic_state.h"
/* Views separate source lane-analysis behavior from each game's record ABI.
 * Slots 1..6 are skaters. Slot 0 and inactive records are never updated. */
struct panic_lane_record {
    const float *position;
    const float *predicted_position;
    float speed;
    panic_u8 active;
    int *shot_tick;
    int *pass_tick;
    float *receiver_position;
};
struct panic_lane_frame {
    int possession_team;
    unsigned possession_role;
    int tick;
    unsigned prediction_ticks;
    float shot_radius, pass_radius;
    const float *puck_position;
    const float *team_target[2];
    struct panic_lane_record records[2][7];
};
struct panic_lane_services {
    void *context;
    void (*project)(void*,const float*,const float*,const float*,float*);
    float (*distance)(void*,const float*,const float*,const float*,const float*,float*,float*);
};
struct panic_lane_history {
    int shot_tick,pass_tick;
    float receiver_position[4];
};
struct panic_lane_settings { unsigned prediction_ticks; float shot_radius,pass_radius; };
struct panic_lane_condition_services {
    void *context;
    unsigned char (*zone)(void*,float,float);
    unsigned char (*shot_zone)(void*);
};
struct panic_lane_conditions { unsigned char pass,shot,combined; };
#if defined(_WIN32)
__declspec(dllexport)
#else
__attribute__((visibility("default")))
#endif
struct panic_lane_conditions nhl27_panic_lane_conditions(
    const struct panic_lane_history *,int,int,float,float,
    const struct panic_lane_condition_services *);
#if defined(_WIN32)
__declspec(dllexport)
#else
__attribute__((visibility("default")))
#endif
int nhl24_panic_lane_frame(void *,const float *,int,const struct panic_lane_settings *,
                          struct panic_lane_history [2][7],struct panic_lane_frame *);
#if defined(_WIN32)
__declspec(dllexport)
#else
__attribute__((visibility("default")))
#endif
void nhl27_panic_update_lanes(struct panic_lane_frame *,const struct panic_lane_services *);
#if defined(_WIN32)
__declspec(dllexport)
#else
__attribute__((visibility("default")))
#endif
void nhl27_panic_update_lanes_geometry(struct panic_lane_frame *);
#endif
