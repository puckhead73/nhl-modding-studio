# NHL27 → NHL24 deke conversion and isolated motion trial

> Fresh-session starting point: [2026-10-16 continuation roadmap](../plans/nhl27-deke-backport-session-handoff-2026-10-16.md). The entries below include historical build hashes and test counts. Use the roadmap and current runtime/build/audit receipts for the latest status; the full port remains uninstalled.

## 2026-10-16 continued integration (not installed)

Current candidate: `6b83b5b79eeaa5fee1382c5b16f5f74345353baeb31717a6b86a4468caba0308`, 137 declared patches. The complete port remains uninstalled and inactive; game files are unchanged. Current validation: 50 input tests, 28 panic tests, and four PTM tests pass. Later counts below describe earlier builds.

The glide producer now decodes the source packed float format (all 65,536 values checked in both lanes), preserves exact active-byte comparisons, gates context selection by puck possession, and skips the native-only human third-controller override. These eight executable changes do not yet establish whole-producer parity.

Added the complete outer panic frame update (12,000 original-code comparisons), closest-distance reduction (12,000), and full reset (4,000). The alignment guard's tracker belongs to the actor: source `453cc30` returns actor+718 and constructor `4a3da00` stores that actor at tracker+10. It is not the puck. Native primary PTM path query `325af0` supplies the live begin/end vector; the native owner pose is actor+10 then +30. Native skating-style columns at table `ba1a060` map spine/distance/height/gait to +2d18/+2d78/+2dd8/+2da8. Original-constructor comparison covers 652 row/stride cases. Takeoff output lookup follows native initializer `2ec3776..2ec37ac`: root `2e8ded0`, SkatingGS `40c5a0`, style record `411510`, writable slots +128/+108/+100/+f8. Original native instructions and nullable slots pass comparison.

PTM transition work now includes source-equivalent cleanup (`280440`/`2803c0`), rig-vector exposure (`280410`), and pose capture (`2804d0`). Cleanup tests cover 2,048 ownership/reentrancy combinations and 10,241 vector-count comparisons; capture covers 4,096 phase/float/publication cases. The vector getter returns the vector header at +40, not its data pointer, and compares the low 32 bits of the shifted span. Native services use pose refcounts +18/destructor +18 and controller refcounts +8/destructor +20.

Native PTM match class `326acae6` resolves through cast `30d530`, vtable `a41a508`, evaluator `30d9b0`, destructor `30d260`. NHL24 already has a 70-byte transition at match+1730; its evaluator creates it for a configured match blend-out asset. The native constructor is inline at `312726..3127ae`, capture at `3127d6..312bf6`, blend consumption at `311680..311a25`, and prepared rig offsets at `3125ce`. This is distinct from the missing newer PtmBlend controller trigger. Native transition parameters live at its +10, with duration/curve at parameters+d8/+dc; source uses direct transition+490/+494. The layouts cannot be overlaid.

Native pose allocator `8bb850` matches the inline capture allocation; compose `773ad0` reads the controller property remap at controller+40 -> +98 -> +4; pose publication uses `7737f0`. Native clip sampling uses clip+8, factory virtual +48, seek secondary interface +10, mirror scope `2a1780`, mirror constructor `a0f100`, and controller evaluate +58. The translated source `280f20` sampling order passes 3,072 original-code comparisons, including nested mirror counters and callback changes to the mirror byte. These services are connected to the port-owned capture/cleanup entrypoints. Transition creation, frame hooks, trajectory correction, panic engine integration and final activation remain unfinished.

Corrected four inlined team-capacity release hooks to run before the signed actor-controller-index gate: `2a2b997`, `2a2bd47`, `2a2c424`, `2a2c7a3`. Source `46d8ad0` and `46d8fa0` reset the action feature even when the actor's index is already negative. Moving the hooks preserves that behavior. Both source `46d9480` handoff branches reset: the first calls `46d6480` at `46d9582`, and the second resets inline at `46d97af`. The previous conclusion that the first branch should remain unhooked was incorrect; target `2a2eac2` now resets before unassignment. Added target inactivity-release hook `2a4ea3b` corresponding to source `46e1ed5`. All 49 input tests pass, including assigned/unassigned actor and machine-state cases.

Added the source-equivalent 96-byte panic constructor/destructor and an 18-owner side table. Constructor/destructor comparison covers 2,000 original-machine cases, allocator padding, rating order, stale identities, and slot reuse. Guest lifecycle hooks still require the complete engine adapter.

Recovered locator registry property `f28b0f8` -> target `ac8fc18`, class `738a4434`; native constructor `8c7130`, size `770`, remove operation `8c55d0`. The native property uses live getter `+78` and writable getter `+88`. Implemented registry allocation/publication with native allocation and refcounts; original `4a544f0` comparison passes existing, newly allocated, reentrant replacement, and already-published cases. All 22 panic tests pass. This does not yet connect the locator to skating updates.

`panic_alignment_audit.py` now reproduces the source skating-feature configuration audit and stages two native locator-parameter records. Of 12 configurations, physicsplayerleft, physicsplayerright and fb_player use WorldLocatorParam and LocatorAlignmentSolver. Their closure has 21 assets, no missing or ambiguous partitions, and ten still-unsupported native adapter classes. Source SkatingFeature allocation is `650` bytes (`4009c0`), with locator at `+640`; native allocation is only `70` (`6ab390`). Never apply source `+640` to the target feature. Native ScenarioLocatorParamsAsset is qualified against the original launch record (24 bytes, flags at `+10`, params interface at `+8`). Matching source AlignSolverAsset/AlignSolverInfoAsset class hashes `3ff2d30f`/`4d38e672` were not found in the target immediate scan. The solver/handler runtime and skating-feature integration remain necessary. Audit: `local/nhl27-eboot/compatibility/deke-trial/panic-alignment/report.json`.

Implemented the stumble gate (`4a53b70`, 12,000 original-machine cases), reaction scheduler (`4a51a80`, 10,000 cases including ordered service calls and mid-frame mutations), cancellation tail (`4a53e10`, 6,000 cases with changing session results), and takeoff modifier (`4a52300`, 12,000 cases including NaNs, phase boundaries, missing values and cancellation). The scheduler reacquires bindings at the original call sites and resamples motion state after reactions. The three cancellation gates remain independent.

`panic_tuning.py` recovers all 37 Boolean/integer/float registrations from `4a5e3b1..4a5ed09`, including exact float bits and min/max values. Output is `local/nhl27-eboot/compatibility/deke-trial/panic-tuning.json`. This supplies the source defaults without inferring them from names. The takeoff hold phase preserves existing values; an expired effect does not start cancellation blending until the effect lookup reports absence.

Added registry-backed motion reads for staged NodeActive `9f2ed2b822171119` and secondary movement array `c9f1cc065e44d353`, holding the native registry lock while resolving and reading. Native enum cast `7bada0` validates class `1d52f677`. Native Bool read-only storage is vtable `+80` (`7a1a90`), not `+78`: the latter returns the asset default (`8a87a0`). The tests execute the actual native casts and getter and distinguish initialized context values from defaults without allocating a context block. Five asset-binding tests pass.

Normalized speed source `452d430` uses the same chain as target inline `2b1fdfc..2b1fe49`: actor+60 -> animation+8 -> scale+30, and actor+10 -> pose+90. The new adapter preserves source NaN, signed-zero and epsilon handling; 180 original-machine edge cases pass.

