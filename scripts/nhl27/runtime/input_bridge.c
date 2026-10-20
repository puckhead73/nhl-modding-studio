/* NHL24 1.60 native Boolean bridge and source action-state primitives. No CRT.
 * This is a runtime component, not an installer or a producer-parity claim.
 * All eight bindings are recovered from AnimAssetRI_GSBoolAssets and 0x3ffbf0.
 * A successful write changes the existing native Boolean's backing storage.
 * Source action storage primitives are separate from the native Boolean bridge.
 * Producer scheduling and storage ownership must be supplied by integration.
 */
typedef __UINTPTR_TYPE__ address;
typedef unsigned int u32;
typedef unsigned char u8;
#if defined(_WIN32)
#define EXPORT __declspec(dllexport)
#else
#define EXPORT __attribute__((visibility("default")))
#endif
#define GAME_ABI __attribute__((sysv_abi))

enum result { OK = 0, UNSUPPORTED_ACTION = 1, MISSING_BINDING = 2, INVALID_ARGUMENT = 3 };
static void sync_selected_bytes(void *record);

/* Source 0x3c9ae0/0x3cd090: 244 slots, each a byte plus change tick at +4.
 * The remaining record contains analog inputs; never treat it as Boolean slots.
 * Boolean writes keep padding and analog storage untouched.
 */
struct action_slot { u8 value; u8 previous; u8 reserved[2]; u32 changed_tick; };
/* Float input words are copied as integers to preserve every IEEE bit pattern. */
struct analog_slot { u32 value_bits; u32 previous_bits; u32 opaque[5]; };
struct action_table {
    struct action_slot slots[244];
    struct analog_slot analog[40];
    u32 tick;
    u8 tail[12];
};
_Static_assert(sizeof(struct action_slot) == 8, "source action slot ABI");
_Static_assert(sizeof(struct analog_slot) == 28, "source analog slot ABI");
_Static_assert(__builtin_offsetof(struct action_table, tick) == 0xc00, "source tick ABI");
_Static_assert(sizeof(struct action_table) == 0xc10, "source action table ABI");

/* Controller tables belong to the input service, not to the currently selected
 * player. Switching players must retain the controller's history. The appended
 * selector is encoded as index+1, with zero selecting the actor's local table.
 * The private ELF builder places this zero-filled pool in writable BSS.
 */
static struct {
    u32 initialized;
    struct action_table tables[12];
} controller_pool;

/* Exact float words from source reset 0x3d7570. No decimal round trips. */
static const u32 analog_limits[40][2] = {
    {0xc0490fdbu, 0x40490fdbu}, {0, 0x3f800000u},
    {0xc0490fdbu, 0x40490fdbu}, {0xc0490fdbu, 0x40490fdbu},
    {0, 0x3f800000u}, {0, 0x3f800000u}, {0, 0x3f800000u},
    {0xc0490fdbu, 0x40490fdbu}, {0xc0490fdbu, 0x40490fdbu},
    {0xc0490fdbu, 0x40490fdbu}, {0, 0x3f800000u}, {0, 0x3f800000u},
    {0, 0x40c90fdbu}, {0, 0x40c90fdbu}, {0, 0x3f800000u},
    {0xc0490fdbu, 0x40490fdbu}, {0xc0490fdbu, 0x40490fdbu},
    {0xbf800000u, 0x3f800000u}, {0xbf800000u, 0x3f800000u},
    {0xbf800000u, 0x3f800000u}, {0xbf800000u, 0x3f800000u},
    {0xbf800000u, 0x3f800000u}, {0xc0490fdbu, 0x40c90fdbu},
    {0, 0x40000000u}, {0xc0490fdbu, 0x40c90fdbu}, {0xc0490fdbu, 0x40c90fdbu},
    {0, 0x3f800000u}, {0, 0x3f800000u},
    {0xc0490fdbu, 0x40c90fdbu}, {0xc0490fdbu, 0x40c90fdbu},
    {0xc0490fdbu, 0x40c90fdbu}, {0xc0490fdbu, 0x40c90fdbu},
    {0, 0x3f800000u}, {0, 0x3f800000u},
    {0xc0490fdbu, 0x40c90fdbu}, {0xc0490fdbu, 0x40c90fdbu},
    {0xc0490fdbu, 0x40c90fdbu}, {0xc0490fdbu, 0x40c90fdbu},
    {0xc0490fdbu, 0x40c90fdbu}, {0xc0490fdbu, 0x40c90fdbu}
};

