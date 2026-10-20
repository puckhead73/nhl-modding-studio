/* 4a3c260 geometry with original source overlap/intersection machine leaves.
 * Feet are supplied in native NHL24 world coordinates by the engine adapter. */
#include "panic_state.h"
#include <xmmintrin.h>
#if defined(_WIN32)
#define EXPORT __declspec(dllexport)
#define ABI __attribute__((sysv_abi))
#else
#define EXPORT __attribute__((visibility("default")))
#define ABI
#endif
/* Source 453b4a0: ordered comparison chooses the positive end for both
 * signed zeros. XOR preserves the sign and payload of exceptional values. */
EXPORT float nhl27_panic_end_distance(const float *position,float end_z) {
    union {float f;unsigned u;} end={end_z};
    if(!(0.0f<=position[2]))end.u^=0x80000000u;
    float x=position[0],z=position[2];
    /* Preserve operand order: swapping ADDSS operands changes NaN payloads. */
    __asm__("mulss %0,%0\n\tsubss %2,%1\n\tmulss %1,%1\n\t"
            "addss %1,%0\n\tsqrtss %0,%0"
            : "+&x"(x), "+&x"(z) : "x"(end.f));
    return x;
}
/* Native rink initializer 2a87280 publishes the goal-line Z coordinate here.
 * Both titles use scale * 89ft * .3048 + .0254 for ordinary rink modes.
 * Source-only alternate-rink offset is outside this native rink adapter. */
EXPORT ABI float nhl24_panic_end_distance_guest(panic_address base,void *actor) {
    const unsigned char *pose=*(const unsigned char**)((unsigned char*)actor+0x10);
    return nhl27_panic_end_distance((const float*)(pose+0x30),*(const float*)(base+0xaa5d4a4));
}
typedef panic_u8 (ABI *boundary_query)(float,void*,const float*);
/* Native 3116835..3116863 is the inlined counterpart of source 452bfd0.
 * Reuse the actor's cache so other native queries see the same sample/tick. */
EXPORT void *nhl24_panic_boundary_cache(void *actor,unsigned tick,boundary_query query) {
    unsigned char *a=actor;
    if(*(unsigned*)(a+0x238)!=tick){
        *(unsigned*)(a+0x238)=tick;
        const unsigned char *pose=*(const unsigned char**)(a+0x10);
        (void)query(0.0f,a+0x1d0,(const float*)(pose+0x30));
    }
    return a+0x1d0;
}
EXPORT ABI float nhl24_panic_boundary_distance_guest(panic_address base,void *actor) {
    const float *cache=nhl24_panic_boundary_cache(actor,*(unsigned*)(base+0xba45a04),
                                                (boundary_query)(base+0x2a8dbd0));
    return cache[1];
}
typedef float panic_vec4 __attribute__((vector_size(16)));
EXPORT ABI void *nhl24_panic_first_candidate_guest(panic_address base,void *actor) {
    const unsigned char *pose=*(const unsigned char**)((unsigned char*)actor+0x10);
    panic_vec4 position;
    __builtin_memcpy(&position,pose+0x30,sizeof(position));
    return ((void*(ABI*)(void*,unsigned,panic_vec4))(base+0x27bff20))(actor,0,position);
}
EXPORT ABI void *nhl24_panic_second_candidate_guest(panic_address base,void *actor) {
    return ((void*(ABI*)(void*))(base+0x2842b10))(actor);
}
/* Source 44c6350 adds Z squared to (X squared + Y squared). Native DPPS
 * changes exceptional-value propagation; retain source arithmetic here. */
EXPORT float nhl27_panic_actor_distance(void *candidate,void *actor) {
    const unsigned char *a=*(const unsigned char**)((unsigned char*)actor+0x10);
    const unsigned char *b=*(const unsigned char**)((unsigned char*)candidate+0x10);
    __m128 value=_mm_loadu_ps((const float*)(a+0x30));
    __m128 other=_mm_loadu_ps((const float*)(b+0x30)),temp;
    __asm__("vsubps %2,%0,%0\n\tvmulps %0,%0,%0\n\tvmovshdup %0,%1\n\t"
            "vaddss %1,%0,%1\n\tvshufpd $1,%0,%0,%0\n\tvaddss %1,%0,%0\n\tvsqrtss %0,%0,%0"
            : "+&x"(value), "=&x"(temp) : "x"(other));
    return _mm_cvtss_f32(value);
}
extern ABI panic_u8 nhl27_segment_overlap(const float*,const float*,const float*,const float*);
extern ABI panic_u8 nhl27_segment_intersection(const float*,const float*,const float*,const float*,float*,float*,float*);
EXPORT panic_u8 nhl27_panic_between_feet(const float *puck_start,const float *puck_end,
        const float *right,const float *left,const float *previous_right,const float *previous_left) {
    if (!nhl27_segment_overlap(puck_start,puck_end,right,left)) return 0;
    float a[4]={0},b[4]={0},c[4]={0},d[4]={0},e[4]={0},f[4]={0};
    panic_u8 first=nhl27_segment_intersection(puck_end,puck_start,previous_right,right,a,b,c);
    panic_u8 second=nhl27_segment_intersection(puck_end,puck_start,previous_left,left,d,e,f);
    return ((first^1u)|(!(c[0]>b[0]))) & ((second^1u)|(!(f[0]>e[0])));
}
/* Mode zero of 44c6850; preserve all lanes and clamp only ordered negative Y. */
EXPORT void nhl27_panic_predict_puck(float *out,const float *position,const float *velocity,int ticks) {
    float dt=(float)ticks;
    for(unsigned n=0;n<4;n++)out[n]=velocity[n]*dt+position[n];
    if(out[1]<0)out[1]=0;
}
extern ABI int nhl24_panic_foot_probe_guest(panic_address,void*,unsigned,float*);
typedef void (ABI *history_query)(float*,unsigned,unsigned,unsigned);
EXPORT ABI panic_u8 nhl24_panic_between_legs_guest(panic_address base,void *actor,
                                                  panic_u8 predict,int ticks) {
    if(!base || !actor || !*(void**)(base+0xbadc558))return 0;
    unsigned char *puck=*(unsigned char**)(base+0xbae0ea8);
    if(!puck)return 0;
    unsigned char *pose=*(unsigned char**)(puck+0x10);
    float start[4],end[4],right[4],left[4],previous_right[4],previous_left[4];
    for(unsigned n=0;n<4;n++)start[n]=((float*)(pose+0x30))[n];
    if(predict)nhl27_panic_predict_puck(end,(float*)(pose+0x30),(float*)(pose+0x50),ticks);
    else for(unsigned n=0;n<4;n++)end[n]=((float*)(pose+0x40))[n];
    if(!nhl24_panic_foot_probe_guest(base,actor,0x27,right) ||
       !nhl24_panic_foot_probe_guest(base,actor,0x25,left))return 0;
    /* Source 4c2bd30 and target 30e7ff0 both use manager actor-ID array +1d8
     * and previous transforms +28, with the qualified virtual-foot ID remap. */
    history_query previous=(history_query)(base+0x30e7ff0);
    unsigned id=*(unsigned*)((unsigned char*)actor+0x3c);
    unsigned hand=*(unsigned*)((unsigned char*)actor+0x170);
    previous(previous_right,0x22,id,hand);previous(previous_left,0x20,id,hand);
    return nhl27_panic_between_feet(start,end,right,left,previous_right,previous_left);
}