Verified lifecycle insertion candidates: target construction after component +d0 publication at `2bf036c` (owner R13, actor R15), cleanup at `2bbeb1a` after existing component destruction (owner R13), update at `2bf1810` after `2c45860` (owner R15). Source allocates the 96-byte panic component after +d0, and destroys it after +d0 before resetting owner flags. Target owner+e8 is already used for flags; a separate state allocation is required. These lifecycle hooks are not yet installed.

New dependency finding: source effect property `f20da50` has class `4db1bd69`, with no matching immediate in the target index. Nearby target `ba078e8` is a different class (`5501fa4c`) and must not be substituted. Source `BigPanicEffect` (`f34b2901`, vtable `e819ac0`) and `TakeOffEffect` (`ce986eb5`, vtable `e405bd8`) are initialized by `4b0b520`; the corresponding source service/producers still require integration. Their names and hashes were not found in the target searches. This is remaining engineering work, not a request for user input.

Earlier candidate before the normalized-speed addition: `ccd085e70e8183d046949740d1017bd04b18eb324f831506b9b83d0f65c8ea31`, 127 declared patches. Full engine adapters, lifecycle hooks, remaining input producer paths, PTM transitions and final graph integration remain required.

## 2026-10-16 runtime continuation (work in progress)

The private candidate now includes source-equivalent type-2 deke reactions (2,077 original-machine cases), type-3 poke reactions (12,000 cases), request generation (8,000 cases), and between-feet geometry (12,000 cases). Original overlap and segment-intersection leaves are retained, with their initialized constants verified at source `24fb1a0`. Zero, one and epsilon match native target initializer `1df34c0`; epsilon is exactly `0x1p-16f`. The new RIP-reference scanner includes AVX references omitted by the earlier index.

Source previous-foot service `4543f10 -> 4c2bd30 -> 4a18a40` uses manager actor-ID array `+1d8`, animation `+9f0`, previous transforms `+28`. Target `30e7ff0` uses the same actor-ID array and transform selection, with animation `+9c0` and remapped bone IDs 20..23. The guest between-legs adapter combines these previous positions with current `2abdfe0` probes and mode-zero puck prediction. Source final SETBE parameter comparisons accept unordered values; the adapter preserves that behavior.

Source cached PTM flag `animation+48 -> +e6` is copied from `api_nhlptmpostsceneop_animparams.PTMSkatingActive` by `527d96/527d9c`. Hash-checked source EBX identifies this as existing asset `e0833ad3a430af00`. Target `521aa8..521abd` caches the same signal at `animation+48 -> +ce`. The candidate replaces the three-byte SETNE at `521ab2` with NOPs to retain raw source Boolean values; all 256 byte inputs match source cache writes. The existing post-scene producer preserves the sampling phase without adding a second scheduler.

The source panic/closest movement predicates and target `8a7780` parent-context lookup pass 10,000 original-machine comparisons. The adapter uses the staged native bit-array property key at +8 and byte count at +32, checks cached-active exactly one, and additionally checks NodeActive exactly one for PanicTurn bit 9. Closest-player bit 3 does not require NodeActive.

Latest private candidate SHA-256: `4037ca1462f3d4defd3ebed89807805ddc9efb28d773511cb97d15a19537a7a2`, 127 declared patches. This is not installed or active. Complete panic update/lifecycle, remaining producer parity, PTM behavior requirements, final graph integration and installation remain open. Work continues; these tests do not establish gameplay parity.

## Earlier private candidate — input service reset and panic components

The candidate includes 76 producer hooks, the generic asset writer, two Boolean readers, local actor snapshots, six human-sample hooks covering 12 slots, six ownership hooks, 31 reset-before-unassignment call wrappers and a full input-service reset hook. The latter resets the pool and all 40 actor features before controller initialization, preserving table owner fields. Allocation remains `0x21a0` with local storage at `+0x1580` and selector at `+0x2190`.

Candidate SHA-256: `7afb181a991563f9e017b0eaf50c2b124ae31a4af5e8de4105c58632e45f5e22`. All 48 input tests pass, including machine-state fixtures and declared-patch-only ELF restoration. Panic state operations pass five tests, including original-instruction attribute normalization. The source direction predicate is relocated with its exact constants and passes an execution test. These panic components are linked but have no complete runtime update integration yet.

Asset lookup now checks the state-bank anchor and resolves signals across registered banks because PTMActive/PathLock live in launch. It rejects conflicting definitions, holds one registry lock for all nine panic properties and publishes output only on complete success. Three host lookup tests pass; the guest lifetime/registry path is not live-qualified. Float serialization passes two tests against the original native record. The missing closest-distance FloatAsset and alignment-request BoolAsset are staged.

The graph contains 516 fragments, 1,382 objects and 40 replacements, with no missing IDs/interface/structural errors. **63 executable-behavior requirements remain; none were bypassed. Not installed or active.** Remaining producer semantics, ownership paths, gesture scheduling, full panic reactions/lifecycle and PTM transitions still block completion. No game installation files were modified. Entries below describe historical stages.


The 2026-10-16 candidate additionally links the type-1 skating reaction and native foot/session adapters. The complete original source reaction `4a53360` passes 198 comparison cases with mocked engine services, including preservation and cancellation. All ten panic tests pass. The original target node lifecycle `a71be0` executes entry, exit, repeated-call and disabled-entry cases correctly. The node-active signal and a launch-node activation-array extension are staged, while source lifecycle parity remains a requirement. Inherited native fixups are bound to the original bank hash; they preserve transition arrays and secondary-interface pointers.

Recovered services: source feature property `f20da00` (class hash `f3aae77e`) corresponds to target `ba078c0`; both copy feature `+68` into animation component `+48`. The source `+e6` PTM flag still needs its target/port configuration mapping. Source `465c3f0` ignores its apparent session argument and uses manager disabled byte plus unsigned state-range arithmetic; target equivalents are `ba45a39` and `ba475ac`, verified at `10574b4`, `28325e6`, and `29c13b5`. The session adapter matches all tested original-instruction boundary cases.

Source virtual foot probes `25..28` map to target `20..23`. Source initializer `4c2a8f0` and target `2fec800` use the same handed LeftFoot/RightFoot pairing and bit-identical four offset vectors (`dee6310/dee54c0/deead00/deebb70` versus `8fcf3b0..8fcf3e0`). Target `2abdfe0` accepts `(actor index, bone ID, output vector, handedness)`, reads pose index `+94`, and uses target actor handedness `+170` (source `+180`). Target pose matrices reside at animation `+9c0`, matrix store `+18`, stride `30`. Source probe `4a3c260` uses two foot probes, puck position and a two-tick prediction (`44c6850`, mode 0: position + velocity*2, clamp negative Y to zero), then original segment test `24f74a0`. Its geometry globals `f457870/880/8d0` appear to correspond to target `af62030/40/90`; their initialization/parity is still unqualified. Target `2f0a140` is a goalie-only state writer, not a substitute for the generic source predicate. Source actor `+718` is an embedded component address, not a pointer field; the direction predicate does not use that argument, and the between-legs helper only accesses its actor at `+10`.

## Producer hooks and native asset adapters



The private producer candidate now contains the paired allocation/publication patch plus **36 producer trampolines**: action 139 has four, action 144 has three, action 218 has six, action 219 has two, action 221 has sixteen, action 223 has two and action 224 has three. The inventory contains 73 candidate stores across seven actions. Gesture action 172 still has no emitted producer hooks; the stack-spilled glide paths remain unqualified. These are partial producer coverage, not all eight complete runtime adapters.