/* Source 0x3d72b0 resets the tick and tail-calls 0x3d7570. It preserves
 * the owner at +0xc08 and the four untouched bytes at +0xc04.
 */
EXPORT int nhl27_action_reset(void *storage) {
    if (!storage) return MISSING_BINDING;
    struct action_table *table = storage;
    for (u32 i = 0; i < 244; ++i) {
        table->slots[i].value = table->slots[i].previous = 0;
        table->slots[i].reserved[0] = table->slots[i].reserved[1] = 0;
        table->slots[i].changed_tick = 0xffffffffu;
    }
    for (u32 i = 0; i < 40; ++i) {
        struct analog_slot *slot = &table->analog[i];
        slot->value_bits = slot->previous_bits = slot->opaque[0] = 0;
        slot->opaque[1] = 0xffffffffu; /* change timestamp */
        slot->opaque[2] = 0x34000000u; /* difference threshold */
        slot->opaque[3] = analog_limits[i][0];
        slot->opaque[4] = analog_limits[i][1];
    }
    table->tick = 0;
    return OK;
}

/* Source constructor 0x3d7080 additionally clears the owner. */
EXPORT int nhl27_action_initialize(void *storage) {
    int status = nhl27_action_reset(storage);
    if (status != OK) return status;
    struct action_table *table = storage;
    for (u32 i = 4; i < 12; ++i) table->tail[i] = 0;
    return OK;
}

EXPORT void *nhl27_controller_table(u32 index) {
    if (index >= 12) return (void *)0;
    u32 expected = 0;
    if (__atomic_compare_exchange_n(&controller_pool.initialized, &expected, 1, 0,
                                    __ATOMIC_ACQ_REL, __ATOMIC_ACQUIRE)) {
        for (u32 i = 0; i < 12; ++i)
            nhl27_action_initialize(&controller_pool.tables[i]);
        __atomic_store_n(&controller_pool.initialized, 2, __ATOMIC_RELEASE);
    } else {
        while (__atomic_load_n(&controller_pool.initialized, __ATOMIC_ACQUIRE) != 2)
            __asm__ volatile("pause");
    }
    return &controller_pool.tables[index];
}

EXPORT void *nhl24_input_action_storage(void *record) {
    if (!record) return (void *)0;
    u32 selector = *(u32 *)((u8 *)record + 0x2190);
    if (selector > 12) return (void *)0;
    return selector ? nhl27_controller_table(selector-1) : (u8 *)record+0x1580;
}

/* Source 3d8330 resets all 12 controller tables without clearing owners. */
EXPORT void nhl27_controller_reset_all(void) {
    for (u32 i=0;i<12;++i) nhl27_action_reset(nhl27_controller_table(i));
}

/* Source 3e7a10 differs from ordinary unassignment: reset the formerly selected
 * controller table before switching the feature back to its local storage.
 */
EXPORT int nhl24_input_reset_feature(void *record) {
    if (!record) return MISSING_BINDING;
    u32 selector=*(u32 *)((u8 *)record+0x2190);
    if (selector>12) return INVALID_ARGUMENT;
    nhl27_action_reset((u8 *)record+0x1580);
    if (selector) nhl27_action_reset(nhl27_controller_table(selector-1));
    *(u32 *)((u8 *)record+0x2190)=0;
    sync_selected_bytes(record);
    return OK;
}

/* Source 0x3e7b90/0x3e7bf0. Always reset local history, even when assigning
 * the same controller again. Optional selected-table reset is an exact-byte
 * source tweak (==1), not a truthiness test. Reset preserves each table owner.
 */
EXPORT int nhl24_input_assign_controller(void *record, int index, u8 reset_selected) {
    if (!record) return MISSING_BINDING;
    if (index < -1 || index >= 12) return INVALID_ARGUMENT;
    *(u32 *)((u8 *)record + 0x2190) = (u32)(index+1);
    nhl27_action_reset((u8 *)record+0x1580);
    if (reset_selected == 1)
        nhl27_action_reset(nhl24_input_action_storage(record));
    sync_selected_bytes(record);
    return OK;
}

/* Requires the paired allocation patch: 0x1580 -> 0x21a0 at RVA 0x3ffc58.
 * This helper must never receive an unextended, original binding record.
 */
