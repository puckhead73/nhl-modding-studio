/* Source 49b6020: remember threatened shot/pass lanes, not completed passes.
 * Retain previous ticks when no lane qualifies and last qualifying receiver
 * in ascending role order. The two geometry services run at source call sites. */
#include "panic_lanes.h"
#if defined(_WIN32)
#define EXPORT __declspec(dllexport)
#define GAME_ABI __attribute__((sysv_abi))
#else
#define EXPORT __attribute__((visibility("default")))
#define GAME_ABI
#endif
/* Source zone helpers with their optional fraction outputs omitted (both
 * callers in 4a51d20 pass null). Keep the source blue-line edge convention. */
EXPORT unsigned char nhl27_panic_pass_zone(float direction,float z,float blue_line) {
    float product=direction*z;
    float edge=blue_line-0.15240000188350677f;
    return product>=edge;
}
EXPORT unsigned char nhl27_panic_shot_zone(float direction,float z,float blue_line) {
    if((direction<=0.0f)!=(0.0f<=z))return 0;
    union {float f;unsigned u;} negative={z};negative.u^=0x80000000u;
    /* MAXSS chooses its second operand when unordered or equal. */
    float magnitude=z>negative.f?z:negative.f;
    return magnitude>=blue_line-0.15240000188350677f;
}
/* Source 4a51d64..4a51e14. Subtractions wrap as x86 32-bit arithmetic;
 * the following comparisons are signed. Query both zones before testing age,
 * and query the shot zone only when the shot timestamp passes its age test. */
EXPORT struct panic_lane_conditions nhl27_panic_lane_conditions(
        const struct panic_lane_history *history,int tick,int window,
        float direction,float puck_z,const struct panic_lane_condition_services *s) {
    unsigned char puck_zone=s->zone(s->context,direction,puck_z);
    unsigned char receiver_zone=s->zone(s->context,direction,history->receiver_position[2]);
    int pass_age=(int)((unsigned)tick-(unsigned)history->pass_tick);
    int shot_age=(int)((unsigned)tick-(unsigned)history->shot_tick);
    struct panic_lane_conditions result;
    result.pass=(receiver_zone^puck_zone^1u)&(pass_age<window);
    result.shot=shot_age<window?s->shot_zone(s->context):0;
    result.combined=result.pass|result.shot;
    return result;
}
/* Native 2e202f0 fills speed/flags at 2d4/31c; 2e22100 writes the two
 * 60-sample trajectories at 1520/18e0. Source 49b4e00 clears its current
 * records each analysis update, so the extra lane fields must also reset.
 * This adapter does not assume native trajectory arithmetic equals source. */