Each trampoline preserves all general registers, XMM0–15, flags, stack alignment and the guest red zone, executes the displaced load and returns to the original store sequence. Thirty-two use the timestamp/native-byte producer. Four conditional stores instead call `nhl24_input_observe_change_guest`, which updates the added action table only when the original backing byte differs; it leaves that byte unchanged so the original comparison, flags and conditional store remain intact. Isolated native fixtures compare original and hooked machine states with null/non-null bindings and equal/unequal initial bytes. ELF tests independently verify outbound and return branches, the selected helper, and displaced bytes. `runtime-build/eboot-input-producer-candidate.elf` has SHA-256 `da0263045d1a1b739b1f3acfe4a568408f2e4a980cdb5447be6b2ba79f1ee13c`. It remains uninstalled and lacks frame scheduling.



The scanner now resolves immutable signed-relative jump tables only when an adjacent unsigned bounds check dominates the exact dispatch sequence. It rejects bypass entries, out-of-function targets, mutable/unreadable tables, mismatched indices, and unknown indirect transfers. This qualifies the ten-case lacrosse switch at `0x2bb5080` (table `0x9092eb0`) and the five-case switch at `0x311d8d1`. Resolved case edges participate in the same all-path record-provenance analysis as direct branches. The human-input switch at `0x3152c07` is also resolved: `DEC EAX` at `0x3152bee` clears the upper index bits before `CMP EAX,4`. The qualifier rejects branches into the comparison that would bypass this zero-extension, rather than assuming a 32-bit comparison bounds a 64-bit table index. This enables hooks at `0x31540b9`, `0x315492d` and `0x315493c`. Stack-spilled records remain unqualified; the glide producer spills its getter result at `0x2eca8c5` into `[rbp-0x50]`, and that slot is later reused for float storage, so simply carrying its original type through the whole function would be incorrect.



`ant_action_adapters.py` rewrites the eight source action references to the existing target Boolean objects, preserving context-property identity and reset interfaces. It does not clone Boolean objects, which would create independent properties. Every adapter retains an explicit producer/scheduling requirement. The separate adapted manifest leaves the original staged fragments intact.



The corrected fake-shot audit resolves **exported instance GUIDs**, not just partition roots. Source action 171's mapping imports matcher `cfaff536-b333-26b5-cfaf-f536b33326b5` at offset 168 inside partition `0e5bb51c-ae97-5b1b-0e5b-b51cae975b1b`. It does not reference that partition's root GestureAsset or its step. NHL24 already contains the same matcher identity and matching parameters/gamepad binding. `deke_gesture_stage.py` therefore stages only the new mapping and replacement default configuration, reusing the matcher, Xbox device and native FakeShot Boolean. The new mapping precedes the existing LB+R3 mapping, matching source order; scheduler behavior remains unqualified.



The adapted combined graph contains **512 fragments, 1,378 objects, 39 replacements, no unresolved identities, no interface errors and no structural audit errors**. Its **59 behavior requirements** still prevent installation. This includes the original 50 PTM/controller requirements, eight action-adapter requirements and the additional gesture-scheduler requirement. Panic runtime implementation, complete producer coverage, source scheduling, PTM consumer differences, runtime loading and final activation remain incomplete.



Tick correspondence is now supported by the update functions: source `0x470cd80` increments manager `0x103e1418 + 0x2cc = 0x103e16e4`; target `0x2a94040` increments `0xba45a04`. Both publish that tick through the corresponding game-state interface. Target `0x30dd620` also rewrites the tick during playback. Source animation callback `0x48a790` snapshots through the evaluation context's clock, which the source manager update sets at clock `+0xc`. These observations do not yet prove that a once-per-tick lazy getter hook preserves all source snapshot ordering, so that shortcut was not installed. Source snapshot callback `0x48a790` is installed at vtable `0xe3868e0 + 0x88`; constructors `0x291dd0` and `0x291e00` install that vtable. Its cast function `0x291cd0` recognizes class `0x9e75aeca`, absent from the target immediate scan. Target main-frame driver `0x2a94690` calls its clock update at `0x2a94964`, then converges at `0x2a949a7` before `0x2bbda50`. This is a candidate phase boundary, not an installed snapshot hook. Source metadata at `0xec62d80` links the constructors and names the class `ActionInputUpdateSceneOpAsset` through string `0xdc1e801`; registration alone does not prove this callback participates in the active gameplay scene. Its active scene placement remains unqualified.



PTM blend-out is now a confirmed implementation difference. Target PtmBlendAsset vtable `0xa424d18 + 0x48` creates a 728-byte controller through `0x36bdf0`/`0x36bf60`; controller update `0x369520` requires a non-null asset `+0x20` BlendOutBlend before starting its blend-out branch. Source asset vtable `0xe379408 + 0x60` creates an 800-byte controller through `0x2834d0`/`0x2837a0`. Source controller update `0x2843c0` invokes `0x27ffb0` when remaining time reaches controller `+0x308` and match `+0xbbc0` has no transition. That helper samples the outgoing trajectory, allocates `0x4a0` bytes of transition state, retains the previous controller, publishes the state at match `+0xbbc0`, and invokes `0x2804d0`. Leaving the target BlendOutBlend null does not implement this behavior. No source structure offsets were applied to target objects. Further tracing shows `0x2804d0` evaluates the previous controller and retains a pose resource; `0x27fa30` consumes the transition in match update `0x236850` at calls `0x238e54` and `0x239ce7`. `0x280410` exposes the cached vector only when its element count matches the referenced skeleton count. `0x2803c0` destroys the transition through `0x280440`, frees it through allocator virtual slot `+0x20`, and clears match `+0xbbc0`; callers include `0x236280` and `0x23b890`. Allocation alone therefore cannot implement the transition.



Validation of the implemented components and adapted asset records: **113 tests passed**. This includes generated producer trampolines executing on isolated native buffers, exported-instance resolution regression coverage, and corrected R3 mapping order/identity checks. Gameplay parity and installation are not covered by these tests.



## Input-state implementation and producer comparison



The native component now implements the source action table's reset, initialization, Boolean read/write, frame snapshot, and press/release operations. The layout is 244 eight-byte Boolean slots followed by 40 twenty-eight-byte analog slots, tick at `+0xc00`, and owner at `+0xc08`. Boolean slot `+1` is the previous-frame value; it is not padding. Change timestamps update only when the current byte changes. Frame snapshots preserve timestamps and copy current Boolean/analog values into their previous-value fields. Reset restores all source defaults, including exact analog range/threshold float bits, while preserving the owner; construction additionally clears the owner.



The compiled port was compared directly with four original, hash-verified NHL27 leaf routines on isolated native buffers: setter `0x3c9ae0`, snapshot `0x3d72c0`, pressed `0x3d8030`, and released `0x3d8040`. Tests cover every Boolean slot, repeated writes, non-normalized values, and tick wraparound. The independent bounded interpreter in `action_reset_evidence.py` reconstructs all 3,072 bytes written by reset `0x3d7570`; compiled reset/initialization match those bytes and preserve the expected tail. These tests do not execute game hooks or establish input scheduling parity.



Source frame snapshots are called from pooled input processing `0x4b48580`, inline actor processing `0x470dcf0`, and animation processing `0x48a790`. The first two use tick global `0x103e16e4`; pooled input processing snapshots before producer `0x4b46780`. A target hook must preserve these ordering distinctions, not poll the final Boolean values once per rendered frame.