EXPORT void *nhl24_input_initialize_extension(void *record) {
    if (!record) return (void *)0;
    void *storage = (u8 *)record + 0x1580;
    nhl27_action_initialize(storage);
    *(u32 *)((u8 *)record + 0x2190) = 0;
    return storage;
}

/* Replacement for the single CALL at RVA 0x3ffc9e, before publication of the
 * newly allocated binding record. Derive the image base from that call's return
 * address so the guest hook remains valid under image relocation. Only this
 * exact callsite may invoke the hook. The installer must verify the binary hash
 * and apply allocation growth and call replacement as one transaction.
 */
EXPORT GAME_ABI void *nhl24_input_publish_extended(void *cache, void *record) {
    typedef void *(GAME_ABI *publish_fn)(void *, void *);
    address base = (address)__builtin_return_address(0) - 0x3ffca3;
    nhl24_input_initialize_extension(record);
    return ((publish_fn)(base + 0x407290))(cache, record);
}

EXPORT int nhl27_action_read(const void *storage, u32 action, u8 *value, u32 *changed_tick) {
    if (!value || !changed_tick) return INVALID_ARGUMENT;
    if (action >= 244) return UNSUPPORTED_ACTION;
    if (!storage) return MISSING_BINDING;
    const struct action_table *table = storage;
    *value = table->slots[action].value;
    *changed_tick = table->slots[action].changed_tick;
    return OK;
}

EXPORT int nhl27_action_write(void *storage, u32 action, u8 value) {
    if (action >= 244) return UNSUPPORTED_ACTION;
    if (!storage) return MISSING_BINDING;
    struct action_table *table = storage;
    if (table->slots[action].value != value) {
        table->slots[action].value = value;
        table->slots[action].changed_tick = table->tick;
    }
    return OK;
}

/* Source 0x3d72c0. Call at the input frame boundary, before producers execute.
 * Calling twice in a frame would erase edges: scheduling is the caller's job.
 */
EXPORT int nhl27_action_begin_frame(void *storage, u32 tick) {
    if (!storage) return MISSING_BINDING;
    struct action_table *table = storage;
    table->tick = tick;
    for (u32 i = 0; i < 244; ++i) table->slots[i].previous = table->slots[i].value;
    for (u32 i = 0; i < 40; ++i) table->analog[i].previous_bits = table->analog[i].value_bits;
    return OK;
}

/* Source 0x4b48580: inactive (-1) controller slots are not advanced. The owner
 * is updated AFTER snapshotting and BEFORE input producers run. No actor is
 * required: the service retains input history while a controller is unassigned.
 */
EXPORT int nhl27_controller_begin_frame(u32 index, int active, u32 tick, void *owner) {
    if (index >= 12) return INVALID_ARGUMENT;
    if (active == -1) return OK;
    struct action_table *table = nhl27_controller_table(index);
    nhl27_action_begin_frame(table, tick);
    *(void **)((u8 *)table+0xc08) = owner;
    return OK;
}

EXPORT GAME_ABI void nhl24_input_human_frame_guest(address base, u32 index, void *raw) {
    if (index >= 12) return;
    int active = *(signed char *)(base+0xba479a0+index*0xf0+4);
    nhl27_controller_begin_frame(index, active, *(u32 *)(base+0xba45a04), raw);
}

/* Source 0x3d8030/0x3d8040 use XOR 1 and bytewise AND, not C logical NOT.
 * Preserve that behavior even for a non-normalized input byte.
 */
EXPORT int nhl27_action_edges(const void *storage, u32 action, u8 *pressed, u8 *released) {
    if (!pressed || !released) return INVALID_ARGUMENT;
    if (action >= 244) return UNSUPPORTED_ACTION;
    if (!storage) return MISSING_BINDING;
    const struct action_slot *slot = &((const struct action_table *)storage)->slots[action];
    *pressed = (u8)((slot->previous ^ 1) & slot->value);
    *released = (u8)((slot->value ^ 1) & slot->previous);
    return OK;
}

static unsigned offset_for(u32 action) {
    switch (action) {
    case 224: return 0x12f0; /* SignalSpin360CW */
    case 223: return 0x12e8; /* SignalSpin360CCW */
    case 221: return 0x12c8; /* SignalShotRelease */
    case 172: return 0x1570; /* FakeShotDefault */
    case 144: return 0x1428; /* Lacrosse prepare forehand */
    case 219: return 0x1290; /* SignalGlideFwdDeke */
    case 218: return 0x1288; /* SignalEnterLoosePuckDeking */
    case 139: return 0x720;  /* Forward between-legs deke */
    default: return 0;
    }
}

