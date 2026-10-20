/* Source 4a51a80 through its tail call to 4a53e10. */
#include "panic_update.h"
#if defined(_WIN32)
#define EXPORT __declspec(dllexport)
#else
#define EXPORT __attribute__((visibility("default")))
#endif

_Static_assert(sizeof(struct panic_arcade_request)==0x2c,"source arcade request stride");

/* Source 453e350/453e080 intentionally acquire the optional manager twice.
 * An absent manager/record service allows availability, but cannot block a
 * specific request. Availability ignores state; blocking requires state 2. */
static const struct panic_arcade_request *request_records(const struct panic_request_services *s) {
    void *c=s->context;
    if(!s->manager(c))return 0;
    return s->records(c,s->manager(c));
}
EXPORT panic_u8 nhl27_panic_request_available(void *actor,const struct panic_request_services *s) {
    const struct panic_arcade_request *r=request_records(s);
    if(!r)return 1;
    panic_u32 id=*(panic_u32*)((panic_u8*)actor+0x3c);
    for(unsigned i=0;i<16;i++) {
        if(r[i].actor_id!=id)continue;
        panic_u32 kind=r[i].kind;
        if(kind==11 || (kind&~2u)==13)return 0;
    }
    return 1;
}
EXPORT panic_u8 nhl27_panic_request_blocked(void *actor,panic_u32 kind,
        const struct panic_request_services *s) {
    const struct panic_arcade_request *r=request_records(s);
    if(!r)return 0;
    panic_u32 id=*(panic_u32*)((panic_u8*)actor+0x3c);
    for(unsigned i=0;i<16;i++)
        if(r[i].state==2 && r[i].actor_id==id && r[i].kind==kind)return 1;
    return 0;
}

/* Source 455e7b0. Byte-returning calls test AL, scalar fields test the full
 * dword, and the request option is exactly one. Do not fold or cache queries:
 * callbacks may change actor status, the role, or the current owner. */
EXPORT panic_u8 nhl27_panic_availability_tail(void *actor,
        const struct panic_availability_services *s) {
    void *c=s->context;
#define AVAIL(q) s->query(c,PANIC_AVAIL_##q,actor)
    if((panic_u8)AVAIL(MANAGER_BLOCKED))return 0;
    if((panic_u32)AVAIL(FLAG_B0) && !(panic_u32)AVAIL(FLAG_B8))return 0;
    if((panic_u8)AVAIL(ACTOR_BYTE_40))return 0;
    if((panic_u32)AVAIL(CONTROL) && (panic_u32)AVAIL(CONTROL_MODE)!=1)return 0;
    if((panic_u32)AVAIL(SOURCE_STATUS)==0x11)return 0;
    if((panic_u8)AVAIL(SPECIAL_ANIMATION))return 0;
    if((panic_u32)AVAIL(SOURCE_STATUS)==0x0e)return 0;
    if((panic_u8)AVAIL(GLOBAL_BLOCKED))return 0;
    if((panic_u32)AVAIL(ROLE_KIND)==3)return 0;
    if((panic_u32)AVAIL(FLAG_B4))return 0;
    panic_u8 *owner=(panic_u8*)AVAIL(OWNER);
    if(!owner)return 0;
    if((panic_u8)AVAIL(STATE_CONDITION))return 0;
    if((panic_u8)AVAIL(ACTOR_STATE_BLOCKED))return 0;
    if((panic_u8)AVAIL(ANIMATION_BLOCKED))return 0;
    /* These owner/component offsets are shared by the source/native layouts.
     * Keep the captured owner, but acquire its component after callbacks. */
    panic_u8 *component=*(panic_u8**)(owner+0x98);
    if(component) {
        panic_u32 kind=*(panic_u32*)(component+0x10);
        if(kind==0x4000 || (kind==0x2000 && component[0x28]))return 0;
    }
    if((panic_u8)AVAIL(REQUEST_OPTION)==1 && !(panic_u8)AVAIL(REQUEST_ALLOWED))return 0;
#undef AVAIL
    return 1;
}

/* Source 3f0560. Child evaluation contexts belong to the checking feature;
 * queries can change its active asset, so repeat the source's asset guards. */
EXPORT panic_u8 nhl27_panic_pending_check(void *const *begin,void *const *end,
                                         const struct panic_pending_services *s) {
    if(begin==end || !(panic_u32)(((panic_address)end-(panic_address)begin)>>3))return 0;
    void *c=s->context;
    for(void *const *p=begin;p!=end;++p) {
        void *child=s->resolve(c,*p);
        if(!s->has_asset(c,child))continue;
        if(s->query(c,child,PANIC_CHECK_HIP_STATE))return 1;
        if(!s->has_asset(c,child))continue;
        if((panic_u8)s->query(c,child,PANIC_CHECK_RALLY_REQUEST)) {
            if((panic_u8)s->query(c,child,PANIC_CHECK_INTERACTING))return 1;
            if((panic_u8)s->query(c,child,PANIC_CHECK_FAILED_SHOULDER))return 1;
        }
        if(s->has_asset(c,child) && (panic_u8)s->query(c,child,PANIC_CHECK_PUSH_ENABLED))return 1;
    }
    return 0;
}