`deke_gesture_audit.py` reproduces the action-172 producer investigation from hashed assets. FakeShotDefault uses the `LB_R3_matcher` gesture; its scalar matcher and timing parameters match NHL24. However, the source Default_Config includes **two** mappings while NHL24 has **one**. The additional source mapping drives **action 171** directly from an exported R3 matcher, as corrected above. Matching action 172's parameters therefore does not qualify the surrounding configuration or scheduler.



Glide producer `0x4a1c960` has corresponding target branches in `0x2eca610`: source actor offsets `+0x25c/+0x8dc` correspond to target `+0x23c/+0x7bc`, followed by the same observed nonzero-value gate. Further target writes occur in `0x2ecb960`; complete producer coverage and ordering remain unqualified.



`build_input_eboot.py` builds a private allocation/publication candidate. It grows the GSBool binding allocation from `0x1580` to `0x2190` at `0x3ffc58`, appending one action table. The replacement call at `0x3ffc9e` initializes the extension before invoking the original reference-counted publisher `0x407290`. The wrapper derives the relocated image base from the exact return address. Binding vtable `0xa426580` releases through `0x8aa1e0` → deleting destructor `0x4072e0` → the original allocator's free slot; it does not pass a fixed `0x1580` size to free.



The candidate reconstructs the original ELF payloads and exact trailing metadata, reuses the empty PT_NOTE header for a new read/execute segment at `0xbed4000`, and links the freestanding payload there. It verifies all original payload bytes while allowing only the allocation immediate, publication call and empty header to change. The private file is `runtime-build/eboot-input-allocation-candidate.elf`, SHA-256 `8273291a8cc802dc55db08fab3537b09e13fe3284a7f3b56cc9e49912f53c1c4`. It is **not installed or a complete deke port**. Source runtime loader code was inspected for additional PT_LOAD handling; loading this candidate in the installed runtime remains untested.



The publication hook was also executed in an isolated native fixture with the original NHL24 publisher and the exact caller/publisher address distance. Both empty-cache and existing-reference cases pass, preserving the original binding bytes, updating reference counts correctly and initializing only the extension. The old-reference fixture retains a reference, so destruction was inspected but not executed. Target ELF inspection now includes PS4 RELRO mappings, allowing the original vtable segment to be checked on round trip.



Allocation-only validation at the earlier stage: **94 tests passed**.



**Installation remains incomplete.** These are compiled runtime components, a private executable candidate, and reproducible evidence. Live allocation/lifetime qualification, producer hooks, native class/vtable integration, panic reactions, PTM differences, final package linking, and activation remain outstanding. No game files were modified by this work.



## Native input bridge implementation



`scripts/nhl27/runtime/input_bridge.c` now implements native reads and writes for all eight recovered input bindings, with sixteen named entry points and a context resolver using target functions `0x2ec80c0` and `0x3ffbf0`. It changes the existing NHL24 Boolean backing storage; it does not clone that storage or infer an input from another move. Unsupported actions and absent bindings return explicit status codes without modifying the caller's output. Byte values are preserved exactly.



`python scripts/nhl27/build_input_bridge.py` builds a freestanding x86-64 FreeBSD ELF object for subsequent guest-code integration and a no-entry-point Windows DLL for isolated tests. The DLL is a test fixture, not an emulator plugin. `runtime-build/input-bridge-build.json` records compiler commands and hashes and explicitly marks the component uninstalled and not installable on its own.



Compiled-code tests exercise all eight named read/write pairs against independent native pointer fixtures, checking adjacent-byte guards, unrelated bindings, unchanged record pointers, null bindings, unsupported actions and non-normalized byte values. The game-context resolver is compiled but has not been executed against a live game context. The action-state primitives described above implement timestamp behavior, but allocation and producer/frame integration remain outstanding alongside class/vtable integration, panic reactions and the PTM requirements. No executable, CAS or TOC installation occurred. This component does not make the full port ready for gameplay testing.



## Binding and lifecycle investigation



The full port remains unimplemented and uninstalled. This investigation recovered concrete target binding and lifecycle locations; it did not qualify input timing, port panic reactions, discharge the 50 PTM compatibility requirements, or activate the staged graph.



All eight candidate inputs are external references in NHL24 `AnimAssetRI_GSBoolAssets`, native ID `0x30b7d024f81ea1df`, type `0x16a79c0c`. Getter `0x3ffbf0` allocates a `0x1580`-byte per-context binding record and resolves each asset reference through virtual `+0x88`. The runtime pointer sits eight bytes after the asset field offset:



| Source action | Target signal | Asset field | Runtime pointer |

| --- | --- | --- | --- |

| 224 | SignalSpin360CW | 0x12e8 | 0x12f0 |

| 223 | SignalSpin360CCW | 0x12e0 | 0x12e8 |

| 221 | SignalShotRelease | 0x12c0 | 0x12c8 |

| 172 | FakeShotDefault | 0x1568 | 0x1570 |

| 144 | bgs_psk_lacrosse_prepare_forehand | 0x1420 | 0x1428 |

| 219 | SignalGlideFwdDeke | 0x1288 | 0x1290 |

| 218 | SignalEnterLoosePuckDeking | 0x1280 | 0x1288 |

| 139 | bgs_psk_forward_betweenLegsDeke | 0x718 | 0x720 |



The observed target lookup chain is actor `+0x60` → animation `+0xc28` → binding `+0x20`. `0x2ec80c0` resolves the root wrapper using that context; wrapper `+0x10` points to the root, root `+0x68` points to the Boolean asset record. `0x3ffbf0` receives this record and context `+0x60`. These are recovered addresses and layouts, not installed adapters. NHL24 lacrosse preparation writes are visible at `0x2bb5457` and following branches; NHL27 counterparts are `0x47e3b00`, `0x47e3bc0`, and `0x47e3c80`. Branch selection and timing still require comparison.



NHL27's action storage property at `0xf2244d0` is initialized by `0x3b01c0`, with vtable `0xe402230`. Slot `+0xb0` resolves to `0x3b0490`, which uses the evaluation context in RSI and falls back to the property's pointer at `+0x40`. The pool allocator `0x3d8060` allocates `0x90c0` bytes: 12 records of `0xc10` bytes. Each record starts with **244** eight-byte Boolean entries, followed by other input data at `+0x7a0`; the whole record must not be interpreted as 384 Boolean slots. The current tick is at `+0xc00`. The executable evidence tool finds 75 locally constant setter call sites spanning seven requested actions; this is not a complete producer inventory. Action 172's asset-driven gesture mapping was subsequently recovered as described above; it is not a direct constant-action setter call.



The panic component is a **96-byte SkatingManager**. NHL27 allocation is at `0x480d45a`, constructor `0x4a54380`, owner slot `+0xe8`, destructor `0x4a544d0`, and owner cleanup `0x480cea0`. NHL24's corresponding assignment and reset paths are `0x2bef040` and `0x2bbe800`; its owner `+0xe8` contains flags and cannot hold this pointer. The target owner-update function is `0x2bf10a0`, called at `0x2f0def0` from `0x2f0dde0` through actor `+0x208` (source actor uses `+0x228`). Candidate insertion `0x2bf1810` follows interaction update `0x2c45860`, with the owner in R15 and actor at owner `+0x90`. This identifies an insertion location but does not qualify a trampoline, sidecar lifetime, or the translated panic implementation.



