#include "ptm_transition.h"

/* Source 2801e4..280398: retain the constructor's temporary controller,
 * snapshot configuration after allocation, retain the stored controller,
 * publish, release the temporary, then capture through the published slot. */
PTM_EXPORT void nhl27_ptm_transition_initialize(struct ptm_transition *state,struct ptm_transition **slot,const struct ptm_creation *args,
        void *evaluation,const struct ptm_creation_services *services) {
    void *controller=args->controller;
    if(controller)__atomic_add_fetch((int*)((panic_u8*)controller+8),1,__ATOMIC_SEQ_CST);
    unsigned curve=*args->curve;
    panic_u8 feature=*args->feature_flag;
    state->match=args->match;state->controller=controller;
    if(controller)__atomic_add_fetch((int*)((panic_u8*)controller+8),1,__ATOMIC_SEQ_CST);
    state->clip=args->clip;state->pose=0;state->pose_handle=args->invalid_handle;
    state->trajectory=args->trajectory;state->match_state=args->match_state;
    state->allocator=args->allocator;state->inline_storage=(panic_address)state->inline_vectors;
    state->begin=state->end=state->inline_storage;state->capacity=state->inline_storage+sizeof(state->inline_vectors);
    state->clip_time=args->clip_time;state->mirrored=args->mirrored;
    state->prepare_offsets=1;state->feature_flag=feature;
    for(unsigned n=0;n<4;n++)state->target[n]=args->target[n];
    state->duration=args->duration;state->curve=curve;*slot=state;
    if(controller && __atomic_sub_fetch((int*)((panic_u8*)controller+8),1,__ATOMIC_SEQ_CST)==0)
        services->destroy_controller(services->context,controller);
    services->capture(services->context,*slot,evaluation);
}

PTM_EXPORT void nhl27_ptm_transition_create(struct ptm_transition **slot,const struct ptm_creation *args,
        void *evaluation,const struct ptm_creation_services *services) {
    struct ptm_transition *state=services->allocate(services->context,sizeof(*state));
    nhl27_ptm_transition_initialize(state,slot,args,evaluation,services);
}

static void release_controller(void *controller,const struct ptm_transition_services *services) {
    if(controller && __atomic_sub_fetch((int*)((panic_u8*)controller+8),1,__ATOMIC_SEQ_CST)==0)
        services->destroy_controller(services->context,controller);
}

/* 280440 clears the first references before releasing them, then re-reads
 * after vector cleanup. Reentrant callbacks may have republished references. */
PTM_EXPORT void nhl27_ptm_transition_destruct(struct ptm_transition *state,
                                            const struct ptm_transition_services *services) {
    if(state->pose) {
        void *pose=state->pose;state->pose=0;
        services->release_pose(services->context,pose);
    }
    if(state->controller) {
        void *controller=state->controller;state->controller=0;
        release_controller(controller,services);
    }
    if(state->begin && state->begin!=state->inline_storage)
        services->release_vector(services->context,&state->allocator,state->begin,state->capacity-state->begin);
    if(state->pose)services->release_pose(services->context,state->pose);
    release_controller(state->controller,services);
}

/* 2803c0 publishes null only after destructor and allocator callbacks. */
PTM_EXPORT void nhl27_ptm_transition_clear(struct ptm_transition **slot,
                                         const struct ptm_transition_services *services) {
    struct ptm_transition *state=*slot;
    if(state) {
        nhl27_ptm_transition_destruct(state,services);
        services->free_transition(services->context,state);
    }
    *slot=0;
}

/* 280410 only exposes prepared offsets when their count matches the live rig. */
PTM_EXPORT const panic_address *nhl27_ptm_transition_offsets(const struct ptm_transition *state,int joints) {
    if(!state)return 0;
    return (unsigned)((state->end-state->begin)>>4)==(unsigned)joints?&state->begin:0;
}

/* 2804d0 samples the outgoing clip at the end of the blend, evaluates the
 * previous controller at zero delta, and retains the resulting pose. */
PTM_EXPORT void nhl27_ptm_transition_capture(struct ptm_transition *state,void *evaluation,
                                            const struct ptm_capture_services *services) {
    void *context=services->context;
    panic_address clip=services->sample_clip(context,state,evaluation,state->clip_time+state->duration);
    panic_address prior=services->evaluate_controller(context,state->controller,evaluation);
    panic_address combined=services->combine(context,evaluation,state->controller,prior,clip);
    void *pose=services->create_pose(context,state->controller);
    void *old=state->pose;
    void *published=old;
    if(pose!=old) {
        if(pose)services->retain_pose(context,pose);
        state->pose=pose;published=pose;
        if(old) {
            services->release_pose(context,old);
            published=state->pose;
        }
    }
    state->pose_handle=services->write_pose(context,evaluation,combined,published);
}

/* 27fdb0 is the actual transition evaluator tail-called by 27fa30. The
 * terminal comparison is JBE after UCOMISS, so unordered also takes that
 * branch. Preserve the original arithmetic and callback ordering. */
PTM_EXPORT panic_address nhl27_ptm_transition_evaluate(struct ptm_transition *state,void *evaluation,
        const void *selection,panic_address input,float delta,const struct ptm_evaluate_services *services) {
    void *context=services->context;
    if(state->pose_handle==services->invalid_handle(context) && state->pose)
        state->pose_handle=services->import_pose(context,evaluation,state->pose);
    float duration=state->duration;
    float threshold=duration-0x1p-16f;
    float elapsed=(services->clip_position(context,selection)+delta)-state->clip_time;
    float weight;
    panic_address result;
    if(!(threshold>elapsed)) {
        result=services->blend_complete(context,evaluation,state->match,input,state->pose_handle);
        weight=1.0f;
    } else {
        float progress=elapsed/duration;
        progress=0.0f>progress?0.0f:progress; /* MAXSS with progress as second operand. */
        weight=services->curve(context,1.0f-progress,state->curve);
        result=services->blend_partial(context,evaluation,state->match,input,state->pose_handle,weight);
    }
    if(state->feature_flag==1) {
        float movement[4];
        for(unsigned n=0;n<4;n++)movement[n]=delta*state->target[n];
        result=services->apply_trajectory(context,evaluation,state->match,result,movement,weight);
    }
    state->pose_handle=services->invalid_handle(context);
    services->prepare_offsets(context,state,evaluation);
    return result;
}
