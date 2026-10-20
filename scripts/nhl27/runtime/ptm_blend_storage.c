/* Retained pose pair used by source 2837a0/283e60. These routines do not
 * publish an active controller or substitute native pose history for pose A. */
#include "ptm_blend_storage.h"
PTM_EXPORT void nhl24_ptm_blend_storage_init(struct ptm_blend_extension *e,void *pose,
    float nominal,float blend_in,float blend_out,const struct ptm_blend_pose_services *s){
    for(unsigned i=0;i<sizeof(*e);i++)((panic_u8*)e)[i]=0;
    e->pose_b=s->clone(s->context,pose);
    if(e->pose_b)s->retain(s->context,e->pose_b);
    e->clock.nominal=nominal;
    e->clock.duration=0x1.342618p+116f; /* Source dedad4c: bits 799a130c. */
    e->clock.blend_in=blend_in;
    e->clock.blend_out=blend_out;
    e->clock.speed=1.0f;
    e->retime=1;
    /* Only the complete update adapter may enable this extension. */
    e->magic=PTM_BLEND_MAGIC;
}
PTM_EXPORT void nhl24_ptm_blend_storage_capture_a(struct ptm_blend_extension *e,void *pose,
    const struct ptm_blend_pose_services *s){
    if(e->magic!=PTM_BLEND_MAGIC)return;
    void *next=s->clone(s->context,pose),*previous=e->pose_a;
    if(next==previous)return;
    if(next)s->retain(s->context,next);
    e->pose_a=next;
    if(previous)s->release(s->context,previous);
}
PTM_EXPORT void nhl24_ptm_blend_storage_destroy(struct ptm_blend_extension *e,
    const struct ptm_blend_pose_services *s){
    if(e->magic!=PTM_BLEND_MAGIC)return;
    void *a=e->pose_a,*b=e->pose_b;
    /* Unpublish first: release may invoke arbitrary pose destructors. */
    e->magic=0;e->pose_a=0;e->pose_b=0;e->enabled=0;e->initialized=0;
    if(a)s->release(s->context,a);
    if(b)s->release(s->context,b);
}

extern void nhl24_ptm_pose_retain(void*);
extern void nhl24_ptm_pose_release(void*);
static void *guest_clone(void *context,void *pose){
    typedef void *(PTM_GAME_ABI *clone_fn)(void*,int);
    return ((clone_fn)((panic_address)context+0x8bb990))(pose,0);
}
static void guest_retain(void *context,void *pose){(void)context;nhl24_ptm_pose_retain(pose);}
static void guest_release(void *context,void *pose){(void)context;nhl24_ptm_pose_release(pose);}
static struct ptm_blend_pose_services guest_services(panic_address base){
    return (struct ptm_blend_pose_services){(void*)base,guest_clone,guest_retain,guest_release};
}
PTM_EXPORT PTM_GAME_ABI void nhl24_ptm_blend_initialize_guest(panic_address base,void *blend,void *snapshot){
    struct ptm_blend_pose_services s=guest_services(base);
    nhl24_ptm_blend_storage_init((void*)((panic_u8*)blend+PTM_BLEND_EXTENSION_OFFSET),
        *(void**)((panic_u8*)blend+0x30),*(float*)((panic_u8*)snapshot+0x10),0,0,&s);
}
PTM_EXPORT PTM_GAME_ABI void nhl24_ptm_blend_finish_clock_guest(panic_address base,void *blend){
    (void)base;
    struct ptm_blend_extension *e=(void*)((panic_u8*)blend+PTM_BLEND_EXTENSION_OFFSET);
    e->clock.blend_in=*(float*)((panic_u8*)blend+0x2c8);
    e->clock.blend_out=*(float*)((panic_u8*)blend+0x2cc);
}
PTM_EXPORT PTM_GAME_ABI void nhl24_ptm_blend_destroy_guest(panic_address base,void *blend){
    struct ptm_blend_pose_services s=guest_services(base);
    nhl24_ptm_blend_storage_destroy((void*)((panic_u8*)blend+PTM_BLEND_EXTENSION_OFFSET),&s);
}