Research tooling was corrected to scan overlapping E8 call candidates and read SELF virtual mappings correctly. Ghidra initially misclassified calls in the lacrosse routines as PIC branches, dropping reachable writes. Defining helpers `0x3c9ae0` and `0x3cd090` first and regenerating the callers restored those writes. Disassembly remains the check against decompiler errors.



`python scripts/nhl27/deke_runtime_evidence.py` reproduces the hash-bound evidence in private `runtime-binding-evidence.json`. `deke_input_audit.py` now records all eight native binding offsets. **84 tests passed** after the tracing and binding checks were added; the three executable evidence tests also passed after adding the target hook signatures. No game files were changed or gameplay tests run.



## Latest implementation state — motion matching and executable integration



**The full port is not installed or active. The runnable package remains the earlier 389-clip trial.** No gameplay testing was performed in this pass. The remaining work is native runtime implementation and qualification; user gameplay testing cannot substitute for it.



The dependency closure now contains **441 partitions and 130 reused partition boundaries**. The closure walker checks imported instance identities, rather than assuming that a known partition root contains every NHL27 export. This exposed two enumeration partitions containing five new move values: `LPD_TOEDRAG_BACKSKATING=25`, `LPD_TOEDRAG_STRAFE_LEFT=26`, `LPD_TOEDRAG_STRAFE_RIGHT=27`, `StrafingLeft=4`, and `StrafingRight=5`. Existing enumerator numeric values are unchanged. The source enumerations are serialized; executable selection of the new values remains unimplemented.



The controller compiler emits **290 of 298 non-clip/non-channel-map partitions**. New support covers eight PTM master databases, eleven owned 504-byte database records, three pose-feature runtime assets, PtmBlend, PTMMatch, and 17 target parameter/related layouts covering 222 audited fields. All eight weight assets now serialize: the four previously rejected override arrays contain only null references. Packed KD vectors retain their exact integer words; PCA dimensions, backing lengths, pointer qualifiers and owned database initialization are checked. A serialized record does not establish equivalent search or controller behavior.



The combined audit covers **511 fragments, 1,376 native objects, and 38 existing-object replacements**. It reports zero duplicate definitions, zero invalid relocation slots, and zero unresolved interface casts. **Eight unresolved identities remain, all ActionInputBool inputs.** Fifty explicit compatibility requirements cover source-only PTM semantics, KD search compatibility, feature reference frames, trajectory joint handling and blend fallback behavior. These requirements propagate into fragments and cause the linker to reject activation rather than silently omit behavior.



`ant_link.py` now supports explicit same-layout replacements, audited native type-table additions, and typed references to identities in other loaded banks. New cross-bank fixups retain the requested interface hash, absent-owner marker and zero flags used by the original state bank; they do not embed foreign blob offsets. Runtime bank load order remains unqualified. Three staged classes require type-table entries: AlternativeFeatureSchemaAsset (16 bytes), PtmBlendAsset (80 bytes), and PrevPoseBlendAsset (24 bytes). Their factories exist in NHL24; this does not register source-only classes.



Target cast functions confirm FeatureSchema interfaces at object +8 for motion-type, trajectory, position, velocity and alternative schemas. PrevPoseBlend exposes BlendAsset at its primary address. PTM master initialization callback `0x326c90` restores owned database vtables after memory-image relocation; compiled masters are marked for initialization. The previously suspected KD fixup `0x3b9e30` is a destructor and must not be called as initialization.



Source ActionInput getter `0x3d6c30` and setter `0x3d6cd0` use a shared action-state table. Each eight-byte slot contains a Boolean value and change timestamp; writes update the timestamp only on value changes. Table accessors `0x3e7b10`/`0x3e7b50` select an actor/context-local or pooled table. Matching an NHL24 Boolean asset name and reset flags does not reproduce this storage and scheduling contract. No input aliases or input executable adapters are installed.



Panic-turn tracing now includes source owner update `0x480e710`, callsite `0x480e8bf`, component pointer at owner +0xe8, component update `0x4a517d0`, panic update `0x4a51a80`, reset `0x4a51930`, request generation `0x4a53570`, state handlers `0x4a53360`/`0x4a52840`/`0x4a52cd0`, stumble request `0x4a53b70`, and event emission `0x4a53ce0`. These addresses and layouts belong to NHL27. NHL24 actor bindings, component allocation, lifecycle hooks and code implementations are **not ported**.



Validation: **79 tests pass**, including independently read target factory sizes for all 17 parameter layouts, real source PCA/KD payload preservation, initialization ownership, import closure regression, cross-bank interface records, and native type-table growth. These are offline checks. They do not establish gameplay parity or a completed installation.



Reproduce the current staging and audit:



```powershell

python scripts/nhl27/deke_closure_stage.py local/nhl27-eboot/compatibility

python scripts/nhl27/ant_controller_stage.py local/nhl27-eboot/compatibility

python scripts/nhl27/ant_tag_stage.py local/nhl27-eboot/compatibility

python scripts/nhl27/deke_link_audit.py local/nhl27-eboot/compatibility

python -m unittest discover -s scripts/nhl27 -p 'test_*.py'

```



Private evidence: `combined-link-audit.json`, `runtime-port-contract.json`, and `assembly-receipt.json` under `local/nhl27-eboot/compatibility/deke-trial/`. They explicitly retain `installed: false`. Final CAS/TOC assembly and activation have not been performed for this graph.



## Previous implementation state — complete dependency slice



**The complete port is not installed or active. Do not treat the earlier 389-clip package as the requested finished trial.** Gameplay testing remains with the user after integration; no additional gameplay test was performed in this pass.



The six integration roots expand to 439 source partitions and 132 existing native identity boundaries. This slice has no missing catalog imports or conflicting payload variants. Identity correspondence at those boundaries does not establish unchanged behavior. `deke_closure_stage.py` reproduces the traversal and stages resources by name, resource ID and stored hash, without assuming patch precedence for conflicting payloads.



All **136 DCT clips** in that slice (including all original 54 unmatched moves) now pass exact decoded-channel permutation and byte-exact inverse conversion. Seven source channel maps are covered; one new native CTD map is still required. The converter also exports 315 incoming/intermediate assets for inspection. These results are under `closure-stage/`, separate from the runnable package.



`ant_tags.py` and `ant_tag_stage.py` now serialize all 136 clip trees into target-sized records: 136 clip records, 136 animation records, and **476 embedded tag/collection records**. The native field offsets were traced through NHL24 serializers, including inherited timeline and monitored-tag fields. Unknown tag types fail closed. Default collections retain the complete tag array and separately index monitored tags. The original target bank contains 6,964 default collections; every monitored entry inspected was also in its collection's main tag array.



Two external identities remain unresolved in these clip/tag fragments, affecting 48 clips: `0xde0e918abd457be9` (`bgs_ptm_requestPanicTurns`) and `0xbf7ddeb8d65c16f5` (the panic-turn movement-mask item). Their additional dependencies and producers must be implemented, not replaced with a constant or an arbitrary existing bit. The source movement enumeration adds `PanicTurn = 9` and `MediumEffort = 10`; its requested movement-mask state is also absent from the target launch bank.



The eight ActionInputBool candidates now have an explicit audit against NHL24 Boolean assets. All three reset/deactivation flags match in all eight candidates. This does **not** verify input production or frame timing, and no aliases were installed. Three source types are absent from the recovered target registration catalog: `ActionInputBoolAsset`, `PTMWeightsTweakAsset`, and `PTMCalcUglinessParamsAsset`. Their adaptation remains unfinished. NHL27 also has a set of panic-turn reaction tuning registrations with no matching target strings, so adding asset definitions alone is insufficient.