/* Source 4a52090: do not pre-sample eligibility. Several services read mutable
 * possession/request state, and the source intentionally queries it twice. */
EXPORT void nhl27_panic_update_feature(struct panic_state *state,const struct panic_feature_services *s){
    void *c=s->context;panic_u8 *feature=s->feature(c,state->actor);
    if(!(panic_u8)s->query(c,PANIC_FEATURE_INITIALIZED,(panic_address)feature,0))return;
    panic_u8 enabled=0;
    if((panic_u8)s->query(c,PANIC_SPECIAL_SESSION,0,0)){
        panic_address actor=state->actor;
        panic_address other=s->query(c,PANIC_SESSION_ACTOR,0,0);
        if((panic_u8)s->query(c,PANIC_SAME_ACTOR,actor,other)){
            other=s->query(c,PANIC_SESSION_ACTOR,0,0);
            if(other && s->query(c,PANIC_SESSION_ACTOR_TYPE,other,0)==0x36)goto done;
        }
    }
    if((panic_u8)s->query(c,PANIC_ANIMATION_FLAG_691,state->actor,0))goto done;
    if(!(panic_u8)s->query(c,PANIC_ACTOR_ALLOWED,state->actor,1))goto done;
    if(!(panic_u8)s->query(c,PANIC_ACTOR_AVAILABLE,state->actor,0))goto done;
    if((panic_u8)s->query(c,PANIC_ANIMATION_REACTION,state->actor,0))goto done;
    if((panic_u8)s->query(c,PANIC_ACTOR_STATE_BLOCKED,state->actor,0))goto done;
    if((panic_u8)s->query(c,PANIC_GAME_BLOCKED,1,0))goto done;
    if(*(panic_u32*)(state->actor+0x48)-5u<=4u)goto done;
    panic_address session=s->query(c,PANIC_SESSION_OBJECT,0,0);
    if((panic_u8)s->query(c,PANIC_SESSION_BLOCKED,session+0x670,0))goto done;
    if((panic_u8)s->query(c,PANIC_ANIMATION_BLOCKED,state->actor,0))goto done;
    if(s->query(c,PANIC_ACTOR_ROLE_KIND,state->actor,0)==3)goto done;
    if((panic_u8)s->query(c,PANIC_GLOBAL_BLOCKED,0,0))goto done;
    if((panic_u8)s->query(c,PANIC_ACTOR_BLOCKED,state->actor,1))goto done;
    if((panic_u8)s->query(c,PANIC_REQUEST_BLOCKED,state->actor,10))goto done;
    enabled=(panic_u8)s->query(c,PANIC_COMPONENT_BLOCKED,state->actor,0)^1u;
done:
    feature[0x18]=enabled;
}

/* Source 4a51930 keeps cooldown, stamp, poke opponent and baseline attributes.
 * Closest-distance storage is resolved again after clearing preconditions. */
EXPORT void nhl27_panic_reset_full(struct panic_state *state,
                                  const struct panic_reset_services *services) {
    void *context=services->context;
    state->duration=state->active_ticks=0;
    state->opponent=0;
    struct panic_bindings *bindings=services->bindings(context);
    if(bindings->requested)*bindings->requested=0;
    if(bindings->stumble)*bindings->stumble=0;
    if(bindings->between_legs)*bindings->between_legs=0;
    if(bindings->type)*bindings->type=0;
    panic_u8 session=services->session_state(context);
    if(state->actor && session==1)
        services->pose(context,state->actor,&state->facing,state->position);
    else {
        state->facing=0;
        for(unsigned n=0;n<4;n++)state->position[n]=0;
    }
    services->clear_preconditions(context,state->actor);
    bindings=services->bindings(context);
    if(bindings->closest_distance)*bindings->closest_distance=0x1.fffffep127f;
    state->flag=0;
    services->reset_alignment(context,state);
    panic_u8 *active=services->feature_active(context,state->actor);
    if(active)*active=0;
}

/* MINSS returns its second operand for unordered/equal inputs. Reordering
 * these comparisons changes NaN and signed-zero results. */
static float closest_min(float a,float b) {return a<b?a:b;}

EXPORT void nhl27_panic_update_closest(struct panic_state *state,
                                      const struct panic_closest_services *services) {
    void *context=services->context;
    struct panic_bindings *bindings=services->bindings(context);
    if(!services->motion_active(context,state->actor))return;
    void *first=services->first_candidate(context,state->actor);
    void *second=services->second_candidate(context,state->actor);
    float cached=services->cached_distance(context,state->actor);
    float first_distance=0x1.fffffep127f,second_distance=0x1.fffffep127f;
    if(first)first_distance=services->candidate_distance(context,first,state->actor);
    if(second)second_distance=services->candidate_distance(context,second,state->actor);
    float third=services->third_distance(context,state->actor);
    if(bindings->closest_distance)
        *bindings->closest_distance=closest_min(cached,
            closest_min(third,closest_min(second_distance,first_distance)));
}

