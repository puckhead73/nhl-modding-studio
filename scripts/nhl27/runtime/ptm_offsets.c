/* NHL27 offset preparation using NHL24 pose, joint and arena services. */
#include "ptm_native.h"
#include <smmintrin.h>
typedef panic_address A;
typedef panic_u8 B;
#define ABI PTM_GAME_ABI
extern ABI void nhl27_ptm_sincos(float,float*,float*);
extern ABI void *nhl24_ptm_pose_create_guest(A,void*);
extern void nhl24_ptm_pose_retain(void*);
extern void nhl24_ptm_pose_release(void*);
extern A nhl24_ptm_controller_evaluate(void*,void*);
extern ABI A nhl24_ptm_sample_clip_guest(A,void*,void*,void*,float,const B*);
typedef void *(ABI *tls_fn)(void*);
typedef void (ABI *reset_fn)(void*);
typedef void (ABI *root_fn)(void*,void*);
typedef void *(ABI *layout_fn)(void*);
typedef void (ABI *joint_fn)(void*,void*,void*,void*,int);
typedef void *(ABI *alloc_fn)(void*,A,const char*,unsigned,unsigned,unsigned);
typedef void (ABI *free_fn)(void*,void*,A);
struct offset_scope {void *arena;A handle;void *evaluation[3];B flag;B padding[7];};

/* Four native TLS arenas, acquired in index order. Null is the engine's
 * exhausted-pool result; the native flush routine handles that result. */
PTM_EXPORT void nhl24_ptm_offset_scope_begin(struct offset_scope *scope,void *tls,void *context,A invalid) {
    B *t=(B*)tls;void *arena=0;
    for(unsigned n=0;n<4;n++)if(!t[0x6280+n]){
        t[0x6280+n]=1;arena=t+0x13c0+n*0x13b0;break;
    }
    scope->arena=arena;scope->handle=invalid;scope->evaluation[0]=arena;
    scope->evaluation[1]=context;scope->evaluation[2]=0;scope->flag=0;
}
PTM_EXPORT void nhl24_ptm_offset_scope_end(struct offset_scope *scope,void *tls,reset_fn reset) {
    if(!scope->arena)return;
    B *t=(B*)tls;
    for(unsigned n=0;n<4;n++)if(scope->arena==t+0x13c0+n*0x13b0){
        reset(scope->arena);t[0x6280+n]=0;
    }
    scope->arena=0;
}

/* Source 2808ce writes the integral and fractional translation separately.
 * Its subtraction occurs before selection; preserve that for non-finite data. */
PTM_EXPORT __attribute__((target("sse4.1"))) void nhl27_ptm_offset_root(float *root,float *integral,
        const float *position,const float *rotation) {
    __m128 p=_mm_loadu_ps(position);
    if(integral){
        __m128 magnitude=_mm_and_ps(p,_mm_castsi128_ps(_mm_set1_epi32(0x7fffffff)));
        __m128 mask=_mm_cmplt_ps(_mm_set1_ps(20),magnitude);
        __m128 floor=_mm_floor_ps(p);
        _mm_storeu_ps(integral+8,_mm_and_ps(mask,floor));
        p=_mm_blendv_ps(p,_mm_sub_ps(p,floor),mask);
    }
    _mm_storeu_ps(root+8,p);_mm_storeu_ps(root,_mm_set1_ps(1));
    _mm_storeu_ps(root+4,_mm_loadu_ps(rotation));
}

/* Same lane ordering as 280c43..280cc5 and 280e38..280eae. */
PTM_EXPORT void nhl27_ptm_offset_world(float *out,const float *root,const float *joint) {
    __m128 scaled=_mm_mul_ps(_mm_loadu_ps(root),_mm_loadu_ps(joint+8));
    __m128 q=_mm_loadu_ps(root+4);
    __m128 qshift=_mm_shuffle_ps(q,q,0xc9);
    __m128 shift=_mm_shuffle_ps(scaled,scaled,0xc9);
    __m128 cross=_mm_sub_ps(_mm_mul_ps(q,shift),_mm_mul_ps(qshift,scaled));
    cross=_mm_shuffle_ps(cross,cross,0xc9);
    __m128 intermediate=_mm_add_ps(_mm_mul_ps(_mm_shuffle_ps(q,q,0xff),scaled),cross);
    shift=_mm_shuffle_ps(intermediate,intermediate,0xc9);
    cross=_mm_sub_ps(_mm_mul_ps(q,shift),_mm_mul_ps(qshift,intermediate));
    cross=_mm_add_ps(cross,cross);cross=_mm_shuffle_ps(cross,cross,0xc9);
    _mm_storeu_ps(out,_mm_add_ps(_mm_add_ps(scaled,cross),_mm_loadu_ps(root+8)));
}