`ant_link.py` rebuilds object/ID/name/relocation tables for fully resolved fragments, retaining original data offsets and remapping initialization indices. Loader inspection identifies outer header +28 as a dynamically growing vector reserve hint, and +32 as an optional fixed allocation arena; zero selects the existing general-allocation path. This linker has synthetic coverage but has not yet emitted an integrated controller bank or qualified cross-bank additions in the runtime.



Validation: **71 unit tests pass**. All 136 conversions and tag trees were generated from the supplied source data. No new executable patches, controller graph activation, CAS/TOC installation, or gameplay validation were completed in this pass. Remaining integration work comprises the selection graphs, state/enum registration and producers, input adapters, pose-matching differences, executable logic translation, and final package activation.



Reproduce this pass after the original source inventories and 54-move report exist:



```powershell

python scripts/nhl27/deke_closure_stage.py local/nhl27-eboot/compatibility

python scripts/nhl27/deke_new_moves.py local/nhl27-eboot/compatibility --stage-directory local/nhl27-eboot/compatibility/deke-trial/closure-stage

python scripts/nhl27/ant_tag_stage.py local/nhl27-eboot/compatibility

python scripts/nhl27/deke_input_audit.py local/nhl27-eboot/compatibility

python -m unittest discover -s scripts/nhl27 -p 'test_*.py'

```



The sections below preserve the earlier package and investigation history.



## Current state — expanded trial and unmatched-move conversion



The runnable expanded trial contains **373 DCT and 16 VBR clips (389 total)** with source playback/trim metadata. Bounded buffer repacking removed the earlier capacity blocker for these matched clips. It preserves the original blob allocation and opaque outer header; each bank gains one 16-byte relocation entry. Updated ANT resource metadata and CAS/TOC records pass disk round trips. New native object registration remains unresolved; this is distinct from repacking existing buffers.



Launch the current private trial with `scripts/nhl27/launch_expanded_deke_trial.ps1`. The older 15-clip launcher/package remains available separately. The expanded package previously reached Free Skate with a rendered skater, stick and puck. That proves scene loading, not evaluation of every modified clip or behavior parity. Gameplay testing is now explicitly delegated to the user. Temporary automation mouse bindings were removed from the private trial input file, with a backup retained.



**All 54 unmatched DCT moves now have losslessly converted resources.** Forty-eight use existing target channel maps. Six panic-turn clips share one new map: every source DOF and category was independently found in existing NHL24 animation bindings, and the native CTD fragment retains the complete source order. No missing channels are dropped. Every conversion passes exact decoded-component permutation and byte-exact inverse conversion. All 54 also have native clip/animation record fragments and symbolic binding relocations; these require bank object registration and graph linking before the runtime can use them. Twenty-one panic-turn fragments additionally retain explicit unresolved references to their embedded tag/event subtrees. They are **not included in the 389-clip runnable package**.



The reverse import trace reaches six existing target integration boundaries with no catalog dead ends:



| Existing ancestor | Unmatched clips reaching it |

| --- | ---: |

| `stateflowlocomotion_sf_primary/nodeglidefwddeke` | 15 |

| `pcdb_toedrag` | 15 |

| `pcdb_psk_ptm_priority_transition` | 15 |

| `pcdb_psk_ptm_priority_stop` | 6 |

| `pcdb_psk_lpd_bank` | 2 |

| `seq_dk18_toedrag_left` | 1 |



These are import paths, not proof of runtime selection. Eighty-nine parent/intermediate asset exports are preserved with source hashes. Direct sequence roots match a target native ID for only one unmatched clip: NHL27's RL5 toe drag replaces RL4 under `seq_dk18_toedrag_left`. The other moves need intermediate graph insertion rather than simple clip-ID replacement. Effective source patch precedence and complete forward dependency closure still need resolution before linking.



### Executable findings



The seven named `gDekeController*` registration defaults are byte-identical in NHL27 and NHL24 1.60: angle minimum 230, angle maximum 290, dead-zone magnitude 0.5, forehand angle 160, shooting-angle offset 60, forehand shooting magnitude 0.4, and forehand magnitude 0.7. These defaults need no port patch. This does not establish consumer-logic or runtime-tuning equivalence.



| Function / structure | NHL27 VA | NHL24 1.60 VA |

| --- | --- | --- |

| Controller factory | `0x3a36c0` | `0x424e60` |

| Constructor | `0x3a0fc0` | `0x4168d0` |

| Main vtable | `0xe4017b8` | `0xa426178` |

| Update wrapper | `0x3a1d00` | `0x3faa20` |

| State reset | `0x3a1270` | `0x3fb0c0` |

| Update body | `0x3a13c0` | `0x3fb1c0` |



Both factories allocate `0x1a0` bytes. The NHL27 primary vtable has 24 callable entries versus 23 in NHL24; source slot 21 is an added null-output method (`0x706e0`), followed by two time-related methods corresponding to target slots 21–22. The asset interface adjustment also changes (`-0x28` versus `-0x08` in the alternate factories). The update bodies access different asset/API layouts. These concrete ABI differences prevent direct executable-function transplantation. No executable patch has been emitted, and the semantic comparison of the update body and helpers remains unfinished.



Private artifacts under `local/nhl27-eboot/compatibility/deke-trial/`:



- `expanded-trial/package-receipt.json`: runnable 389-clip package.

- `new-moves/report.json`: 54 conversions, native fragments, channel-map requirements, integration paths and parent field exports.

- `deke-thresholds.json`: executable hashes, registration/load addresses and exact default bytes.

- `vbr/assembly-audit.json`: keyed-track reconstruction comparison; not a full engine interpolation test.



Reproduce the new work from the existing private inventories and extracted banks:



```powershell

python scripts/nhl27/deke_new_moves.py local/nhl27-eboot/compatibility

python scripts/nhl27/deke_thresholds.py local/nhl27-eboot/compatibility

python -m unittest discover -s scripts/nhl27 -p 'test_*.py'

```



Validation: all **59 synthetic/parser tests pass**. All 54 converted resource and native-fragment hashes, the new CTD fragment, and the existing runnable trial's TOC/CAS hashes were verified. These checks do not constitute gameplay validation.



Remaining implementation: native object registration, six integration graphs and their complete dependency closures, blend/tag/event translation, API/game-state adaptation, and verified executable behavior changes. The full trial package and NHL27 puck-handling parity are **not complete**.



## Historical investigation below



The following sections record earlier stages, including the original 15-clip package. Their capacity limits, test counts and gameplay observations are superseded by the current state above.



Status: partial conversion and experimental package. **The full deke/logic port is not complete. NHL27 puck-handling parity is not established.** This report supersedes the earlier animation report's unresolved direct-binding and native-object-layout findings; that earlier report remains a record of its smaller sample.



## Implemented results



The expanded source selection finds 444 resource variants whose names match `deke` or `/dk[0-9]` in the sampled Data/Patch game/globals inventories. Of these, 428 export through the bounded DCT reader, totaling 12059 stored sample keys. Sixteen hard-turn clips contain VBR animation objects and are explicitly rejected, not silently omitted from a complete-port claim. This name-based selection is not a complete behavior dependency graph or an effective patch-layer resolution.



