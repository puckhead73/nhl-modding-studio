/* NHL24 1.60 PTM resource services. Original match evaluator 30d9b0 uses
 * pose resources with +18 refcounts; controllers use +8. Keep them distinct. */
#include "ptm_native.h"
#if defined(_WIN32)
#define GAME_ABI __attribute__((sysv_abi))
#else
#define GAME_ABI
#endif
typedef panic_address address;
typedef panic_u8 byte;
typedef void (GAME_ABI *destroy_fn)(void*);
typedef void *(GAME_ABI *pose_create_fn)(void*,void*);
typedef address (GAME_ABI *evaluate_fn)(void*,void*,float);
typedef address (GAME_ABI *combine_fn)(void*,address,address,unsigned);
typedef address (GAME_ABI *write_fn)(void*,address,void*);

PTM_EXPORT void nhl24_ptm_pose_retain(void *pose) {
    if(pose)__atomic_add_fetch((int*)((byte*)pose+0x18),1,__ATOMIC_SEQ_CST);
}

PTM_EXPORT void nhl24_ptm_pose_release(void *pose) {
    if(pose && __atomic_sub_fetch((int*)((byte*)pose+0x18),1,__ATOMIC_SEQ_CST)==0)
        ((destroy_fn)(*(address**)pose)[3])(pose);
}

PTM_EXPORT void nhl24_ptm_controller_destroy(void *controller) {
    ((destroy_fn)(*(address**)controller)[4])(controller);
}

PTM_EXPORT address nhl24_ptm_controller_evaluate(void *controller,void *evaluation) {
    return ((evaluate_fn)(*(address**)controller)[11])(controller,evaluation,0.0f);
}

/* 312b08..312bce inlines 8bb850's skeleton-dependent pose allocation.
 * The native allocator retains ownership of pose layout and alignment. */
PTM_EXPORT void *nhl24_ptm_pose_create(void *controller,void *allocator,pose_create_fn create) {
    byte *owner=*(byte**)((byte*)controller+0x30);
    return create(*(void**)(owner+0x20),allocator);
}

PTM_EXPORT GAME_ABI void *nhl24_ptm_pose_create_guest(address base,void *controller) {
    return nhl24_ptm_pose_create(controller,*(void**)(base+0xaa3dbb0),(pose_create_fn)(base+0x8bb850));
}

/* Native compose call 773ad0 takes the skeleton remap identifier from the
 * controller's properties. Source 6cdd20 hides that lookup in its helper. */
PTM_EXPORT address nhl24_ptm_pose_combine(void *evaluation,void *controller,address prior,address clip,
                                        combine_fn combine) {
    byte *properties=*(byte**)((byte*)controller+0x40);
    byte *mapping=*(byte**)(properties+0x98);
    unsigned id=mapping?*(unsigned*)(mapping+4):0;
    return combine(*(void**)evaluation,prior,clip,id);
}

PTM_EXPORT GAME_ABI address nhl24_ptm_pose_combine_guest(address base,void *evaluation,
                                                       void *controller,address prior,address clip) {
    return nhl24_ptm_pose_combine(evaluation,controller,prior,clip,(combine_fn)(base+0x773ad0));
}

PTM_EXPORT GAME_ABI address nhl24_ptm_pose_write_guest(address base,void *evaluation,address handle,void *pose) {
    return ((write_fn)(base+0x7737f0))(*(void**)evaluation,handle,pose);
}

typedef void (GAME_ABI *controller_create_fn)(void**,void*,void*);
typedef void (GAME_ABI *controller_seek_fn)(void*,float);

static void release_controller_reference(void *controller) {
    if(controller && __atomic_sub_fetch((int*)((byte*)controller+8),1,__ATOMIC_SEQ_CST)==0)
        nhl24_ptm_controller_destroy(controller);
}

/* Source 27fa30's automatic end-of-clip branch. Native selections retain an
 * unpacked signed clip index and frame index; the terminal time is clip+90. */
PTM_EXPORT int nhl24_ptm_auto_due(void *match,const void *selection,float delta){
    byte *parameters=*(byte**)((byte*)match+0x11f0);
    if(!*(void**)(parameters+0xb8))return 0;
    const byte *s=selection;
    byte *clip=((byte**)*(void**)(*(byte*const*)(s+0x10)+0xb0))[*(const int*)s];
    float threshold=(*(float*)(clip+0x90)-*(float*)(parameters+0xd8))-0x1p-16f;
    float time=*(float*)(*(byte**)(clip+0x18)+(address)*(const int*)(s+4)*16+4);
    float elapsed=delta+(time+*(const float*)(s+0x1c));
    return elapsed>=threshold;
}

/* Source 27fb10 creates its controller from a retained pose/property snapshot,
 * allocates first, then reads timing/configuration. The automatic transition
 * has zero trajectory and feature flag regardless of the manual flag. */
