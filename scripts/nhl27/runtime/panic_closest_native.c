/* Engine adapters for source 4a52200; invoked by the panic frame integrator. */
#include "panic_update.h"
#define ABI __attribute__((sysv_abi))
#if defined(_WIN32)
#define EXPORT __declspec(dllexport)
#else
#define EXPORT __attribute__((visibility("default")))
#endif
typedef panic_address A;
typedef unsigned char B;
extern ABI int nhl24_panic_bind_guest(A,void*,struct panic_bindings*);
extern ABI int nhl24_panic_motion_context_guest(A,void*,B,unsigned,B*);
extern ABI B nhl24_panic_cached_active(void*);
extern ABI void *nhl24_panic_first_candidate_guest(A,void*);
extern ABI void *nhl24_panic_second_candidate_guest(A,void*);
extern ABI float nhl24_panic_boundary_distance_guest(A,void*);
extern ABI float nhl24_panic_end_distance_guest(A,void*);
extern float nhl27_panic_actor_distance(void*,void*);
struct closest_context {A base;void *evaluation;struct panic_bindings bindings;int status;};
static struct panic_bindings *bindings(void *p){return &((struct closest_context*)p)->bindings;}
static B active(void *p,A actor){
    struct closest_context *c=p;B result=0;
    c->status=nhl24_panic_motion_context_guest(c->base,c->evaluation,
        nhl24_panic_cached_active((void*)actor),3,&result);
    return c->status?0:result;
}
static void *first(void *p,A actor){return nhl24_panic_first_candidate_guest(((struct closest_context*)p)->base,(void*)actor);}
static void *second(void *p,A actor){return nhl24_panic_second_candidate_guest(((struct closest_context*)p)->base,(void*)actor);}
static float boundary(void *p,A actor){return nhl24_panic_boundary_distance_guest(((struct closest_context*)p)->base,(void*)actor);}
static float end(void *p,A actor){return nhl24_panic_end_distance_guest(((struct closest_context*)p)->base,(void*)actor);}
static float distance(void *p,void *candidate,A actor){
    A base=((struct closest_context*)p)->base,*methods=*(A**)candidate;
    /* Qualify the concrete native actor implementation before substituting
     * source scalar summation. Other actor subclasses retain virtual dispatch. */
    if(methods[6]==base+0x26323d0 && methods[4]==base+0x2749130)
        return nhl27_panic_actor_distance(candidate,(void*)actor);
    return ((float(ABI*)(void*,void*))methods[6])(candidate,(void*)actor);
}
EXPORT ABI int nhl24_panic_update_closest_guest(A base,struct panic_state *state){
    if(!base || !state || !state->actor)return 1;
    B *animation=*(B**)(state->actor+0x60);
    if(!animation)return 1;
    B *wrapper=*(B**)(animation+0xc28);
    if(!wrapper || !*(void**)(wrapper+0x20))return 1;
    struct closest_context context={0};
    context.base=base;context.evaluation=*(B**)(wrapper+0x20)+0x60;
    context.status=nhl24_panic_bind_guest(base,context.evaluation,&context.bindings);
    if(context.status)return context.status;
    const struct panic_closest_services services={&context,bindings,active,first,second,boundary,distance,end};
    nhl27_panic_update_closest(state,&services);
    return context.status;
}