The expanded EBX catalog scans 70480 unique candidates across those inventories. It records 193 EFIX trailing-byte errors. All direct partition imports for the 428 accepted clips resolve within the successfully parsed catalog, including generated `_af/.../toolchanneltodofassetdata` bindings missed by the earlier `af/`-only search. Nested imports, graph eligibility and runtime evaluation still need qualification.



The version-6 ANT memory-image layout is now parsed through the supplied NHL24 base and 1.60 game and launch banks. Evidence includes the 1.60 loader at `0xad74b0`, complete section consumption to EOF, valid object/type/name bounds, and relocation and external-reference bounds. The 96-byte little-endian image header is followed by type sizes, type hashes, relocations, objects, external references, initialization indices, name offsets and the data image. Launch banks additionally contain opaque external tables in the version-3 outer header; these are preserved, not interpreted as object identifiers.



All 54 selected source clip-root IDs present in the two target game banks resolve to the observed 72-byte ClipController and 104-byte DCT native types and decode successfully. Their channel-to-DOF IDs all differ from the source IDs. The referenced target maps reside in the **launch bank**, not the game bank.



For all 54 matched clips, source and target channel maps have compatible unique DOF sets within quaternion/vector categories and identical scalar ordering. The converter reorders the original compressed coefficient streams into target channel order without requantizing. Each identity repack is byte-exact; every decoded reordered component equals the corresponding source component under the same decoder. This does not prove equality with engine SIMD rounding or interpolation.



## Trial contents and exclusions



Twenty-five of the 54 converted clips fit existing target key/body allocations. Ten of those differ in controller timing or playback flags and are excluded by the conservative trial builder. The other 29 require allocation growth. **Fifteen motion replacements** are included in each private native-bank candidate, with existing graph logic, identities, references and timing metadata retained.



Included names cover one missing forehand-to-forehand clip, slide-by right, two right-pass-deke variants, one Subban big-stride variant and ten windmill variants. Exact asset names and write ranges are in `bank-trial-report.json`. These are existing move identities; their replacement does not add NHL27's new moves or port input decisions.



The builder verifies matching controller scalar metadata, animation trim/end-frame metadata, native channel counts/cycle/base flags, allocation capacity, absence of other relocation references into replaced buffers, and non-overlapping writes. Reopening each entire output bank reproduces every included converted resource byte-for-byte. Reversing only the declared write ranges recovers the original bank exactly.



NHL24 1.60 bank SHA-256:



- Original: `cc638f146bf85bed18cb9d945b7a567e8c37cd25482776fd73952efa316652b9`

- Motion trial: `63096668e41b0c38cd5c7184b78d6e3370412818317ee69347a6dfc5f2661793`



The private package copies the affected patch CAS, appends uncompressed CAS frames, updates the existing resource's manifest SHA-1 and its TOC offset/size pair, then rereads and validates the emitted files. Its CAS grows from 211587993 to 305592958 bytes. Native-bank length, resource ID and manifest decoded length remain unchanged. The resource is not injected into the eboot.



## Runtime and isolation



The user's `F:\Games\NHL24` answer resolves in this installation to `F:\Games\NHL 24`; the no-space directory does not exist. The runtime is `F:\Games\NHL 24\nhl24.exe`, SHA-256 `0da12e71bf4f0acea34e758f46b849ecd7f0b90a3e3570ddb1dc71dd5bfc01ce`. Its log reports revision `7ce4ce8317b61583798cd2228e098ecbee8bdc0b`, description `7ce4ce83-dirty`.



The private trial root is `local/nhl27-eboot/compatibility/deke-trial/trial-package/`. Its `game` and `game-UPDATE` junctions point to the installed base/update; `game-mods` contains only the replacement files. The runtime's `/app0` and `/hostapp` mounts are read-only. The trial has its own `user` directory, configuration, patch profile and saves; existing user saves were not copied. Installed game files and eboots were not edited. Do not recursively delete through these junctions.



The startup probe recognizes NHL24 **01.60** and reports files in the mods folder. Both trial and unmodified baseline remain alive over separate bounded 45-second observations and are then stopped by the probe. Both logs contain nine C++ exception reports; process survival alone is not a clean boot qualification, and matching counts do not establish identical execution. The baseline correctly reports no mods folder. No gameplay input, visual animation evaluation, or proof that the modified game bank was read has been obtained. Raw logs, runtime receipts and `runtime-comparison.json` remain private alongside the package.



To launch the experimental package with the audited executable and private configuration:



```powershell

& 'E:\Repositories\nhl-modding-studio\scripts\nhl27\launch_deke_trial.ps1'

```



The launcher verifies the executable, TOC and CAS hashes. The trial requires the prepared private package and junctions. It is not a distributable mod or a validated NHL27 gameplay port.



## Reproduction and remaining work



```powershell

$py = 'C:\Users\puckh\AppData\Local\Programs\Python\Python312\python.exe'

$root = 'local/nhl27-eboot/compatibility'

& $py scripts/nhl27/animation_stage.py $root --all-dekes --scan-all-af

& $py scripts/nhl27/deke_native_audit.py $root

& $py scripts/nhl27/deke_reorder_stage.py $root

& $py scripts/nhl27/deke_bank_trial.py $root

& $py scripts/nhl27/deke_trial_package.py $root

& $py -m unittest discover -s scripts/nhl27 -p 'test_*.py'

```



These commands require the existing private inventories, recovered schemas, game banks and launch banks. They do not recreate an installation or choose effective source patch precedence. The full export catalog and original binary payloads stay under ignored `local/`. All 39 synthetic/parser tests pass, including DCT block-boundary permutation/inverse tests, malformed native-bank bounds and CAS framing boundaries. Real-input bank/package round trips also pass; those are distinct from runtime gameplay validation.



The remaining full-port dependencies are: complete VBR track binding and assembly; effective-layer and transitive graph closure; allocating new native objects/data with all loader metadata understood; converting new native types and controller graphs; input/eligibility/state transitions; puck/stick synchronization and event behavior; executable-side behavioral differences; and repeatable in-game comparisons against the source title. Clips absent from the two target game banks are not proven absent from every NHL24 bank.



## Extended launch-bank and VBR investigation



The later launch-bank search substantially expands coverage beyond the initial game-bank trial. NHL24 base has 313 additional DCT clip conversions; 1.60 has 319. Each conversion passes byte-exact identity repacking and exact decoded-component permutation checks. One additional name-selected root, `fc_nhl15_deke_v006`, is not a qualified DCT clip and is rejected; name matches must not be reported as converted animations. The 1.60 game and launch results therefore contain 373 converted DCT resource occurrences, before effective-layer deduplication.



Of the 319 launch conversions in 1.60, 222 fit existing buffers. The conservative builder includes 55: 146 otherwise-fitting clips have different trim/end-frame metadata, and 21 have different controller timing/flags. The remaining 98 rejected rows include 97 clips requiring larger buffers and the non-clip root. The initial 15-clip packaged overlay remains unchanged. The extra launch-bank candidates are decoded research outputs, not installed or gameplay-validated packages.



`deke_bank_trial.py` now accepts an explicit `--port-metadata` candidate mode. It transfers source controller scalars, trim/end-frame values, base flags and cycle alongside verified reordered motion, while requiring equal channel dimensions, matching DOF-set identity, exclusive buffers and sufficient existing capacity. It preserves a separate output from the conservative candidate. It does not transfer state-machine logic. Both modes reopen emitted banks, compare reconstructed resources and verify that reversing declared writes restores the original file exactly.



