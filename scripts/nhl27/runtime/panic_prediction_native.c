/* Adapt only the predictor inputs whose native/source layouts are recovered.
 * Use source arithmetic, including its polynomial angle reduction, for both
 * trajectories. NHL24's own prediction buffers remain owned by NHL24. */
#include "panic_lanes.h"
#if defined(_WIN32)
#define EXPORT __declspec(dllexport)
#define GAME_ABI __attribute__((sysv_abi))
/* Freestanding Windows test DLL: touch every guard page for the predictor's
 * local record. The FreeBSD guest object uses its normal native stack ABI. */
__asm__(".text\n.globl __chkstk\n__chkstk:\n"
        "pushq %rcx\npushq %rax\nleaq 24(%rsp),%rcx\n"
        "cmpq $4096,%rax\njb 2f\n1:\nsubq $4096,%rcx\n"
        "testb %al,(%rcx)\nsubq $4096,%rax\ncmpq $4096,%rax\njae 1b\n"
        "2:\nsubq %rax,%rcx\ntestb %al,(%rcx)\npopq %rax\npopq %rcx\nretq\n");
#else
#define EXPORT __attribute__((visibility("default")))
#define GAME_ABI
#endif
extern GAME_ABI void nhl27_panic_prediction_arrays(void*,const void*const*);
/* Asset integration sets this only after all panic consumers are installed. */
EXPORT unsigned char nhl27_panic_lanes_enabled;
static struct {
    panic_address owner;
    unsigned char valid[2];
    struct panic_lane_history banks[2][2][7];
} lane_history;

EXPORT GAME_ABI void nhl24_panic_lanes_reset_guest(panic_address base,void *history) {
    (void)base;
    if(lane_history.owner==(panic_address)history) {
        lane_history.valid[0]=lane_history.valid[1]=0;
        lane_history.owner=0;
    }
}

EXPORT int nhl24_panic_predict_position(const void *record,const float settings[3],
                                      unsigned sample,float output[4]) {
    if(!record || !settings || !output || sample>=60)return 0;
    const unsigned char *native=record;
    _Alignas(16) unsigned char source[0x1e80];
    /* The bounded source loops read position, direction, acceleration vector,
     * the shared 108-byte physics block and three scalar motion fields. */
    for(unsigned n=0;n<0x60;n++)source[n]=native[n];
    for(unsigned n=0;n<0x108;n++)source[0x160+n]=native[0x160+n];
    for(unsigned n=0;n<12;n++)source[0x344+n]=native[0x2b4+n];
    struct parameter {unsigned char unused[0x34];float value;} parameters[3];
    const void *pointers[3];
    for(unsigned n=0;n<3;n++){
        parameters[n].value=settings[n];pointers[n]=&parameters[n];
    }
    nhl27_panic_prediction_arrays(source,pointers);
    unsigned offset=(*(const unsigned short*)(native+0x31c)&8)?0x1700:0x1ac0;
    const float *prediction=(const float*)(source+offset+sample*16);
    for(unsigned n=0;n<4;n++)output[n]=prediction[n];
    return 1;
}

EXPORT int nhl24_panic_lane_predict(struct panic_lane_frame *frame,
                                   const float settings[3],float positions[2][7][4]) {
    if(!frame || !settings || !positions || frame->prediction_ticks>=60)return 0;
    for(unsigned t=0;t<2;t++)for(unsigned r=1;r<7;r++) {
        struct panic_lane_record *view=&frame->records[t][r];
        if(view->active!=1)continue;
        if(!nhl24_panic_predict_position(view->position,settings,frame->prediction_ticks,
                                         positions[t][r]))return 0;
        view->predicted_position=positions[t][r];
    }
    return 1;
}

EXPORT int nhl24_panic_lanes_update(void *history,const float *puck,int tick) {
    if(!history || !puck)return 0;
    unsigned bank=*(unsigned*)((unsigned char*)history+0x1bfb0);
    if(bank>=2)return 0;
    if(lane_history.owner!=(panic_address)history) {
        lane_history.valid[0]=lane_history.valid[1]=0;
        lane_history.owner=(panic_address)history;
    }
    lane_history.valid[bank]=0;
    struct panic_lane_frame frame;
    /* Source initializer 49e6790..49e6963: shot/pass radius 1.5, horizon 6;
     * secondary turn damping 0, human turn damping .97, AI damping .75. */
    const struct panic_lane_settings settings={6,1.5f,1.5f};
    const float prediction_settings[3]={0.0f,0.97f,0.75f};
    float positions[2][7][4];
    if(!nhl24_panic_lane_frame(history,puck,tick,&settings,lane_history.banks[bank],&frame))return 0;
    if(!nhl24_panic_lane_predict(&frame,prediction_settings,positions))return 0;
    nhl27_panic_update_lanes_geometry(&frame);
    lane_history.valid[bank]=1;
    return 1;
}