PTM_EXPORT void nhl24_ptm_auto_create(void *match,void *evaluation,const void *selection,
        struct ptm_transition **slot,address invalid,address allocator,
        const struct ptm_creation_services *services){
    byte *m=match;const byte*s=selection;
    void *asset=*(void**)(*(byte**)(m+0x11f0)+0xb8);
    void *snapshot[5]={*(void**)(m+0x30),0,*(void**)(m+0x40),0,((void**)evaluation)[1]};
    nhl24_ptm_pose_retain(snapshot[0]);
    snapshot[1]=*(void**)(m+0x38);
    if(snapshot[1])__atomic_add_fetch((int*)((byte*)snapshot[1]+8),1,__ATOMIC_SEQ_CST);
    void *controller=0;
    ((controller_create_fn)(*(address**)asset)[23])(&controller,asset,snapshot);
    struct ptm_transition *state=services->allocate(services->context,sizeof(*state));
    byte *parameters=*(byte**)(m+0x11f0);
    byte *clip=((byte**)*(void**)(*(byte*const*)(s+0x10)+0xb0))[*(const int*)s];
    float duration=*(float*)(parameters+0xd8);
    const float zero[4]={0,0,0,0};const byte disabled=0;
    const struct ptm_creation args={match,controller,clip,m+0x12a0,m+0x14e0,
        invalid,allocator,*(float*)(clip+0x90)-duration,duration,
        (unsigned*)(parameters+0xdc),&disabled,s[0x18],zero};
    nhl27_ptm_transition_initialize(state,slot,&args,evaluation,services);
    release_controller_reference(controller);
    if(snapshot[1] && __atomic_fetch_sub((int*)((byte*)snapshot[1]+8),1,__ATOMIC_SEQ_CST)<=1)
        ((destroy_fn)(*(address**)snapshot[1])[2])(snapshot[1]);
    nhl24_ptm_pose_release(snapshot[0]);
}

/* Source 280f20 expressed through native clip and controller interfaces.
 * Native 3127d6..312add demonstrates the context scope, clip asset +8,
 * factory +48 and seek interface +10. Do not use source clip +28 here. */
PTM_EXPORT address nhl24_ptm_sample_clip(void *match,void *clip,void *evaluation,float time,
                                        const panic_u8 *mirrored,const struct ptm_native_clip_services *services) {
    struct ptm_mirror_scope scope;
    void *asset=*(void**)((byte*)clip+8);
    void *context=((void**)evaluation)[1];
    services->begin_mirror(&scope,context,*mirrored);
    void *snapshot[5];
    snapshot[0]=*(void**)((byte*)match+0x30);
    nhl24_ptm_pose_retain(snapshot[0]);
    snapshot[1]=*(void**)((byte*)match+0x38);
    if(snapshot[1])__atomic_add_fetch((int*)((byte*)snapshot[1]+8),1,__ATOMIC_SEQ_CST);
    snapshot[2]=*(void**)((byte*)match+0x40);
    snapshot[3]=0;snapshot[4]=context;
    void *controller=0;
    ((controller_create_fn)(*(address**)asset)[9])(&controller,asset,snapshot);
    void *seek=(byte*)controller+0x10;
    ((controller_seek_fn)(*(address**)seek)[2])(seek,time);
    if(*mirrored==1 && (*scope.word&0xffffu)<=(scope.saved&0xffffu)+1u) {
        void *replacement=services->create_mirror(controller,snapshot,0);
        if(replacement!=controller) {
            if(replacement)__atomic_add_fetch((int*)((byte*)replacement+8),1,__ATOMIC_SEQ_CST);
            void *previous=controller;controller=replacement;
            release_controller_reference(previous);
        }
    }
    address result=nhl24_ptm_controller_evaluate(controller,evaluation);
    release_controller_reference(controller);
    /* Properties use their own reference-counted base and destructor slot. */
    if(snapshot[1] && __atomic_fetch_sub((int*)((byte*)snapshot[1]+8),1,__ATOMIC_SEQ_CST)<=1)
        ((destroy_fn)(*(address**)snapshot[1])[2])(snapshot[1]);
    nhl24_ptm_pose_release(snapshot[0]);
    byte *word=(byte*)scope.word;
    word[2]=(byte)(scope.saved>>16);word[3]=(byte)(scope.saved>>24);
    if(scope.reset_count)*(unsigned short*)word=0;
    return result;
}

PTM_EXPORT GAME_ABI address nhl24_ptm_sample_clip_guest(address base,void *match,void *clip,
                                                       void *evaluation,float time,const panic_u8 *mirrored) {
    const struct ptm_native_clip_services services={
        (void(GAME_ABI*)(struct ptm_mirror_scope*,void*,panic_u8))(base+0x2a1780),
        (void*(GAME_ABI*)(void*,void*,void*))(base+0xa0f100)};
    return nhl24_ptm_sample_clip(match,clip,evaluation,time,mirrored,&services);
}