That metadata candidate successfully includes 222 launch-bank and 25 game-bank occurrences for 1.60 (247 total), versus 218 and 25 for base. The 1.60 launch output SHA-256 is `0fc80b1a8cfeae9419af07057829c6489544d7c143411b869fa2b144ee0c5d11`; the game output is `428901d6e9548204db191e4c03bb8a707e3ab547f6384eaf83c46b59bbb5a16f`. These separate decoded banks have not replaced the packaged 15-clip trial.



All 16 VBR roots exist in NHL24's launch bank. The envelope, constant palette indices, channel run map, width descriptors and compressed block boundaries have been parsed. The portable coefficient reader/writer reproduces every source block byte-for-byte. Isolated, hash-bound native helpers from both supplied executable builds independently produce identical coefficient buffers and bit-identical inverse-transform output for every block in all 16 clips. These are dynamic-block results, not fully assembled, bound and interpolated player motion.



The audited helper pairs are source `0x64a6c0`/`0x64ac20` and NHL24 1.60 `0x851160`/`0x8516e0`. The oracle maps only these helpers and referenced constants into a disposable process; it does not run a game entry point. Its output guards remain intact. `vbr_coefficients.py` needs no executable mapping. Actual scalar-palette reconstruction, key-time interpolation, quaternion normalization, DOF remapping and runtime synchronization still require qualification. Six VBR source payload sets fit their original palette/key/stream capacities; seven clips have different dynamic/constant channel dimensions. Those differences prohibit treating matching root IDs as direct replacement permission.



The source deke-controller import closure contains 99 partitions with no missing imports in the scanned catalog. Named-field export succeeds for all 99, yielding 138 exported instances; 130 IDs match the inspected target banks. The eight unmatched IDs represent one API game-state asset, four blend curves and three channel maps. All 12 matched `DekeCoordAsset` records have identical stick magnitude, angle, mapping and node values. This narrows the behavioral investigation to their surrounding graph and executable consumers; it does not establish equivalent deke logic. A signed relative-pointer bug in the EBX field reader was fixed and covered by regression tests.



Allocation growth remains unresolved at the outer bank header: the version-6 memory-image sections, relocations and object tables are parsed, but the semantics of outer big-endian fields at offsets 28 and 32 have not been established. No growth writer should guess those fields or treat a parser round trip as proof the game loader accepts the result.



Reproduce the extended checks with:



```powershell

& $py scripts/nhl27/vbr_decode_audit.py $root

& $py scripts/nhl27/deke_native_audit.py $root launch

& $py scripts/nhl27/deke_reorder_stage.py $root launch

& $py scripts/nhl27/deke_bank_trial.py $root launch

& $py scripts/nhl27/deke_bank_trial.py $root launch --port-metadata

& $py scripts/nhl27/deke_bank_trial.py $root state --port-metadata

& $py -m unittest discover -s scripts/nhl27 -p 'test_*.py'

```



The extended synthetic/parser suite has 45 passing tests. Private receipts are `vbr/decode-audit.json`, `vbr/native-capacities.json`, `launch-native-comparison.json`, `launch-reorder-report.json`, `launch-bank-trial-report.json`, `controller-fields/report.json` and `controller-coordinates.json` under the existing deke-trial directory. Full-port completion and NHL27 gameplay parity remain unverified.



## Ongoing runtime integration: producer and scheduling corrections



The current private EBOOT candidate adds a generic Boolean-writer observer at

`0x8a7fae`, after native property initialization and before the final store. It

compares the asset against the eight GSBool asset references, then checks the

resolved context backing pointer. This covers asset/device writes, including

action 172, separately from direct native producer stores. The original store

still executes. Its trampoline preserves all YMM registers because the binding

getter can call game code. The isolated destructive-callee fixture checks guest

argument mapping, GPRs, flags, stack restoration and all 256 vector bits.



Direct producer hooks now also support a store-site hook after an extra native

predicate, and runtime class guards where the compiler spills the record. The

guard checks vtable `base+0xa426580`; the sole observed constructor reference is

`0x3ffc8d`, within the allocation path already extended to 0x2190 bytes. Failed

class guards leave action storage untouched and execute the original code.

These are candidate integrations, not evidence of source producer parity.

Latest counts and image hashes are in `runtime-build/input-producer-eboot-build.json`.



`scene_schedule_audit.py` reproduces the supplied source/target scene comparison

with patch-layer precedence and payload hashes. The 21 entity columns have

identical identities. The matrices contain 56 versus 53 operations, seven

source-only and four target-only entries, and eleven changed column masks.

`ActionInputUpdateSceneOpAsset` is registered in the source executable but is

absent from this supplied source matrix; registration alone does not establish

an active snapshot phase.



Source human snapshots occur in `ActionInputCallbackImpl`, vtable `0xe81cff0`

slot +0x10 (`0x4b48580`), registered by `0x4b2d300`. The corresponding target

initialization has no such callback registration. Target `0x314eb70` is a

special-actor producer gated by global `0xba474e0` and a negative actor +0x23c;

it must not be used as the human snapshot callback. Target team actor arrays

are `0xba3c2c0` and `0xba3c360`; `0xba5d7e0` is the separate all-player lookup.

Snapshot scheduling, source controller-pool ownership, panic reactions and PTM

runtime differences remain unresolved. No full-port installation is active.

# Controller scheduling and action readers — continued implementation



The private candidate now has a 12-entry shared controller pool in an extension

of the existing writable ELF segment. Code remains RX. The binding allocation is

`0x21a0`: local action table at `+0x1580`, selector at `+0x2190` (zero means local,

1–12 selects controller 0–11). Controller history survives player switching;

assignment always resets local history, and the optional selected-table reset

uses source `gResetInputManagerOnLink`'s false default. Initialization is guarded

with acquire/release atomics. Pool ownership/reset coverage remains incomplete.



Source input-system vtable `0xe8a1ec0 + 0x1c0` points to `0x69a2da0`, which stores

ActionInputCallbackImpl at input-system `+0x60`. Source `0x69a1808` invokes that

callback immediately after raw sample update `0x69a87e0`. Target `0x5000810`

unrolls six controller slots per team. Six post-call hooks around `0x5003d60`

recover all 12 indices from the team flag at `[rbp-0x70]` and unrolled slot. They

snapshot shared history and set its raw-input owner. The native controller array

is `0xba479a0`, stride `0xf0`. Local snapshots now consult the input selector,

matching the source feature index, rather than assuming actor.controller equals

the feature's selection.



Assignment hook `0x2a2e3d9` selects the pool after the native assignment/reset.

Unassignment hook `0x2a2cc6e` and its four inlined copies clear selection and reset

local history. Mapped native bytes are synchronized when ownership changes.

Primary getter `0x8a7e40` and Boolean-interface getter `0x8a7e90` read the selected

action table for the eight mapped assets; unrelated assets execute their original

getters. The secondary interface is adjusted by 64 bytes. Transition writes still

pass through the generic writer hook.



Executable machine-state fixtures cover all 12 sample indices, both reader

interfaces, native fallback, non-normalized byte values, assignment/clear order,

registers, flags, full YMM state, and stack preservation. ELF verification checks

RW BSS growth without changing original file-backed data or creating RWX code.

The latest hash and declared patches are in `runtime-build/input-producer-eboot-build.json`.

This is still a private candidate: source producer parity, remaining reset paths,

panic/PTM behavior, complete graph integration, installation and activation are

not complete. The new CFG pointer-flow inventory is being used to find stores

missed by the earlier local basic-block scanner.
