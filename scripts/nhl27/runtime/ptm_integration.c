/* Connect the source transition evaluator to the qualified native adapters. */
#include "ptm_native.h"
#include "ptm_lifecycle.h"
#include <xmmintrin.h>
typedef panic_address A;
#define ABI PTM_GAME_ABI
extern ABI A nhl24_ptm_pose_import_guest(A,void*,void*);
extern float nhl24_ptm_clip_position(const void*);
extern ABI float nhl24_ptm_curve_guest(A,unsigned,float);
extern ABI A nhl24_ptm_blend_complete_guest(A,void*,void*,A,A);
extern ABI A nhl24_ptm_blend_partial_guest(A,void*,void*,A,A,float);
extern ABI A nhl24_ptm_trajectory_guest(A,void*,A,const float*,float);
extern ABI void nhl24_ptm_prepare_offsets_guest(A,struct ptm_transition*,void*);
extern void nhl24_ptm_controller_destroy(void*);
extern ABI void nhl24_ptm_transition_capture_guest(A,struct ptm_transition*,void*);
extern ABI void nhl27_ptm_sample_path(float*,void*,float);
static void auto_create_guest(A,void*,void*,const void*,struct ptm_transition**);
static A invalid(void*c){return *(A*)((A)c+0xbe14630);}
static A import(void*c,void*e,void*p){return nhl24_ptm_pose_import_guest((A)c,e,p);}
static float position(void*c,const void*s){(void)c;return nhl24_ptm_clip_position(s);}
static float curve(void*c,float v,unsigned id){return nhl24_ptm_curve_guest((A)c,id,v);}
static A complete(void*c,void*e,void*m,A i,A p){return nhl24_ptm_blend_complete_guest((A)c,e,m,i,p);}
static A partial(void*c,void*e,void*m,A i,A p,float w){return nhl24_ptm_blend_partial_guest((A)c,e,m,i,p,w);}
static A trajectory(void*c,void*e,void*m,A i,const float*v,float w){
    (void)m;return nhl24_ptm_trajectory_guest((A)c,e,i,v,w);
}
static void prepare(void*c,struct ptm_transition*s,void*e){nhl24_ptm_prepare_offsets_guest((A)c,s,e);}
PTM_EXPORT ABI A nhl24_ptm_transition_evaluate_guest(A base,struct ptm_transition *state,
        void *evaluation,const void *selection,A input,float delta) {
    const struct ptm_evaluate_services services={(void*)base,invalid,import,position,curve,
        complete,partial,trajectory,prepare};
    return nhl27_ptm_transition_evaluate(state,evaluation,selection,input,delta,&services);
}

/* Native 31166f: the operation handle is at frame+50, evaluation at +30,
 * selection at +260 and elapsed step at +28. The assembly guard admits only
 * an initialized, explicitly enabled match. It also owns automatic creation. */
PTM_EXPORT ABI void nhl24_ptm_match_evaluate_frame_guest(A base,void *match,void *frame){
    panic_u8 *f=(panic_u8*)frame;
    struct ptm_match_extension *e=(void*)((panic_u8*)match+PTM_NATIVE_MATCH_SIZE);
    if(!e->transition && nhl24_ptm_auto_due(match,f+0x260,*(float*)(f+0x28)))
        auto_create_guest(base,match,*(void**)(f+0x30),f+0x260,&e->transition);
    if(e->transition)
        *(A*)(f+0x50)=nhl24_ptm_transition_evaluate_guest(base,e->transition,
            *(void**)(f+0x30),f+0x260,*(A*)(f+0x50),*(float*)(f+0x28));
}

static struct ptm_transition *allocate(void *context,A bytes){
    A base=(A)context;void *allocator=*(void**)(base+0xaa3dbb0);
    typedef void*(ABI *allocate_fn)(void*,A,const char*,unsigned,unsigned,unsigned);
    return ((allocate_fn)(*(A**)allocator)[3])(allocator,bytes,(const char*)(base+0x90f1c31),1,16,0);
}
static void destroy(void*c,void*p){(void)c;nhl24_ptm_controller_destroy(p);}
static void capture(void*c,struct ptm_transition*s,void*e){nhl24_ptm_transition_capture_guest((A)c,s,e);}
static void auto_create_guest(A base,void*match,void*evaluation,const void*selection,struct ptm_transition**slot){
    const struct ptm_creation_services services={(void*)base,allocate,destroy,capture};
    nhl24_ptm_auto_create(match,evaluation,selection,slot,*(A*)(base+0xbe14630),base+0x913a9d4,&services);
}
PTM_EXPORT ABI void nhl24_ptm_transition_create_guest(A base,struct ptm_transition **slot,
        const struct ptm_creation *args,void *evaluation){
    const struct ptm_creation_services services={(void*)base,allocate,destroy,capture};
    nhl27_ptm_transition_create(slot,args,evaluation,&services);
}

/* Source 23a640's five-way selection, with the native table and selector
 * layouts qualified at 30e025..30e077 and 30ebbf. */
PTM_EXPORT void *nhl24_ptm_selected_path(void *match,void *extra_path){
    panic_u8 *m=(panic_u8*)match;
    panic_u8 *selector=*(panic_u8**)(m+0x11a8);
    int category=selector?*(int*)(selector+0x18):0;
    if(category<0)category=0;if(category>4)category=4;
    /* Native +330 is the table pointer, not a fifth category index. */
    if(category==4)return extra_path?extra_path:m+0x13c0;
    int index=((int*)(m+0x320))[category];
    if(index==-1)return m+0x13c0;
    return *(panic_u8**)(m+0x330)+(__INTPTR_TYPE__)index*0x80+0x10;
}

PTM_EXPORT void nhl24_ptm_prepare_target_path(void *match,float remaining,void *extra_path){
    float *target=(float*)((panic_u8*)match+0x1440);
    __m128 difference=_mm_sub_ps(_mm_loadu_ps(target),_mm_set1_ps(0x1.fffffep127f));
    difference=_mm_and_ps(difference,_mm_castsi128_ps(_mm_set1_epi32(0x7fffffff)));
    if(!_mm_movemask_ps(_mm_cmpnle_ps(difference,_mm_set1_ps(0x1p-16f))))
        nhl27_ptm_sample_path(target,nhl24_ptm_selected_path(match,extra_path),remaining);
}

PTM_EXPORT ABI void nhl24_ptm_start_transition_guest(A base,struct ptm_transition **slot,
        void *match,void *evaluation,const void *selection,void *controller,const float *target,
        float remaining,const panic_u8 *trajectory_flag,void *extra_path){
    panic_u8 *m=(panic_u8*)match;const panic_u8 *s=(const panic_u8*)selection;
    nhl24_ptm_prepare_target_path(match,remaining,extra_path);
    void **clips=*(void***)(*(panic_u8*const*)(s+0x10)+0xb0);
    panic_u8 *parameters=*(panic_u8**)(m+0x11f0);
    const struct ptm_creation args={match,controller,clips[*(const int*)s],m+0x12a0,m+0x14e0,
        *(A*)(base+0xbe14630),base+0x913a9d4,nhl24_ptm_clip_position(selection)+0.0f,
        remaining**(float*)(m+0x88),(unsigned*)(parameters+0xdc),trajectory_flag,s[0x18],target};
    nhl24_ptm_transition_create_guest(base,slot,&args,evaluation);
}
