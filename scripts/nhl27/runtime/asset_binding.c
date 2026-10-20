/* Resolve added native assets in the same loaded bank as an existing anchor.
 * Native bank lookup (vtable +0x30) preserves signed IDs and bank aliases.
 * No cached asset pointers survive bank unload/reload. The caller must hold
 * the native bank registry lock and keep the selected bank alive while using
 * the result. These helpers are not installed runtime hooks by themselves.
 */
typedef __UINTPTR_TYPE__ address;
typedef unsigned long long u64;
typedef unsigned int u32;
typedef unsigned char u8;
#include "panic_state.h"
#define GAME_ABI __attribute__((sysv_abi))
#if defined(_WIN32)
#define EXPORT __declspec(dllexport)
#else
#define EXPORT __attribute__((visibility("default")))
#endif

enum binding_status { BIND_OK, BIND_INVALID, BIND_MISSING, BIND_AMBIGUOUS, BIND_INTERFACE };
typedef void *(GAME_ABI *asset_query)(void *, const u64 *);
typedef void *(GAME_ABI *asset_cast)(void *, u32);
typedef void *(GAME_ABI *value_storage)(void *, void *);

static void *query(void *bank, u64 id) {
    if (!bank) return (void *)0;
    address *table = *(address **)bank;
    if (!table || !table[6]) return (void *)0;
    return ((asset_query)table[6])(bank, &id);
}

EXPORT int nhl24_ant_find_anchored_asset(void *const *banks, u32 count,
        u64 anchor_id, void *anchor, u64 id, void **result) {
    if (!result || !anchor || (count && !banks)) return BIND_INVALID;
    void *selected = (void *)0;
    for (u32 i = 0; i < count; ++i) {
        if (query(banks[i], anchor_id) != anchor) continue;
        if (selected && selected != banks[i]) return BIND_AMBIGUOUS;
        selected = banks[i];
    }
    if (!selected) return BIND_MISSING;
    void *asset = query(selected, id);
    if (!asset) return BIND_MISSING;
    *result = asset;
    return BIND_OK;
}

EXPORT int nhl24_ant_bind_value(void *asset, void *context, void **result) {
    if (!asset || !context || !result) return BIND_INVALID;
    address *table = *(address **)asset;
    if (!table || !table[2]) return BIND_INTERFACE;
    /* Same writable-property interface used by GSBool/Skating serializers.
     * Cast before calling +0x88; arbitrary ANT assets need not expose it.
     */
    void *property = ((asset_cast)table[2])(asset, 0x7a327d7du);
    if (!property) return BIND_INTERFACE;
    table = *(address **)property;
    if (!table || !table[17]) return BIND_INTERFACE;
    void *storage = ((value_storage)table[17])(property, context);
    if (!storage) return BIND_MISSING;
    *result = storage;
    return BIND_OK;
}

EXPORT int nhl24_ant_find_registered_asset(void *const *banks, u32 count,
        u64 anchor_id, void *anchor, u64 id, void **result) {
    if (!result) return BIND_INVALID;
    void *verified=(void *)0;
    int status=nhl24_ant_find_anchored_asset(banks,count,anchor_id,anchor,anchor_id,&verified);
    if (status) return status;
    /* GSBool lives in state; PTMActive and PathLock live in launch. A valid
     * anchor does not imply that the requested asset belongs to its bank.
     * Accept only a unique object across registered banks, never an arbitrary
     * first match if overlapping bank generations expose different objects.
     */
    void *selected=(void *)0;
    for (u32 i=0;i<count;++i) {
        void *candidate=query(banks[i],id);
        if (!candidate) continue;
        if (selected && selected!=candidate) return BIND_AMBIGUOUS;
        selected=candidate;
    }
    if (!selected) return BIND_MISSING;
    *result=selected;
    return BIND_OK;
}

/* Registry 0xac8fd68 owns its bank vector at +0/+8. Registration 0xad9540
 * holds the +0x20 mutex. Match that lock convention, including its recursion
 * count at +8 and the native unlock thunk. Resolve storage before unlocking;
 * neither the bank nor asset pointer is retained by the port.
 */
static GAME_ABI int bind_many_guest(address base, void *context, const u64 *ids, u32 count, void **result) {
    typedef void *(GAME_ABI *root_getter)(void *);
    typedef void (GAME_ABI *lock_fn)(void *);
    typedef int (GAME_ABI *unlock_fn)(void *);
    if (!base || (address)context < 0x60 || !result || !ids || !count || count>9) return BIND_INVALID;
    void *wrapper = ((root_getter)(base+0x2ec80c0))((u8 *)context-0x60);
    if (!wrapper) return BIND_MISSING;
    void *root = *(void **)((u8 *)wrapper+0x10);
    if (!root) return BIND_MISSING;
    void *anchor = *(void **)((u8 *)root+0x68);
    void *manager = *(void **)(base+0xac8fd68);
    if (!anchor || !manager) return BIND_MISSING;
    void *lock = (u8 *)manager+0x20;
    ((lock_fn)(base+0xde4c10))(lock);
    address begin = *(address *)manager;
    address end = *(address *)((u8 *)manager+8);
    int status = BIND_INVALID;
    void *slots[9];
    if (end >= begin && !((end-begin)&7) && (end-begin)/8 <= 4096) {
        for (u32 i=0;i<count;++i) {
            void *asset = (void *)0;
            status = nhl24_ant_find_registered_asset((void *const *)begin,(u32)((end-begin)/8),
                0x30b7d024f81ea1dfULL,anchor,ids[i],&asset);
            if (!status) status = nhl24_ant_bind_value(asset,context,slots+i);
            if (status) break;
        }
    }
    --*(u32 *)((u8 *)lock+8);
    ((unlock_fn)(base+0x8ec57c0))(lock);
    if (!status) for (u32 i=0;i<count;++i) result[i]=slots[i];
    return status;
}

