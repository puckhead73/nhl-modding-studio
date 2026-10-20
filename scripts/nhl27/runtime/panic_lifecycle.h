#ifndef NHL27_PANIC_LIFECYCLE_H
#define NHL27_PANIC_LIFECYCLE_H
#include "panic_state.h"
struct panic_lifecycle_services {
    void *context;
    void (*reset)(void *,struct panic_state *);
    int (*rating)(void *,panic_address,unsigned);
};
#endif