/* Source 4a517d0: keep the six consumers in source order and sample the
 * alignment guard afterwards. Cached active is nonzero; the live asset is
 * exactly one. JAE admits an unordered deadline into the trajectory check. */
EXPORT void nhl27_panic_update_frame(struct panic_state *state,
                                    const struct panic_frame_services *services) {
    void *context=services->context;
    if(!services->cached_active(context,state->actor)) {
        services->reset(context,state);
        return;
    }
    services->reactions(context,state);
    services->preconditions(context,state);
    services->feature(context,state);
    services->closest(context,state);
    services->takeoff(context,state);
    services->alignment(context,state);
    struct panic_bindings *bindings=services->bindings(context);
    if(!bindings->path_lock || *bindings->path_lock!=11)return;
    if(bindings->ptm_active && *bindings->ptm_active==1 &&
       !((float)services->tick(context)>=state->align_deadline)) {
        const struct panic_trajectory_view *trajectory=services->trajectory(context,state->actor);
        void *tracker=services->query_path_tracker(context,state->actor);
        /* Read the span after querying the path tracker, as the source does. */
        if((trajectory->end-trajectory->begin)&(panic_address)0xfffffffe0ULL) {
            float tolerance=services->tolerance(context);
            const float *position=services->tracker_position(context,tracker);
            const float *sample=(const float*)(trajectory->begin+16);
            unsigned within=1;
            for(unsigned n=0;n<3;n++) {
                union {float f;panic_u32 u;} delta={position[n]-sample[n]};
                delta.u&=0x7fffffff;
                within&=(delta.f<=tolerance);
            }
            if(within)return;
        }
    }
    services->reset_alignment(context,state);
}

EXPORT void nhl27_panic_update_reactions(struct panic_state *state,
        const struct panic_update_services *services) {
    void *context=services->context;
    struct panic_bindings *bindings=services->bindings(context);
    if(bindings->interrupt_exit)*bindings->interrupt_exit=0;
    bindings=services->bindings(context);
    int dispatch=1;
    if(bindings->ptm_active && *bindings->ptm_active==1 &&
       services->animation_is_panic(context)) {
        state->active_ticks=(int)((panic_u32)state->active_ticks+1u);
        state->duration=state->duration>1 ? state->duration-1 : 0;
        state->cooldown=services->cooldown_ticks(context);
        bindings=services->bindings(context);
        if(bindings->stumble && *bindings->stumble==1)
            services->active_stumble(context,state);
    } else {
        state->active_ticks=0;
        if(state->duration>0)--state->duration;
        else {
            if(state->cooldown<=0)services->generate_request(context,state);
            dispatch=0;
        }
    }
    if(dispatch) {
        bindings=services->bindings(context);
        if(bindings->type) {
            switch(*bindings->type) {
                case 1:services->skating(context,state);break;
                case 2:services->deke(context,state);break;
                case 3:services->poke(context,state);break;
            }
        }
    }
    services->request_stumble(context,state);
    services->emit_event(context,state);
    int old_cooldown=state->cooldown;
    state->cooldown=old_cooldown>1 ? old_cooldown-1 : 0;
    if(old_cooldown>1) {
        bindings=services->bindings(context);
        if(!bindings->ptm_active || *bindings->ptm_active!=1 ||
           !services->animation_is_panic(context)) {
            bindings=services->bindings(context);
            if(bindings->requested)*bindings->requested=0;
        }
    }
    services->cancel_gates(context,state);
}

static void cancel_once(struct panic_state *state,const struct panic_cancel_services *services) {
    void *context=services->context;
    state->duration=state->active_ticks=0;
    state->opponent=0;
    struct panic_bindings *bindings=services->bindings(context);
    if(bindings->requested)*bindings->requested=0;
    if(bindings->stumble)*bindings->stumble=0;
    if(bindings->between_legs)*bindings->between_legs=0;
    if(bindings->type)*bindings->type=0;
    panic_u8 session=services->session_state(context);
    if(state->actor && session==1)services->pose(context,&state->facing,state->position);
    else {
        state->facing=0;
        for(unsigned n=0;n<4;n++)state->position[n]=0;
    }
    bindings=services->bindings(context);
    if(bindings->interrupt_exit)*bindings->interrupt_exit=1;
}

/* 4a53e10: the three gates are independent, not an else-if chain. A later
 * session query can differ after the collision service has run. */
EXPORT void nhl27_panic_update_cancel(struct panic_state *state,
                                     const struct panic_cancel_services *services) {
    void *context=services->context;
    const panic_u8 *requested=services->interrupt_requested(context);
    if(requested && *requested==1)cancel_once(state,services);
    if(services->collision(context))cancel_once(state,services);
    if(services->session_present(context) && services->session_state(context))return;
    cancel_once(state,services);
}