EXPORT GAME_ABI int nhl24_ant_bind_guest(address base, void *context, u64 id, void **result) {
    return bind_many_guest(base,context,&id,1,result);
}

EXPORT GAME_ABI int nhl24_panic_bind_guest(address base, void *context, struct panic_bindings *result) {
    if (!result) return BIND_INVALID;
    /* Source AnimAssetRI_SkatingGameStateAssets imports; preserve identities
     * shared with the staged controllers rather than cloning their storage.
     */
    static const u64 ids[9]={0xe0833ad3a430af00ULL,0xecf161355bd30ed9ULL,
        0xa684658da6bdcecaULL,0xde0e918abd457be9ULL,0xa04e5c3762bb2993ULL,
        0x7e633726de0e1429ULL,0x2d0bcff5c6551725ULL,0xa98b7f6ed7bcd9bdULL,
        0x9238092cfdec447cULL};
    void *slots[9];
    int status=bind_many_guest(base,context,ids,9,slots);
    if (status) return status;
    result->ptm_active=slots[0]; result->path_lock=slots[1];
    result->stumble=slots[2]; result->requested=slots[3];
    result->closest_distance=slots[4]; result->between_legs=slots[5];
    result->type=slots[6]; result->interrupt_exit=slots[7]; result->align_requested=slots[8];
    return BIND_OK;
}

typedef void *(GAME_ABI *context_lookup)(void *,u32);
/* Read the actual staged NodeActive and native secondary movement bit array.
 * This uses read-only property access (+0x80), not the allocating writer at
 * +0x88. In particular a missing context block must remain uninitialized. */
EXPORT int nhl24_panic_motion_registered(void *const *banks,u32 count,void *anchor,
        void *context,u8 cached_active,u32 bit,context_lookup lookup,u8 *result) {
    if(!result || !context || !lookup || (bit!=3 && bit!=9))return BIND_INVALID;
    if(cached_active!=1){*result=0;return BIND_OK;}
    void *asset=(void*)0;
    int status;
    if(bit==9) {
        status=nhl24_ant_find_registered_asset(banks,count,0x30b7d024f81ea1dfULL,
            anchor,0x9f2ed2b822171119ULL,&asset);
        if(status)return status;
        address *table=*(address**)asset;
        if(!table || !table[2])return BIND_INTERFACE;
        void *property=((asset_cast)table[2])(asset,0x7a327d7du);
        if(!property)return BIND_INTERFACE;
        table=*(address**)property;
        if(!table || !table[16])return BIND_INTERFACE;
        const u8 *node=((value_storage)table[16])(property,context);
        if(!node || *node!=1){*result=0;return BIND_OK;}
    }
    status=nhl24_ant_find_registered_asset(banks,count,0x30b7d024f81ea1dfULL,
        anchor,0xc9f1cc065e44d353ULL,&asset);
    if(status)return status;
    address *table=*(address**)asset;
    if(!table || !table[2])return BIND_INTERFACE;
    const u8 *property=((asset_cast)table[2])(asset,0x1d52f677u);
    if(!property)return BIND_INTERFACE;
    if(property[32]<=bit/8){*result=0;return BIND_OK;}
    u32 key=*(const u32*)(property+8);
    if(key<0x1000000u){*result=0;return BIND_OK;}
    const u8 *storage=lookup(context,key);
    *result=storage ? (storage[bit/8]>>(bit&7))&1u : 0;
    return BIND_OK;
}

EXPORT GAME_ABI int nhl24_panic_motion_context_guest(address base,void *context,
        u8 cached_active,u32 bit,u8 *result) {
    typedef void *(GAME_ABI *root_getter)(void *);
    typedef void (GAME_ABI *lock_fn)(void *);
    typedef int (GAME_ABI *unlock_fn)(void *);
    if(!base || (address)context<0x60 || !result || (bit!=3 && bit!=9))return BIND_INVALID;
    if(cached_active!=1){*result=0;return BIND_OK;}
    void *wrapper=((root_getter)(base+0x2ec80c0))((u8*)context-0x60);
    if(!wrapper)return BIND_MISSING;
    void *root=*(void**)((u8*)wrapper+0x10);
    if(!root)return BIND_MISSING;
    void *anchor=*(void**)((u8*)root+0x68);
    void *manager=*(void**)(base+0xac8fd68);
    if(!anchor || !manager)return BIND_MISSING;
    void *lock=(u8*)manager+0x20;
    ((lock_fn)(base+0xde4c10))(lock);
    address begin=*(address*)manager,end=*(address*)((u8*)manager+8);
    int status=BIND_INVALID;
    if(end>=begin && !((end-begin)&7) && (end-begin)/8<=4096)
        status=nhl24_panic_motion_registered((void *const*)begin,(u32)((end-begin)/8),
            anchor,context,cached_active,bit,(context_lookup)(base+0x8a7780),result);
    --*(u32*)((u8*)lock+8);
    ((unlock_fn)(base+0x8ec57c0))(lock);
    return status;
}
