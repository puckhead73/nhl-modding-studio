/* NHL24 services recovered for the panic runtime. These are private guest-image
 * offsets for the hash checked by build_input_eboot.py, not emulator changes. */
#include "panic_state.h"
#include "panic_update.h"
#if defined(_WIN32)
#define EXPORT __declspec(dllexport)
#define GAME_ABI __attribute__((sysv_abi))
#else
#define EXPORT __attribute__((visibility("default")))
#define GAME_ABI
#endif
typedef panic_address address;
typedef unsigned char byte;
typedef void (GAME_ABI *bone_query)(unsigned,unsigned,float*,unsigned);

typedef void *(GAME_ABI *path_query)(void*);
typedef void *(GAME_ABI *record_query)(void*,void*);

/* Availability-only status view. Source 480e6f8 publishes status 17 after
 * setting SignalDekeOnKneesEnter (GSBool runtime +e20); native 2bf106d
 * publishes status 18 after the same signal (+1270). Native status 17 is a
 * different state and must not accidentally satisfy the source comparison.
 * Only comparisons against 14 and 17 are qualified for this view. */
EXPORT GAME_ABI unsigned nhl24_panic_availability_status(void *actor) {
    unsigned status=*(unsigned*)((byte*)actor+0xac);
    return status==18?17:status==17?0xffffffffu:status;
}

/* Source 455ec20 checks the availability resource and action state before
 * entering 455e7b0. Native 27c10d0 reverses that order, so calling it directly
 * would execute unrelated state queries even when the prefix rejects. */
typedef void *(GAME_ABI *evaluation_root_query)(void*);
EXPORT int nhl24_panic_availability_prefix(void *actor,void *registry,
        evaluation_root_query root_query,record_query action_query,panic_u8 *out) {
    if(!actor || !registry || !root_query || !action_query || !out)return 0;
    byte *animation=*(byte**)((byte*)actor+0x60);
    byte *wrapper=animation?*(byte**)(animation+0xc28):0;
    byte *evaluation=wrapper?*(byte**)(wrapper+0x20):0;
    if(!evaluation)return 0;
    address *methods=*(address**)registry;
    if(!methods || !methods[0x78/8])return 0;
    void **slot=((void**(GAME_ABI*)(void*,void*))methods[0x78/8])(registry,evaluation+0x60);
    if(!slot)return 0;
    const byte *state=*slot;
    if(state && state[0x31]==1 && state[0x33]!=1) { *out=0;return 1; }
    /* The preceding query can mutate evaluation ownership; reacquire it. */
    animation=*(byte**)((byte*)actor+0x60);
    wrapper=animation?*(byte**)(animation+0xc28):0;
    evaluation=wrapper?*(byte**)(wrapper+0x20):0;
    if(!evaluation)return 0;
    byte *root_wrapper=root_query(evaluation);
    byte *root=root_wrapper?*(byte**)(root_wrapper+0x10):0;
    void *property=root?*(void**)(root+0x68):0;
    if(!property)return 0;
    /* Native 27c1141 reacquires the evaluation after resolving the root. */
    animation=*(byte**)((byte*)actor+0x60);
    wrapper=animation?*(byte**)(animation+0xc28):0;
    evaluation=wrapper?*(byte**)(wrapper+0x20):0;
    if(!evaluation)return 0;
    byte *actions=action_query(property,evaluation+0x60);
    if(!actions)return 0;
    const byte *disabled=*(const byte**)(actions+0x1528);
    *out=!disabled || !*disabled;
    return 1;
}
EXPORT GAME_ABI int nhl24_panic_availability_prefix_guest(address base,void *actor,panic_u8 *out) {
    if(!base)return 0;
    return nhl24_panic_availability_prefix(actor,(void*)(base+0xba07870),
        (evaluation_root_query)(base+0x2ec80c0),(record_query)(base+0x3ffbf0),out);
}

/* Native 2ef0245..2ef0272 follows animation+c18 -> node+60 -> descriptor+18,
 * the same chain used by source 4a19aa0/4a19c80 after animation+c50.
 * Native 2f00040 provides the full source exclusion set by calling 2effb90
 * for its final three kinds. The latter alone is not the complete predicate. */