EXPORT int nhl24_panic_lane_frame(void *history,const float *puck,int tick,
        const struct panic_lane_settings *settings,struct panic_lane_history extra[2][7],
        struct panic_lane_frame *frame) {
    if(!history || !puck || !settings || !extra || !frame)return 0;
    unsigned char *h=history;
    unsigned bank=*(unsigned*)(h+0x1bfb0);
    int team=*(int*)(h+0x7c);
    if(bank>=2 || team < -1 || team>1 || settings->prediction_ticks>=60)return 0;
    frame->possession_team=team;
    frame->possession_role=*(unsigned*)(h+0x88);
    frame->tick=tick;
    frame->prediction_ticks=settings->prediction_ticks;
    frame->shot_radius=settings->shot_radius;
    frame->pass_radius=settings->pass_radius;
    frame->puck_position=puck;
    for(unsigned t=0;t<2;t++) {
        unsigned char *team_record=h+0x1bfc0+bank*0x222e0+t*0x11170;
        frame->team_target[t]=(float*)(team_record+0x10);
        for(unsigned r=0;r<7;r++) {
            unsigned char *record=team_record+0x70+r*0x2700;
            unsigned flags=*(unsigned short*)(record+0x31c);
            struct panic_lane_record *view=&frame->records[t][r];
            struct panic_lane_history *state=&extra[t][r];
            state->shot_tick=state->pass_tick=0;
            for(unsigned k=0;k<4;k++)state->receiver_position[k]=0;
            view->position=(float*)record;
            view->predicted_position=(float*)(record+((flags&8)?0x1520:0x18e0)+settings->prediction_ticks*16);
            view->speed=*(float*)(record+0x2d4);
            view->active=flags&1;
            view->shot_tick=&state->shot_tick;
            view->pass_tick=&state->pass_tick;
            view->receiver_position=state->receiver_position;
        }
    }
    return 1;
}
static float squared_distance(const float *a,const float *b) {
    float value;
    __asm__ volatile("vmovups (%1), %%xmm0\n\t"
        "vsubps (%2), %%xmm0, %%xmm0\n\t"
        "vmulps %%xmm0, %%xmm0, %%xmm0\n\t"
        "vmovshdup %%xmm0, %%xmm1\n\t"
        "vaddss %%xmm1, %%xmm0, %%xmm1\n\t"
        "vshufpd $1, %%xmm0, %%xmm0, %%xmm0\n\t"
        "vaddss %%xmm1, %%xmm0, %0"
        : "=x"(value):"r"(a),"r"(b):"xmm0","xmm1");
    return value;
}
extern GAME_ABI unsigned char nhl27_lane_project(const float*,const float*,const float*,float*);
extern GAME_ABI float nhl27_lane_distance(const float*,const float*,const float*,const float*,float*,float*);
static void project_geometry(void *context,const float *a,const float *b,const float *c,float *out) {
    (void)context;nhl27_lane_project(a,b,c,out);
}
static float distance_geometry(void *context,const float *a,const float *b,
                               const float *c,const float *d,float *one,float *two) {
    (void)context;return nhl27_lane_distance(a,b,c,d,one,two);
}
EXPORT void nhl27_panic_update_lanes_geometry(struct panic_lane_frame *frame) {
    /* Materialize code pointers relative to the loaded payload. A compiler
     * constant aggregate would introduce absolute pointers in .data.rel.ro. */
    volatile struct panic_lane_services services;
    services.context=0;
    services.project=project_geometry;
    services.distance=distance_geometry;
    nhl27_panic_update_lanes(frame,(const struct panic_lane_services*)&services);
}
EXPORT void nhl27_panic_update_lanes(struct panic_lane_frame *f,
                                    const struct panic_lane_services *s) {
    if(f->possession_team==-1 || f->possession_role>=7)return;
    unsigned attacking=(unsigned)f->possession_team;
    unsigned defending=f->possession_team!=1;
    float projection[4]={0},first[4]={0},second[4]={0};
    for(unsigned role=1;role<=6;role++) {
        struct panic_lane_record *d=&f->records[defending][role];
        if(d->active!=1)continue;
        float predicted[4];
        for(unsigned k=0;k<4;k++)predicted[k]=d->predicted_position[k];
        float reach=(float)(int)f->prediction_ticks*d->speed+f->shot_radius;
        float radius2=reach*reach;
        const float *target=f->team_target[attacking];
        s->project(s->context,f->puck_position,target,d->position,projection);
        if(squared_distance(d->position,projection)<radius2 &&
           s->distance(s->context,f->puck_position,target,d->position,
                       predicted,first,second)<f->shot_radius)
            *d->shot_tick=f->tick;
        for(unsigned receiver=1;receiver<=6;receiver++) {
            if(receiver==f->possession_role)continue;
            struct panic_lane_record *r=&f->records[attacking][receiver];
            if(r->active!=1)continue;
            s->project(s->context,f->puck_position,r->position,d->position,projection);
            if(squared_distance(d->position,projection)<radius2 &&
               s->distance(s->context,f->puck_position,r->position,d->position,
                           predicted,first,second)<f->pass_radius) {
                *d->pass_tick=f->tick;
                for(unsigned k=0;k<4;k++)d->receiver_position[k]=r->position[k];
            }
        }
    }
}