EXPORT GAME_ABI void nhl24_panic_lanes_guest(panic_address base,void *history) {
    if(nhl27_panic_lanes_enabled!=1 || !base || !history)return;
    unsigned char *puck=*(unsigned char**)(base+0xbae0ea8);
    if(!puck)return;
    unsigned char *pose=*(unsigned char**)(puck+0x10);
    if(!pose)return;
    nhl24_panic_lanes_update(history,(float*)(pose+0x30),*(int*)(base+0xba45a04));
}

EXPORT GAME_ABI const struct panic_lane_history *nhl24_panic_lane_history_guest(
        panic_address base,void *actor) {
    if(nhl27_panic_lanes_enabled!=1 || !base || !actor)return 0;
    unsigned char *manager=*(unsigned char**)(base+0xbadc558);
    if(!manager)return 0;
    unsigned char *analysis=*(unsigned char**)(manager+0x3d0);
    if(!analysis)return 0;
    unsigned char *history=*(unsigned char**)(analysis+0x48);
    if(!history || history[8] || lane_history.owner!=(panic_address)history)return 0;
    unsigned bank=*(unsigned*)(history+0x1bfb0);
    unsigned team=*(unsigned*)((unsigned char*)actor+0x30);
    unsigned char *role=*(unsigned char**)((unsigned char*)actor+0x100);
    if(bank>=2 || team>=2 || !role || !lane_history.valid[bank])return 0;
    unsigned index=*(unsigned*)(role+0x40);
    return index<7?&lane_history.banks[bank][team][index]:0;
}

extern unsigned char nhl27_panic_pass_zone(float,float,float);
extern unsigned char nhl27_panic_shot_zone(float,float,float);
struct lane_zone_context {float blue_line,direction,z;};
static unsigned char lane_pass_zone(void *opaque,float direction,float z) {
    const struct lane_zone_context *context=opaque;
    return nhl27_panic_pass_zone(direction,z,context->blue_line);
}
static unsigned char lane_shot_zone(void *opaque) {
    const struct lane_zone_context *context=opaque;
    return nhl27_panic_shot_zone(context->direction,context->z,context->blue_line);
}
/* Keep new flags separate from native animation storage. The source 6e3/6e4
 * offsets do not name spare bytes in the native object. Consumer adapters
 * must explicitly obtain these results instead of aliasing native flags. */
EXPORT GAME_ABI int nhl24_panic_lane_conditions_guest(panic_address base,void *actor,
                                                     struct panic_lane_conditions *out) {
    if(!out)return 0;
    out->pass=out->shot=out->combined=0;
    const struct panic_lane_history *history=nhl24_panic_lane_history_guest(base,actor);
    if(!history)return 0;
    unsigned char *a=actor;
    const unsigned char *opponent=*(const unsigned char**)(a+0x920);
    const unsigned char *pose=*(const unsigned char**)(a+0x10);
    const unsigned char *puck=*(const unsigned char**)(base+0xbae0ea8);
    if(!opponent || !pose || !puck)return 0;
    const unsigned char *puck_pose=*(const unsigned char**)(puck+0x10);
    if(!puck_pose)return 0;
    /* Native team-direction read 2632174..2632185 corresponds to source
     * 4709f4f..4709f5f. Actor direction is copied by 2824905..2824912. */
    struct lane_zone_context context={*(const float*)(base+0xaa5d4a8),
                                     *(const float*)(a+0x930),*(const float*)(pose+0x38)};
    volatile struct panic_lane_condition_services services;
    services.context=&context;services.zone=lane_pass_zone;services.shot_zone=lane_shot_zone;
    /* Source initializer 43b6abe sets the shot/pass lane frame window to 5. */
    *out=nhl27_panic_lane_conditions(history,*(int*)(base+0xba45a04),5,
          *(const float*)(opponent+0xec),*(const float*)(puck_pose+0x38),
          (const struct panic_lane_condition_services*)&services);
    return 1;
}