static unsigned animation_kind(const byte *node) {
    const byte *active=*(const byte*const*)(node+0x60);
    if(!active)return 0;
    const byte *descriptor=*(const byte*const*)(active+0x18);
    if(!descriptor)return 0;
    const unsigned *kind=*(const unsigned*const*)(descriptor+8);
    return kind?*kind:0;
}
EXPORT GAME_ABI panic_u8 nhl24_panic_animation_reaction(void *animation) {
    if(!animation)return 0;
    const byte *node=*(const byte**)((byte*)animation+0xc18);
    return node && animation_kind(node)==0x2a;
}
EXPORT GAME_ABI panic_u8 nhl24_panic_animation_blocked(void *animation) {
    if(!animation)return 0;
    const byte *node=*(const byte**)((byte*)animation+0xc18);
    if(!node)return 0;
    unsigned state=*(const unsigned*)(node+0x20);
    if(state!=0 && state!=1 && state!=5)return 0;
    return animation_kind(node)-0x1fu<8u;
}

/* History lookup 2815160, corresponding to source 452cf70. Native records
 * have seven roles, 2700-byte records and their own double-buffer selector.
 * Reject unavailable records rather than applying source offsets to them. */
EXPORT GAME_ABI void *nhl24_panic_history(void *manager,void *actor) {
    if(!manager || !actor)return 0;
    byte *service=*(byte**)((byte*)manager+0x3d0);
    if(!service)return 0;
    byte *history=*(byte**)(service+0x48);
    if(!history || history[8])return 0;
    byte *role=*(byte**)((byte*)actor+0x100);
    if(!role)return 0;
    unsigned index=*(unsigned*)(role+0x40);
    unsigned team=*(unsigned*)((byte*)actor+0x30);
    unsigned bank=*(unsigned*)(history+0x1bfb0);
    if(index>=7 || team>=2 || bank>=2)return 0;
    return history+0x1c030+team*0x11170+bank*0x222e0+index*0x2700;
}

EXPORT GAME_ABI void *nhl24_panic_history_guest(address base,void *actor) {
    return base?nhl24_panic_history(*(void**)(base+0xbadc558),actor):0;
}

/* Source 4533ae0; native inlined predicate at 2831159..283117e and
 * 2a39d26..2a39d4d. Actor owner moved 228 -> 208; component fields stayed. */
EXPORT GAME_ABI panic_u8 nhl24_panic_component_blocked(void *actor) {
    byte *owner=*(byte**)((byte*)actor+0x208);
    if(!owner)return 0;
    byte *component=*(byte**)(owner+0x98);
    return component && *(unsigned*)(component+0x10)==0x20;
}

/* Source 453cd50 / native 27c88b0. The native routine normalizes +58;
 * preserve the source byte because callers can distinguish it from one. */
EXPORT GAME_ABI panic_u8 nhl24_panic_actor_state_blocked(void *actor) {
    if(!actor || *(unsigned*)((byte*)actor+0x20))return 0;
    byte *owner=*(byte**)((byte*)actor+0x208);
    byte *component=owner?*(byte**)(owner+0x98):0;
    if(!component)return 0;
    unsigned kind=*(unsigned*)(component+0x10);
    if(kind==0x10000)return 1;
    return kind==0x8000?component[0x58]:0;
}

/* Source 453a9b0/453aa00 share this owner state. Native 27c63e0 proves
 * actor+1e4 -> +1cc and preserves owner+d0. */
static const unsigned *panic_owner_state(void *actor) {
    if(!actor || *(unsigned*)((byte*)actor+0x1cc))return 0;
    byte *owner=*(byte**)((byte*)actor+0x208);
    return owner?*(const unsigned**)(owner+0xd0):0;
}
EXPORT GAME_ABI panic_u8 nhl24_panic_actor_blocked(void *actor,unsigned kind) {
    const unsigned *state=panic_owner_state(actor);
    return state && *state==kind;
}
EXPORT GAME_ABI panic_u8 nhl24_panic_global_blocked(void *actor,panic_u8 option) {
    const unsigned *state=panic_owner_state(actor);
    if(!state)return 0;
    if(*state-3u<2u)return 1;
    return *state==5?option:0;
}
EXPORT GAME_ABI panic_u8 nhl24_panic_global_blocked_guest(address base,void *actor) {
    const unsigned *state=panic_owner_state(actor);
    if(!state)return 0;
    if(*state-3u<2u)return 1;
    if(*state!=5 || !base)return 0;
    /* Delay the configuration load just as the source does. */
    const byte *option=*(const byte**)(base+0xb880398);
    return option?option[0x29]:0;
}

