/* Bridge the native stack query context to the extra retained source pose.
 * Keys are the caller's two pose-map addresses, never an actor index or an
 * assumed emulator TLS offset. No scopes are published until blend integration. */
#include "ptm_query_scope.h"
#include <xmmintrin.h>
static struct ptm_query_scope *head;
static unsigned registry_lock;
static void lock(void){while(__atomic_exchange_n(&registry_lock,1,__ATOMIC_ACQUIRE))_mm_pause();}
static void unlock(void){__atomic_store_n(&registry_lock,0,__ATOMIC_RELEASE);}
PTM_EXPORT int nhl24_ptm_query_scope_open(struct ptm_query_scope *s,void *current,void *future,
        void *third,void *cache){
    if(!current||!future||!cache)return 0;
    lock();
    for(struct ptm_query_scope *p=head;p;p=p->next)if(p==s){unlock();return 0;}
    s->current_map=current;s->future_map=future;s->third_pose=third;s->third_cache=cache;
    __atomic_store_n(&s->users,0,__ATOMIC_RELAXED);s->next=head;head=s;
    unlock();return 1;
}
PTM_EXPORT void nhl24_ptm_query_scope_close(struct ptm_query_scope *s){
    lock();struct ptm_query_scope **p=&head;
    while(*p && *p!=s)p=&(*p)->next;
    int found=*p!=0;
    if(found){*p=s->next;s->next=0;}
    unlock();
    if(found)while(__atomic_load_n(&s->users,__ATOMIC_ACQUIRE))_mm_pause();
}
extern PTM_GAME_ABI int nhl24_ptm_velocity_query_guest(panic_address,void*,void*,void*,void*);
PTM_EXPORT PTM_GAME_ABI int nhl24_ptm_velocity_dispatch_guest(panic_address base,void *schema,void *query){
    void *current=*(void**)((panic_u8*)query+0x60),*future=*(void**)((panic_u8*)query+0x68);
    lock();struct ptm_query_scope *s=head;
    while(s && (s->current_map!=current || s->future_map!=future))s=s->next;
    if(s)__atomic_add_fetch(&s->users,1,__ATOMIC_RELAXED);
    unlock();
    if(!s)return 0;
    int handled=nhl24_ptm_velocity_query_guest(base,schema,query,s->third_pose,s->third_cache);
    __atomic_sub_fetch(&s->users,1,__ATOMIC_RELEASE);
    return handled;
}