static volatile u8 *binding(void *record, unsigned offset) {
    return *(volatile u8 **)((u8 *)record + offset);
}

static void sync_selected_bytes(void *record) {
    const u32 actions[] = {139,144,172,218,219,221,223,224};
    const struct action_table *table = nhl24_input_action_storage(record);
    if (!table) return;
    for (u32 i = 0; i < 8; ++i) {
        volatile u8 *slot = binding(record, offset_for(actions[i]));
        if (slot) *slot = table->slots[actions[i]].value;
    }
}

/* GSBool asset fields are eight bytes before their runtime pointer fields.
 * BoolAsset cast 0x8e1510 returns its PRIMARY interface for 0x7a327d7d;
 * its primary vtable +0xa8 tail-calls the generic writer 0x8a7ef0.
 * Compare asset identities before resolving any additional context properties.
 */
EXPORT u32 nhl24_input_action_for_asset(const void *gs_asset, const void *asset) {
    static const u32 actions[8] = {224, 223, 221, 172, 144, 219, 218, 139};
    if (!gs_asset || !asset) return 0xffffffffu;
    for (u32 i = 0; i < 8; ++i)
        if (*(void *const *)((const u8 *)gs_asset + offset_for(actions[i]) - 8) == asset)
            return actions[i];
    return 0xffffffffu;
}

EXPORT int nhl24_input_observe_asset_write(void *record, const void *gs_asset,
                                         const void *asset, void *destination, u8 value) {
    u32 action = nhl24_input_action_for_asset(gs_asset, asset);
    if (action == 0xffffffffu) return UNSUPPORTED_ACTION;
    if (!record || !destination || binding(record, offset_for(action)) != destination)
        return MISSING_BINDING;
    /* The displaced native store owns the byte write; only record its event. */
    return nhl27_action_write(nhl24_input_action_storage(record), action, value);
}

/* 0x100 means an unrelated native asset: execute its original getter. For the
 * mapped action assets, a missing feature defaults to zero, as source 0x3d6c30
 * does through its default getter. Preserve non-normalized byte values.
 */
EXPORT u32 nhl24_input_read_asset_value(void *record, const void *gs_asset, const void *asset) {
    u32 action = nhl24_input_action_for_asset(gs_asset, asset);
    if (action == 0xffffffffu) return 0x100;
    const struct action_table *table = nhl24_input_action_storage(record);
    return table ? table->slots[action].value : 0;
}

EXPORT GAME_ABI u32 nhl24_input_read_asset_guest(address base, void *asset, void *context) {
    typedef void *(GAME_ABI *binding_getter)(void *, void *);
    void *root = *(void **)(base+0xac40350);
    if (!root) return 0x100;
    void *gs_asset = *(void **)((u8 *)root+0x68);
    if (nhl24_input_action_for_asset(gs_asset,asset) == 0xffffffffu) return 0x100;
    if (!context) return 0;
    void *record = ((binding_getter)(base+0x3ffbf0))(gs_asset,context);
    return nhl24_input_read_asset_value(record,gs_asset,asset);
}

EXPORT GAME_ABI int nhl24_input_observe_asset_guest(address base, void *asset,
                                                  void *context, void *destination, u8 value) {
    typedef void *(GAME_ABI *binding_getter)(void *, void *);
    void *root = *(void **)(base + 0xac40350);
    if (!root) return MISSING_BINDING;
    void *gs_asset = *(void **)((u8 *)root + 0x68);
    if (nhl24_input_action_for_asset(gs_asset, asset) == 0xffffffffu) return UNSUPPORTED_ACTION;
    void *record = ((binding_getter)(base + 0x3ffbf0))(gs_asset, context);
    return nhl24_input_observe_asset_write(record, gs_asset, asset, destination, value);
}

/* Source 0x470dcf0 snapshots local (non-controller-pool) actors before the
 * frame's game updates. Human pool snapshots have a separate input callback.
 * This entry deliberately never advances a human-controlled actor's table.
 */
EXPORT int nhl24_input_begin_local_actor_frame(void *record, int controller, u32 tick) {
    if (!record) return MISSING_BINDING;
    if (controller >= 0) return OK;
    return nhl27_action_begin_frame((u8 *)record + 0x1580, tick);
}