/* Source 452dc70's typed pointer resource has class 5ac1034c. Constructors
 * 37dcd0 and 41dde0 establish its source/native registry bindings. */
EXPORT GAME_ABI void *nhl24_panic_state_context_guest(address base,void *evaluation) {
    if(!base || !evaluation)return 0;
    void *registry=(void*)(base+0xba0eee8);
    address *methods=*(address**)registry;
    if(!methods || !methods[0x78/8])return 0;
    void **slot=((void**(GAME_ABI*)(void*,void*))methods[0x78/8])(registry,(byte*)evaluation+0x60);
    return slot?*slot:0;
}
EXPORT GAME_ABI void *nhl24_panic_state_object_guest(address base,void *actor) {
    if(!base || !actor)return 0;
    byte *animation=*(byte**)((byte*)actor+0x60);
    if(!animation)return 0;
    byte *wrapper=*(byte**)(animation+0xc28);
    if(!wrapper)return 0;
    byte *evaluation=*(byte**)(wrapper+0x20);
    if(!evaluation)return 0;
    return nhl24_panic_state_context_guest(base,evaluation);
}
/* NhlPhysicsFeature class dec0accd, source ctor 3f1f90/native 6a9560.
 * Checking child publication 6a3b60 stores evaluation contexts in this vector.
 * Do not confuse checking +210 with the action-data member at +170. */
EXPORT GAME_ABI int nhl24_panic_pending_contexts(void *state,struct panic_trajectory_view *out) {
    if(!state || !out)return 0;
    address begin=*(address*)((byte*)state+0x360),end=*(address*)((byte*)state+0x368);
    if(end<begin || ((end-begin)&7) || (!begin && end))return 0;
    out->begin=begin;out->end=end;
    return 1;
}
/* Native 27c0f7c..27c0f92 is the source 3e4220 condition read. Native
 * serialization already adjusts asset+38 to the Boolean interface. */
EXPORT GAME_ABI int nhl24_panic_state_condition(void *object,panic_u8 *out) {
    if(!object || !out)return 0;
    byte *asset=*(byte**)((byte*)object+8);
    byte *evaluation=*(byte**)((byte*)object+0x10);
    if(!asset || !evaluation)return 0;
    void *condition=*(void**)(asset+0x38);
    if(!condition)return 0;
    address *methods=*(address**)condition;
    if(!methods)return 0;
    *out=((panic_u8(GAME_ABI*)(void*,void*))methods[0x18/8])(condition,evaluation+0x60);
    return 1;
}
/* Source 439bdc0. Native packed flag +a8 bit 2 replaces source +b0;
 * source role/status and control fields map to +100/+ac/+18c/+1a8. */
EXPORT panic_u8 nhl24_panic_actor_allowed(void *actor,unsigned mode,
                                        const struct panic_allowed_services *services) {
    if(!actor || !services)return 0;
    byte *a=actor;
    if(a[0xa8]&4)return 0;
    const byte *role=*(const byte**)(a+0x100);
    if(!role)return 0;
    unsigned kind=*(const unsigned*)(role+0x10);
    if(kind && kind!=mode)return 0;
    if(*(unsigned*)(a+0xac))return 0;
    if(services->special_animation(services->context,*(void**)(a+0x60)))return 0;
    if(*(unsigned*)(a+0x18c) && *(unsigned*)(a+0x1a8)!=1)return 0;
    panic_u8 blocked;
    if(!services->state_condition(services->context,actor,&blocked) || blocked)return 0;
    return nhl24_panic_component_blocked(actor)^1u;
}
static panic_u8 allowed_special(void *context,void *animation) {
    if(!animation)return 1;
    address base=*(address*)context;
    unsigned id=*(unsigned*)((byte*)animation+0xa8);
    return ((panic_u8(GAME_ABI*)(unsigned))(base+0x2f005c0))(id);
}
static int allowed_condition(void *context,void *actor,panic_u8 *out) {
    return nhl24_panic_state_condition(nhl24_panic_state_object_guest(*(address*)context,actor),out);
}
EXPORT GAME_ABI panic_u8 nhl24_panic_actor_allowed_guest(address base,void *actor,unsigned mode) {
    if(!base)return 0;
    volatile struct panic_allowed_services services;
    services.context=&base;services.special_animation=allowed_special;services.state_condition=allowed_condition;
    return nhl24_panic_actor_allowed(actor,mode,(const struct panic_allowed_services*)&services);
}