static void joint_position(A base,void *pose,int id,float *out) {
    B *meta=*(B**)((B*)pose+0x20);
    int *mapping=*(int**)(meta+0x98);
    void *roots[2];
    if(*mapping==id){
        ((root_fn)(base+0x2b9310))(roots,pose);
        for(unsigned n=0;n<4;n++)out[n]=((float*)roots[0])[8+n];
    }else{
        float transform[12];
        void *layout=((layout_fn)(base+0x2b4c30))(*(void**)(meta+0x100));
        ((joint_fn)(base+0x8b1a60))(transform,pose,layout,mapping,id);
        ((root_fn)(base+0x2b9310))(roots,pose);
        nhl27_ptm_offset_world(out,roots[0],transform);
    }
}

/* Reset-to-count form of source 1f46f0. Existing live entries have already
 * been discarded, so an overflow allocation is exactly count * 16 bytes. */
PTM_EXPORT void nhl24_ptm_offset_resize(struct ptm_transition *state,unsigned count,void *allocator,
        alloc_fn allocate,free_fn release,const char *label) {
    state->end=state->begin;
    if(count>(unsigned)((state->capacity-state->begin)>>4)){
        void *replacement=allocate(allocator,(A)count*16,label,1,16,0);
        for(A n=0;n<(A)count*4;n++)((float*)replacement)[n]=0;
        if(state->begin && state->begin!=state->inline_storage)
            release(allocator,(void*)state->begin,state->capacity-state->begin);
        state->begin=(A)replacement;state->capacity=state->begin+(A)count*16;
    }else for(A n=0;n<(A)count*4;n++)((float*)state->begin)[n]=0;
    state->end=state->begin+(A)count*16;
}

PTM_EXPORT ABI void nhl24_ptm_prepare_offsets_guest(A base,struct ptm_transition *state,void *evaluation) {
    if(state->prepare_offsets!=1)return;
    B *parameters=*(B**)((B*)state->match+0x11f0);
    if((*(B**)(parameters+0x80))[0x28]!=1)return;
    void *tls=((tls_fn)(base+0x89cd40))(*(void**)(base+0xac89f98));
    struct offset_scope scope;
    nhl24_ptm_offset_scope_begin(&scope,tls,((void**)evaluation)[1],*(A*)(base+0xbe14630));
    void *previous=nhl24_ptm_pose_create_guest(base,state->controller);
    nhl24_ptm_pose_retain(previous);
    scope.handle=nhl24_ptm_controller_evaluate(state->controller,scope.evaluation);
    ((void(ABI*)(void*,void*))(base+0x8a0a90))(&scope,previous);
    void *incoming=nhl24_ptm_pose_create_guest(base,state->match);
    nhl24_ptm_pose_retain(incoming);
    scope.handle=nhl24_ptm_sample_clip_guest(base,state->match,state->clip,scope.evaluation,
        state->clip_time+state->duration,&state->mirrored);
    ((void(ABI*)(void*,void*))(base+0x8a0a90))(&scope,incoming);
    const float *position=(float*)((B*)state->trajectory+0x1a0);
    float sine,cosine;
    nhl27_ptm_sincos(position[3]*0.5f,&sine,&cosine);
    /* Retain source multiply semantics (including signed zero/NaN lanes). */
    float rotation[4]={sine*0.0f,sine,sine*0.0f,cosine};
    void *roots[2];
    ((root_fn)(base+0x2fa840))(roots,previous);
    nhl27_ptm_offset_root(roots[0],roots[1],position,rotation);
    ((root_fn)(base+0x2fa840))(roots,incoming);
    nhl27_ptm_offset_root(roots[0],roots[1],position,rotation);
    unsigned count=*(unsigned*)((B*)state->match_state+0x200);
    void *allocator=*(void**)(base+0xaa3dbb0);A *vtable=*(A**)allocator;
    nhl24_ptm_offset_resize(state,count,allocator,(alloc_fn)vtable[3],(free_fn)vtable[4],(const char*)(base+0x913a9d4));
    for(int n=0;n<*(int*)((B*)state->match_state+0x200);n++){
        B *records=*(B**)(*(B**)((B*)state->match_state+0x208)+0x10);
        int id=*(int*)(records+(A)n*0x260+0x20);
        float a[4],b[4];joint_position(base,incoming,id,a);joint_position(base,previous,id,b);
        for(unsigned lane=0;lane<4;lane++)((float*)state->begin)[n*4+lane]=b[lane]-a[lane];
    }
    state->prepare_offsets=0;
    nhl24_ptm_pose_release(incoming);nhl24_ptm_pose_release(previous);
    nhl24_ptm_offset_scope_end(&scope,tls,(reset_fn)(base+0x89ec10));
}
