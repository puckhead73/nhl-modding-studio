/* Separate state for the eighteen native owner slots. Native owner+e8 contains
 * flags, so the source component pointer cannot be published at that offset. */
#include "panic_lifecycle.h"
#if defined(_WIN32)
#define EXPORT __declspec(dllexport)
#else
#define EXPORT __attribute__((visibility("default")))
#endif
extern float nhl27_panic_attribute(unsigned,int);
struct owner_slot {panic_address owner;struct panic_state state;};
static struct owner_slot owners[18];

EXPORT void nhl27_panic_construct(struct panic_state *state,panic_address actor,
                                  const struct panic_lifecycle_services *services) {
    /* Preserve source allocator padding: ctor stores cover 0..40 and 44..83. */
    panic_u8 *bytes=(panic_u8*)state;
    for(unsigned n=0;n<41;n++)bytes[n]=0;
    for(unsigned n=44;n<84;n++)bytes[n]=0;
    state->actor=actor;
    services->reset(services->context,state);
    static const unsigned ids[4]={0x11d,0x11f,0x121,0x120};
    for(unsigned n=0;n<4;n++)state->attributes[n]=nhl27_panic_attribute(ids[n],
        services->rating(services->context,state->actor,ids[n]));
}

EXPORT void nhl27_panic_destruct(struct panic_state *state,
                                 const struct panic_lifecycle_services *services) {
    services->reset(services->context,state);
    state->actor=0;
}

EXPORT struct panic_state *nhl24_panic_owner_create(unsigned index,panic_address owner,
        panic_address actor,const struct panic_lifecycle_services *services) {
    if(index>=18 || !owner || !actor || !services || !services->reset || !services->rating)return 0;
    struct owner_slot *slot=&owners[index];
    if(slot->owner)return 0; /* A missed destroy must not silently reuse state. */
    nhl27_panic_construct(&slot->state,actor,services);
    slot->owner=owner;
    return &slot->state;
}

EXPORT struct panic_state *nhl24_panic_owner_get(unsigned index,panic_address owner,panic_address actor) {
    if(index>=18 || !owner || !actor)return 0;
    struct owner_slot *slot=&owners[index];
    return slot->owner==owner && slot->state.actor==actor ? &slot->state : 0;
}

EXPORT int nhl24_panic_owner_destroy(unsigned index,panic_address owner,
                                    const struct panic_lifecycle_services *services) {
    if(index>=18 || !owner || !services || !services->reset)return 0;
    struct owner_slot *slot=&owners[index];
    if(slot->owner!=owner)return 0;
    nhl27_panic_destruct(&slot->state,services);
    slot->owner=0;
    return 1;
}