/* Native 2ec3776..2ec37ac resolves the same live style outputs written by
 * the native skater initializer. Do not copy source root/asset offsets: its
 * SkatingGS +128/+28 route differs from native +a8/+8. */
EXPORT void *nhl24_panic_style_outputs(void *actor,path_query root_query,
                                     record_query skating_query,record_query style_query) {
    if(!actor || !root_query || !skating_query || !style_query)return 0;
    byte *animation=*(byte**)((byte*)actor+0x60);
    if(!animation)return 0;
    byte *wrapper=*(byte**)(animation+0xc28);
    if(!wrapper)return 0;
    byte *context=*(byte**)(wrapper+0x20);
    if(!context)return 0;
    byte *root_wrapper=(byte*)root_query(context);
    if(!root_wrapper)return 0;
    byte *root=*(byte**)(root_wrapper+0x10);
    if(!root)return 0;
    void *asset=*(void**)(root+0xf8);
    if(!asset)return 0;
    byte *skating=(byte*)skating_query(asset,context+0x60);
    if(!skating)return 0;
    byte *style=*(byte**)(skating+0xa8);
    if(!style || !*(void**)(style+8))return 0;
    return style_query(*(void**)(style+8),context+0x60);
}

EXPORT GAME_ABI void *nhl24_panic_style_outputs_guest(address base,void *actor) {
    return base?nhl24_panic_style_outputs(actor,(path_query)(base+0x2e8ded0),
        (record_query)(base+0x40c5a0),(record_query)(base+0x411510)):0;
}

EXPORT int nhl24_panic_style_slots(void *record,float **slots) {
    if(!record || !slots)return 0;
    static const unsigned offsets[4]={0x128,0x108,0x100,0xf8};
    for(unsigned n=0;n<4;n++)slots[n]=*(float**)((byte*)record+offsets[n]);
    return 1;
}

EXPORT const struct panic_trajectory_view *nhl24_panic_trajectory(void *actor,path_query query) {
    if(!actor || !query)return 0;
    byte *animation=*(byte**)((byte*)actor+0x60);
    if(!animation)return 0;
    byte *wrapper=*(byte**)(animation+0xc28);
    if(!wrapper)return 0;
    byte *context=*(byte**)(wrapper+0x20);
    if(!context)return 0;
    byte *path=(byte*)query(context+0x60);
    return path?(const struct panic_trajectory_view*)(path+0x10):0;
}

/* Source 21ce80/f20cb38 and native 325af0/ac3d828 are the primary path
 * resources consumed by their PTM post-scene updates (527390/521900).
 * Native owns its vector allocation; only begin/end are shared layouts. */
EXPORT GAME_ABI const struct panic_trajectory_view *nhl24_panic_trajectory_guest(address base,void *actor) {
    return base?nhl24_panic_trajectory(actor,(path_query)(base+0x325af0)):0;
}

/* 453cc30 returns actor+718; constructor 4538a19 passes this actor to
 * 4a3da00, which stores it at tracker+10. The alignment guard therefore
 * compares the owning actor's pose, not the puck's pose. */
EXPORT const float *nhl24_panic_path_owner_position(void *actor) {
    if(!actor)return 0;
    byte *pose=*(byte**)((byte*)actor+0x10);
    return pose?(const float*)(pose+0x30):0;
}

/* Skater table, not overall player ratings. Source 7c3ce89/7c3cf20 and native
 * 8a34af8/8a34b67 bind GeneralSpineAngle to +3588/+2d18. The following style
 * columns keep their order. Native 2ec3120 confirms the row conversion and
 * all four accesses used by source panic constructor 4a54380. */