EXPORT GAME_ABI void nhl24_input_local_frame_guest(address base) {
    typedef void *(GAME_ABI *binding_getter)(void *, void *);
    void *root = *(void **)(base + 0xac40350);
    if (!root) return;
    void *asset = *(void **)((u8 *)root + 0x68);
    if (!asset) return;
    u32 tick = *(u32 *)(base + 0xba45a04);
    /* Two 20-player team arrays are contiguous, independently of the separate
     * all-player-ID lookup at 0xba5d7e0. Match source loop order exactly.
     */
    for (u32 i = 0; i < 40; ++i) {
        void *actor = *(void **)(base + 0xba3c2c0 + 8*i);
        if (!actor) continue;
        void *animation = *(void **)((u8 *)actor + 0x60);
        if (!animation) continue;
        animation = *(void **)((u8 *)animation + 0xc28);
        if (!animation) continue;
        void *context = *(void **)((u8 *)animation + 0x20);
        if (!context) continue;
        void *record = ((binding_getter)(base+0x3ffbf0))(asset, (u8 *)context+0x60);
        if (!record) continue;
        /* Source checks the input feature's pool index, not actor.controller.
         * Those can differ during initialization and controller handoffs.
         */
        u32 selector = *(u32 *)((u8 *)record+0x2190);
        if (selector == 0)
            nhl24_input_begin_local_actor_frame(record, -1, tick);
    }
}

EXPORT int nhl24_input_read(void *record, u32 action, u8 *value) {
    unsigned offset = offset_for(action);
    if (!value) return INVALID_ARGUMENT;
    if (!offset) return UNSUPPORTED_ACTION;
    if (!record) return MISSING_BINDING;
    volatile u8 *slot = binding(record, offset);
    if (!slot) return MISSING_BINDING;
    *value = *slot; /* Match native byte reads, without silently normalizing. */
    return OK;
}

EXPORT int nhl24_input_write(void *record, u32 action, u8 value) {
    unsigned offset = offset_for(action);
    if (!offset) return UNSUPPORTED_ACTION;
    if (!record) return MISSING_BINDING;
    volatile u8 *slot = binding(record, offset);
    if (!slot) return MISSING_BINDING;
    *slot = value;
    return OK;
}

/* Producer entry point for an extended record. Unlike the legacy byte bridge,
 * this records every producer write in the source action service. Call at the
 * producer, not from a once-per-frame poll: multiple writes in a frame matter.
 * Allocation/publication and frame-boundary hooks must be installed first.
 */
EXPORT int nhl24_input_produce(void *record, u32 action, u8 value) {
    unsigned offset = offset_for(action);
    if (!offset) return UNSUPPORTED_ACTION;
    if (!record) return MISSING_BINDING;
    volatile u8 *slot = binding(record, offset);
    if (!slot) return MISSING_BINDING;
    int status = nhl27_action_write(nhl24_input_action_storage(record), action, value);
    if (status != OK) return status;
    *slot = value;
    return OK;
}

/* Explicit guest ABI also permits executing the assembly trampoline in the
 * Windows fixture without confusing its register convention with Win64. */
EXPORT GAME_ABI int nhl24_input_produce_guest(void *record, u32 action, u8 value) {
    return nhl24_input_produce(record, action, value);
}

/* Conditional native stores first compare the backing byte with the desired
 * value. Observe their write predicate without changing that byte: writing it
 * here would change the original CMP's flags and could change later branches.
 * The displaced native sequence remains responsible for the actual store.
 */
EXPORT GAME_ABI int nhl24_input_observe_change_guest(void *record, u32 action, u8 value) {
    unsigned offset = offset_for(action);
    if (!offset) return UNSUPPORTED_ACTION;
    if (!record) return MISSING_BINDING;
    volatile u8 *slot = binding(record, offset);
    if (!slot) return MISSING_BINDING;
    if (*slot != value)
        return nhl27_action_write(nhl24_input_action_storage(record), action, value);
    return OK;
}

/* For compiler-spilled records, check the exact native class before touching
 * the extension. This avoids pretending a stack-slot inference proves type.
 * The paired allocation patch grows every instance carrying this vtable.
 */
EXPORT GAME_ABI int nhl24_input_guarded_produce_guest(void *record, u32 action, u8 value,
                                                    address base, u32 changed_only) {
    if (!record || *(address *)record != base + 0xa426580) return MISSING_BINDING;
    return changed_only ? nhl24_input_observe_change_guest(record, action, value)
                        : nhl24_input_produce(record, action, value);
}

