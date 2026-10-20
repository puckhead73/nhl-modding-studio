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

#define GAME_ABI __attribute__((sysv_abi))
#if defined(_WIN32)
#define static __declspec(dllexport)
#else
#define static __attribute__((visibility("default")))
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

static int deke_ant_find_anchored_asset(void *const *banks, u32 count,
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

static int deke_ant_bind_value(void *asset, void *context, void **result) {
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

static int deke_ant_find_registered_asset(void *const *banks, u32 count,
        u64 anchor_id, void *anchor, u64 id, void **result) {
    if (!result) return BIND_INVALID;
    void *verified=(void *)0;
    int status=deke_ant_find_anchored_asset(banks,count,anchor_id,anchor,anchor_id,&verified);
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
            status = deke_ant_find_registered_asset((void *const *)begin,(u32)((end-begin)/8),
                0x30b7d024f81ea1dfULL,anchor,ids[i],&asset);
            if (!status) status = deke_ant_bind_value(asset,context,slots+i);
            if (status) break;
        }
    }
    --*(u32 *)((u8 *)lock+8);
    ((unlock_fn)(base+0x8ec57c0))(lock);
    if (!status) for (u32 i=0;i<count;++i) result[i]=slots[i];
    return status;
}

int GAME_ABI deke_ant_bind_guest(address base, void *context, u64 id, void **result) {
    return bind_many_guest(base,context,&id,1,result);
}