EXPORT int nhl24_panic_style_rating(const void *table,int actor_id,unsigned source_id,int *out) {
    if(!table || !out)return 0;
    unsigned offset;
    switch(source_id) {
        case 0x11d:offset=0x2d18;break; /* GeneralSpineAngle */
        case 0x11f:offset=0x2d78;break; /* GeneralStrideDistance */
        case 0x121:offset=0x2dd8;break; /* GeneralStrideHeight */
        case 0x120:offset=0x2da8;break; /* GeneralStrideGait */
        default:return 0;
    }
    int row=-1;
    if(actor_id!=-1) {
        int player=actor_id%20;
        if(player!=-1)row=(actor_id/20)*35+(player<18?player:player-2);
    }
    const byte *column=(const byte*)table+offset;
    const byte *data=*(const byte*const*)column;
    if(!data)return 0;
    __INTPTR_TYPE__ index=(__INTPTR_TYPE__)row-*(const int*)(column+0x18);
    *out=*(const int*)(data+index* *(const __INTPTR_TYPE__*)(column+0x20));
    return 1;
}

EXPORT GAME_ABI int nhl24_panic_style_rating_guest(address base,void *actor,
                                                  unsigned source_id,int *out) {
    if(!base || !actor)return 0;
    return nhl24_panic_style_rating(*(void**)(base+0xba1a060),
        *(int*)((byte*)actor+0x3c),source_id,out);
}

/* 4c2a8f0 source virtual-foot definitions map to 2fec800 target definitions:
 * identical LeftFoot/RightFoot handedness and all four offset-vector bits.
 * The target getter handles its native quaternion/scale matrix layout. */
EXPORT int nhl24_panic_foot_probe(void *actor,unsigned source_bone,
                                bone_query query,float *out) {
    if (!actor || !query || !out || source_bone<0x25 || source_bone>0x28) return 0;
    byte *pose=*(byte **)((byte *)actor+0x10);
    if (!pose) return 0;
    query(*(unsigned *)(pose+0x94),source_bone-5,out,
          *(unsigned *)((byte *)actor+0x170));
    return 1;
}

EXPORT GAME_ABI int nhl24_panic_foot_probe_guest(address base,void *actor,
                                                unsigned source_bone,float *out) {
    if (!base) return 0;
    return nhl24_panic_foot_probe(actor,source_bone,(bone_query)(base+0x2abdfe0),out);
}

/* Source 465c3f0 ignores its apparent session argument. Target 10574b4,
 * 28325e6 and 29c13b5 implement the same byte gate and unsigned range test. */
EXPORT panic_u8 nhl24_panic_session_state(panic_u8 disabled,int state) {
    return disabled==0 && (panic_u32)state-3u<0xfffffffeu;
}

EXPORT GAME_ABI panic_u8 nhl24_panic_session_guest(address base) {
    if (!base) return 0;
    return nhl24_panic_session_state(*(byte *)(base+0xba45a39),*(int *)(base+0xba475ac));
}

/* The secondary movement enumeration's native property key is at +8 and its
 * byte count at +32. 8a7780 is the target equivalent of source 74e4c0; both
 * follow parent contexts and honor the block's initialized-bit mask. */
typedef void *(GAME_ABI *context_lookup)(void*,unsigned);
EXPORT panic_u8 nhl24_panic_motion_bit(panic_u8 cached_active,const panic_u8 *node_active,
        const void *asset,void *context,unsigned bit,context_lookup lookup) {
    if(cached_active!=1 || !asset || !context || !lookup || (bit!=3 && bit!=9))return 0;
    if(bit==9 && (!node_active || *node_active!=1))return 0;
    const byte *property=(const byte*)asset;
    if(property[32]<=bit/8)return 0;
    unsigned key=*(const unsigned*)(property+8);
    if(key<0x1000000)return 0;
    const byte *storage=(const byte*)lookup(context,key);
    return storage ? (storage[bit/8]>>(bit&7))&1u : 0;
}
EXPORT GAME_ABI panic_u8 nhl24_panic_motion_bit_guest(address base,panic_u8 cached_active,
        const panic_u8 *node_active,const void *asset,void *context,unsigned bit) {
    if(!base)return 0;
    return nhl24_panic_motion_bit(cached_active,node_active,asset,context,bit,
                                (context_lookup)(base+0x8a7780));
}