EXPORT GAME_ABI int nhl24_input_observe_predicate_guest(void *record, u32 action, u8 value,
                                                      address base, address predicate, u32 kind) {
    if (!record || *(address *)record != base + 0xa426580) return MISSING_BINDING;
    unsigned offset = offset_for(action);
    if (!offset) return UNSUPPORTED_ACTION;
    volatile u8 *slot = binding(record, offset);
    if (!slot) return MISSING_BINDING;
    int selected;
    if (kind == 1) {
        /* 0x29cea73: clear shot-release byte and a human-controlled actor. */
        selected = *slot == 0 && *(signed char *)(predicate + 0x23c) >= 0;
    } else if (kind == 2) {
        /* 0x2a4162b: the low stack byte OR the current backing byte is nonzero. */
        selected = ((u8)predicate | *slot) != 0;
    } else {
        return INVALID_ARGUMENT;
    }
    return selected ? nhl27_action_write(nhl24_input_action_storage(record), action, value) : OK;
}

/* Call only while the game's evaluation context is live, on its owning thread.
 * The caller must verify the exact target executable hash before supplying base.
 * The Windows fixture build never invokes these game functions.
 */
EXPORT void *nhl24_input_resolve(address base, void *evaluation_context) {
    typedef void *(GAME_ABI *root_getter)(void *);
    typedef void *(GAME_ABI *binding_getter)(void *, void *);
    if (!base || (address)evaluation_context < 0x60) return (void *)0;
    root_getter get_root = (root_getter)(base + 0x2ec80c0);
    binding_getter get_binding = (binding_getter)(base + 0x3ffbf0);
    void *wrapper = get_root((u8 *)evaluation_context - 0x60);
    if (!wrapper) return (void *)0;
    void *root = *(void **)((u8 *)wrapper + 0x10);
    if (!root) return (void *)0;
    void *asset = *(void **)((u8 *)root + 0x68);
    if (!asset) return (void *)0;
    return get_binding(asset, evaluation_context);
}

EXPORT GAME_ABI void nhl24_input_assign_actor_guest(address base, void *actor, int index) {
    if (!actor || index < -1 || index >= 12) return;
    void *animation = *(void **)((u8 *)actor+0x60);
    if (!animation) return;
    animation = *(void **)((u8 *)animation+0xc28);
    if (!animation) return;
    void *context = *(void **)((u8 *)animation+0x20);
    if (!context) return;
    void *record = nhl24_input_resolve(base, (u8 *)context+0x60);
    /* Source default gResetInputManagerOnLink is false (0x3f8285, R8D=0).
     * Keep the setting explicit in the core API for a later studio control.
     */
    nhl24_input_assign_controller(record, index, 0);
}

/* Source 46d4c10 resets the pool, then every player's action feature, before
 * resetting controller records. Target 2a33660 is the corresponding controller
 * initialization routine. The target's 40-entry ID table is also the full-player
 * reset traversal used at 2c92750; no controller-index filtering is appropriate.
 */
EXPORT GAME_ABI void nhl24_input_reset_all_guest(address base) {
    nhl27_controller_reset_all();
    for (u32 i=0;i<40;++i) {
        void *actor=*(void **)(base+0xba5d7e0+8*i);
        if (!actor) continue;
        void *animation=*(void **)((u8 *)actor+0x60);
        if (!animation) continue;
        void *feature=*(void **)((u8 *)animation+0xc28);
        if (!feature) continue;
        void *context=*(void **)((u8 *)feature+0x20);
        if (!context) continue;
        nhl24_input_reset_feature(nhl24_input_resolve(base,(u8 *)context+0x60));
    }
}

/* Named entry points remove action-index translation from hook call sites. */
#define ADAPTER(name, action) \
    EXPORT int nhl24_##name##_read(void *record, u8 *value) { \
        return nhl24_input_read(record, action, value); \
    } \
    EXPORT int nhl24_##name##_write(void *record, u8 value) { \
        return nhl24_input_write(record, action, value); \
    } \
    EXPORT int nhl24_##name##_produce(void *record, u8 value) { \
        return nhl24_input_produce(record, action, value); \
    }
ADAPTER(spin_cw, 224)
ADAPTER(spin_ccw, 223)
ADAPTER(shot_release, 221)
ADAPTER(fake_shot, 172)
ADAPTER(lacrosse_prepare, 144)
ADAPTER(glide_deke, 219)
ADAPTER(enter_deke, 218)
ADAPTER(between_legs, 139)