static address capture_clip(void *context,struct ptm_transition *state,void *evaluation,float time) {
    return nhl24_ptm_sample_clip_guest((address)context,state->match,state->clip,evaluation,time,&state->mirrored);
}
static address capture_controller(void *context,void *controller,void *evaluation) {
    (void)context;
    return nhl24_ptm_controller_evaluate(controller,evaluation);
}
static address capture_combine(void *context,void *evaluation,void *controller,address prior,address clip) {
    return nhl24_ptm_pose_combine_guest((address)context,evaluation,controller,prior,clip);
}
static void *capture_create(void *context,void *controller) {
    return nhl24_ptm_pose_create_guest((address)context,controller);
}
static void capture_retain(void *context,void *pose) {(void)context;nhl24_ptm_pose_retain(pose);}
static void capture_release(void *context,void *pose) {(void)context;nhl24_ptm_pose_release(pose);}
static address capture_write(void *context,void *evaluation,address handle,void *pose) {
    return nhl24_ptm_pose_write_guest((address)context,evaluation,handle,pose);
}

PTM_EXPORT GAME_ABI void nhl24_ptm_transition_capture_guest(address base,struct ptm_transition *state,void *evaluation) {
    const struct ptm_capture_services services={(void*)base,capture_clip,capture_controller,capture_combine,
        capture_create,capture_retain,capture_release,capture_write};
    nhl27_ptm_transition_capture(state,evaluation,&services);
}

typedef void (GAME_ABI *free_fn)(void*,void*,address);
static void destroy_controller_adapter(void *context,void *controller) {
    (void)context;nhl24_ptm_controller_destroy(controller);
}
static void native_free(void *context,void *allocation,address size) {
    void *allocator=*(void**)((address)context+0xaa3dbb0);
    ((free_fn)(*(address**)allocator)[4])(allocator,allocation,size);
}
static void free_vector_adapter(void *context,void *allocator,address begin,address bytes) {
    (void)allocator;native_free(context,(void*)begin,bytes);
}
static void free_transition_adapter(void *context,struct ptm_transition *state) {
    native_free(context,state,0);
}
PTM_EXPORT GAME_ABI void nhl24_ptm_transition_clear_guest(address base,struct ptm_transition **slot) {
    const struct ptm_transition_services services={(void*)base,capture_release,destroy_controller_adapter,
        free_vector_adapter,free_transition_adapter};
    nhl27_ptm_transition_clear(slot,&services);
}

/* Native selection 30d9b0: index/frame are signed 32-bit values, database
 * at +10, fractional clip time at +1c. Clip frame records have stride 16. */
PTM_EXPORT float nhl24_ptm_clip_position(const void *selection) {
    const byte *s=(const byte*)selection;
    const byte *database=*(const byte*const*)(s+0x10);
    const byte *const *clips=*(const byte*const*const*)(database+0xb0);
    const byte *clip=clips[*(const int*)s];
    const byte *frames=*(const byte*const*)(clip+0x18);
    return *(const float*)(frames+(__INTPTR_TYPE__)*(const int*)(s+4)*16+4)+*(const float*)(s+0x1c);
}

typedef void (GAME_ABI *pose_view_fn)(void*,void*,void*);
typedef address (GAME_ABI *pose_import_fn)(void*,void*);
PTM_EXPORT address nhl24_ptm_pose_import(void *evaluation,void *pose,pose_view_fn make_view,pose_import_fn import) {
    void *view[3]={pose,0,0};
    nhl24_ptm_pose_retain(pose);
    make_view(&view[1],*(void**)((byte*)pose+0x20),0);
    address result=import(*(void**)evaluation,view);
    if(view[1] && __atomic_fetch_sub((int*)((byte*)view[1]+8),1,__ATOMIC_SEQ_CST)<=1)
        ((destroy_fn)(*(address**)view[1])[2])(view[1]);
    nhl24_ptm_pose_release(view[0]);
    return result;
}

PTM_EXPORT GAME_ABI address nhl24_ptm_pose_import_guest(address base,void *evaluation,void *pose) {
    return nhl24_ptm_pose_import(evaluation,pose,(pose_view_fn)(base+0x8c3770),(pose_import_fn)(base+0x769cc0));
}

/* The three sampled curve tables (21 floats each) are byte-identical in both
 * images. Curve 3 is the special linear path and never indexes that table. */
typedef float (GAME_ABI *curve_fn)(unsigned,float);
PTM_EXPORT GAME_ABI float nhl24_ptm_curve_guest(address base,unsigned curve,float value) {
    return ((curve_fn)(base+0x753c20))(curve,value);
}

typedef address (GAME_ABI *partial_fn)(void*,void*,address,address,void*);
PTM_EXPORT GAME_ABI address nhl24_ptm_blend_partial_guest(address base,void *evaluation,void *match,
                                                        address input,address captured,float weight) {
    void *arena=*(void**)evaluation;
    if(!arena)return *(address*)(base+0xbe14630);
    struct {address vtable;float value;unsigned padding;} parameter={base+0xa4475a8,weight,0};
    return ((partial_fn)(base+0x7772c0))(arena,(byte*)match+0x38,input,captured,&parameter);
}
PTM_EXPORT GAME_ABI address nhl24_ptm_blend_complete_guest(address base,void *evaluation,void *match,
                                                         address input,address captured) {
    /* Same remap lookup as compose, with the terminal blend operation. */
    return nhl24_ptm_pose_combine(evaluation,match,input,captured,(combine_fn)(base+0x7684e0));
}