/* Native NHLPTMPostSceneOp caches the same PTMSkatingActive asset as the source.
 * Source flags+e6 moved to target flags+ce. The companion cache-byte patch
 * preserves noncanonical values so the port's ==1 predicate stays exact. */
EXPORT GAME_ABI panic_u8 nhl24_panic_cached_active(void *actor) {
    if(!actor)return 0;
    byte *animation=*(byte**)((byte*)actor+0x60);
    if(!animation)return 0;
    byte *flags=*(byte**)(animation+0x48);
    return flags ? flags[0xce] : 0;
}

/* Source 452d430 and target inline 2b1fdfc..2b1fe49 share the scale chain and
 * pose speed fields. Keep the source absolute-value bit operation and JAE
 * semantics (unordered scale still reaches division). */
EXPORT float nhl24_panic_normalized_speed(void *actor) {
    if(!actor)return 0;
    float scale=0;
    byte *animation=*(byte**)((byte*)actor+0x60);
    if(animation) {
        byte *parameters=*(byte**)(animation+8);
        if(parameters)scale=*(float*)(parameters+0x30);
    }
    union {float f;panic_u32 u;} magnitude={scale};
    magnitude.u&=0x7fffffffu;
    if(0x1p-16f>=magnitude.f)return 0;
    byte *pose=*(byte**)((byte*)actor+0x10);
    return *(float*)(pose+0x90)/scale;
}

/* Source locator registry f28b0f8 maps to target ac8fc18 (class 738a4434).
 * This property uses +78 for the live read, unlike BoolAsset. Its native
 * constructor and allocator own the registry's internal pools and listeners.
 * Source 4a544f0 and native 2bdeb04 use the same publication/refcount sequence.
 */
typedef void **(GAME_ABI *reference_slot)(void *,void *);
typedef void *(GAME_ABI *registry_allocate)(void *,address,const char *,unsigned,unsigned,unsigned);
typedef void (GAME_ABI *registry_construct)(void *);
typedef void (GAME_ABI *registry_destroy)(void *);
typedef void (GAME_ABI *registry_remove)(void *,void *);

static void locator_release(void *registry) {
    if(registry && __atomic_sub_fetch((int*)((byte*)registry+8),1,__ATOMIC_SEQ_CST)==0)
        ((registry_destroy)(*(address**)registry)[4])(registry);
}

EXPORT void *nhl24_panic_locator_ensure(void *property,void *context,void *allocator,
                                      registry_construct construct,const char *label) {
    if(!property || !context || !allocator || !construct)return 0;
    address *methods=*(address**)property;
    void *registry=*((reference_slot)methods[0x78/8])(property,context);
    if(registry)return registry;
    registry=((registry_allocate)(*(address**)allocator)[3])(allocator,0x770,label,1,8,0);
    if(!registry)return 0;
    construct(registry);
    __atomic_add_fetch((int*)((byte*)registry+8),1,__ATOMIC_SEQ_CST);
    /* Reacquire the slot after allocation: initialization may publish a value. */
    methods=*(address**)property;
    void **slot=((reference_slot)methods[0x88/8])(property,context);
    void *previous=*slot;
    if(previous!=registry) {
        __atomic_add_fetch((int*)((byte*)registry+8),1,__ATOMIC_SEQ_CST);
        *slot=registry;
        locator_release(previous);
    }
    locator_release(registry);
    return registry;
}

EXPORT GAME_ABI void *nhl24_panic_locator_registry_guest(address base,void *context) {
    if(!base)return 0;
    return nhl24_panic_locator_ensure((void*)(base+0xac8fc18),context,
        *(void**)(base+0xaa3dbb0),(registry_construct)(base+0x8c7130),
        (const char*)(base+0x9143b92));
}

EXPORT GAME_ABI void nhl24_panic_locator_remove_guest(address base,void *registry,void *locator) {
    if(base && registry && locator)
        ((registry_remove)(base+0x8c55d0))(registry,locator);
}
