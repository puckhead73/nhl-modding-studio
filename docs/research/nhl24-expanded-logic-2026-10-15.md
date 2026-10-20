# NHL 24: appearance, sliders, statistics and mode expansion

Follow-up to [executable research](nhl24-eboot-2026-10-15.md). The accepted league
ceiling is now **48**; 49/50-team expansion has been removed from the implementation
scope. All eight additional requests below are part of the research scope.

This document records static findings, not completed gameplay modifications. All
executable addresses in this document refer to the **01.60 image**, SHA-256
`fe5dcab197256487dd47906a4b0e0601bfe77fac9f47ad3f4f91b03b9372deb9`.
The new asset inventory reads the **base installation**; it does not resolve patch
override precedence. No game, patch, roster or save was modified or launched.

## 1. Player head meshes and portraits

There are separate presentation, geometry, texture and variant fields. A single
player ID is not sufficient to specify all of them.

At `0x035e8150`, the presentation object is built from an unpacked player structure:

| Output | Source in this structure | Observed operation |
|---|---|---|
| `ArtId` | unsigned 16-bit `+0x94` | Copy |
| `HasPortrait` | byte `+0x141` | Publish boolean |
| `PortraitId` | same art ID | Use art ID if flag is nonzero, otherwise `65535` |
| `HeadId` | unsigned 16-bit `+0xc8` | Copy separately from portrait |

These offsets belong to the traced runtime structure, **not offsets in a packed
database file**. They must not be used to hex-edit a TDB record.

Additional configuration bindings are explicit in the update settings bridge:

| Name | Setting ID | Name-reference instruction |
|---|---|---|
| `Player_Art` | `0x7d0` | `0x04f5e0b8` |
| `Player_FEPortrait` | `0x811` | `0x04f6233e` |
| `Player_HeadGeometryId` | `0x829` | `0x04f63bce` |
| `Player_HeadTextureId` | `0x82a` | `0x04f63dda` |
| `Player_HeadType` | `0x82b` | `0x04f63ee0` |
| `Player_HeadVariant` | `0x82c` | `0x04f63cd4`, `0x04f63fe6` |

Creation-zone output at `0x033c12d0` separately exposes `HeadGeometryId`,
`HeadTextureId`, `HeadType` and associated appearance fields. One stock-player
branch reads geometry as a signed 16-bit value at `+0xd0` and texture as an unsigned
16-bit value at `+0xd2`. Serializer `0x04c6cf90` publishes those as `headid` and
`headtexture`. Thus the presentation `HeadId` at `+0xc8` must not be assumed to be
the geometry field at `+0xd0`. The creation-zone selection path at `0x08068160`
passes `Player_HeadGeometryId` into its setting operation at `0x08068f54`.

The base Frostbite manifests contain actual resource families, for example:

* `content/character/skater/shared/head/licensed/aaron_ekblad/components/head_aaron_ekblad`
* The corresponding `_mesh`, helmet-placement, bundle and mesh-variation resources.
* `content/character/skater/shared/head/generic/female_30100/components/head_female_30100`
* `ui/textures/common/playerportraits/p9` and separate `playerportraitstoast` assets.
* Portrait placeholders `p0` and `p65535`.

The executable also formats `nhl/ui/assets/portraits/p%d.png` at `0x08988e84`
and `0x089892ce`. The latter is within a stream-writing path; this string alone is
not proof that loose PNG injection is the renderer's supported import route.

**Practical integration direction:** assign geometry/texture/type/variant through
the player appearance data and portrait/art fields independently. Register new head
resources and their dependencies in the asset lookup/bundle system. A new filename
or a new numeric player field alone does not establish that the asset resolves.

**Still missing:** the precise numeric-geometry-ID → Frostbite resource reference
lookup, its insertion/registration format, patch-layer precedence, and a roundtrip
test that displays a newly added head on a selected player. This is not yet a
complete new-head import recipe. Mesh conversion itself remains the separate
headmesh workstream.

## 2. Gameplay sliders and player attributes

The requested final reference must document **each UI slider and each attribute's
consumers**, not just restate its label. The current output is a partial code-backed
reference plus an inventory, not the complete dictionary.

The supplied catalogue contains 82 curve records with `Slider` in the name. There
are 74 unique storage matches in 01.60; other entries remain unresolved/ambiguous.
Duplicate names remain separate. Private `slider-evidence.json` and `.csv` preserve
their registrations, arrays, measured values and candidate runtime-slot references.
This count is not the number of UI sliders: one UI slider can affect several curves,
and many sliders are not represented by these named arrays.

### Recovered interpolation

The pass-reception preparation path `0x02ed6910` selects a setting according to the
sign bit of the player's control byte at `+0x23c`, reads its value, and calculates:

`x = setting_value * (number_of_curve_points - 1) / (setting_range_count - 1)`.

It clamps to the first/last point outside the interval, otherwise interpolates
linearly between adjacent points. The eight reception components examined have
setting-range count **101**, so this path maps values 0–100 across the curve.
Seven-point curves therefore place their knots at 0, 16.67, 33.33, 50, 66.67,
83.33 and 100; they are not seven discrete slider positions. The corresponding
setting-ID pairs are `(89,90)`, `(85,86)`, `(85,86)`, `(83,84)`, `(87,88)`,
`(83,84)`, `(79,80)`, `(83,84)`. These are internal slider IDs, not the general settings IDs above.
The exact human/CPU orientation of each pair still needs an end-to-end UI check.

### Selected measured curves and meaning

| Control | Curve points in increasing input order | What is established |
|---|---|---|
| Pass accuracy | 40, 40, 30, 20, 10, 5, 0 | Human/CPU curves feed a multiplicative term in the pass calculation at `0x02bfcf26`. A larger slider reduces this contribution; it is not a success percentage. Complete downstream error construction remains to be annotated. |
| Wrist-shot accuracy (`ShotAccuracySlider`) | 4, 2, 1.3, .8, .6, .3, .01 | Consumer `0x02a090b0` interpolates the curve with denominator 100. The parameter falls sharply as input increases. It does not directly encode shooting percentage. |
| Acceleration | .5, .6, .7, .85, 1.033, 1.2, 1.5 | Consumer `0x02eef280` interpolates and stores the result in its skating preparation structure at `+0x138`; it is a separate parameter from top speed. |
| Skating speed | .85, .92, .98, 1.07, 1.10, 1.15, 1.20 | Separate curve/consumer from acceleration; a neutral 1.0 is not the midpoint knot. Full physical-speed formula remains open. |
| Poke-check power | .5, .7, .8, 1, 1.1, 1.3, 2 | `0x02b10cf0` multiplies a contact-response magnitude by the interpolated value, then caps it and applies a vector scale. It is not an AI pokecheck-attempt-frequency control. |
| Puck control | 20, 2, .6, .4, .35, .3, 0 | Strongly descending curve, used by `0x02c5dc90`; do not describe it as a linear possession bonus without completing that consumer trace. |
| Reception reaction | 1.9, 1.6, 1.3, 1, .7, .4, .3 | Interpolated reception component in `0x02ed6910`; larger input reduces this component. Units and downstream use are not yet qualified. |
| Reception puck-speed component | .1, .4, .7, 1, 1.3, 1.6, 1.9 | Increases while the reaction component decreases. Reception sliders do not all operate in the same direction internally. |
| Reception hands component | 1.09, 1.06, 1.03, 1, .97, .94, .91 | Narrow descending adjustment in the same reception preparation path. |
| Bench energy recovery | .001, .025, .04, .045, .065, .075, .125 | Separate normal and Be A Pro curves; consumer `0x02843110`. A common UI label does not guarantee identical behavior in both modes. |

These are storage/consumer observations. Effects on measured match outcomes have
not been tested. Some remaining named curves have only registration references;
their existence does not prove an active gameplay consumer.

For attributes, the earlier report's pressure multiplier and 36–99 final clamp
remain verified only for the traced getter. Attribute selector curves, X-Factors,
fatigue, mode modifiers and simulation ratings must be traced independently.
**A complete attribute-ID/name/formula map has not yet been recovered.** In
particular, pressure-sensitive attribute IDs 8 and 15 are not assigned names here.

The final dictionary requires, per control: exact UI label and ID, human/CPU and
mode routing, normalization, curve/formula, downstream consumers, interactions,
units, clamp behavior, edge cases, and an evidence-backed plain-language effect.

## 3. Offline ONES

Offline-facing mode names `ones_now` and `h2h/ones_now` are referenced at
`0x025813ce` and `0x025ab82e`. Keep these separate from the numerous online
`ResortEliminatorOnes` paths, which use Blaze IDs and online progression.

Dedicated ONES CPU decision controls are registered at:

| Control | Registration name reference |
|---|---|
| `gOnesCPU_DekeExecBias` | `0x031792d6` |
| `gOnesCPU_DumpExecBias` | `0x0317938d` |
| `gOnesCPU_ShotExecBias` | `0x03179444` |
| `gOnesCPU_SkateExecBias` | `0x031794fb` |
| `gOnesCPU_PassExecBias` | `0x031795b2` |
| `gOnesCPU_BoardPlayBias` | `0x03179669` |

These are behavior-tuning leads, **not controls for participant count**. Disabling
AI decision execution would leave an inactive participant and does not establish
that the match can operate with fewer players.

Still required: the offline selectable-player source/filter, player-slot creation,
human/CPU assignment, goalie slot, score ownership, tie/overtime rules and camera
assumptions. Treat expanding selectable players, removing one CPU opponent for
1v1, removing both for practice, and adding simultaneous participants as separate
changes. None is yet enabled.

## 4. Hit totals

Two explicit hit-qualification tweakables have been traced into contact handling:

| Control | Default | Registration object | Runtime pointer slot |
|---|---:|---|---|
| `gTimeAfterReleaseForHitToCount` | 40 | `0x0b94c808` | `0x0b94c850` |
| `gProximityToPuckForHitToCount` | 10 | `0x0b94c858` | `0x0b94c8a0` |

Initializer references: `0x02b099fe` and `0x02b09ab7`. The first is an integer
default with declared maximum 200. The second is a float with declared maximum 50.
Consumers read runtime value `+0x34`; registration defaults are not live values.

Contact handler `0x02ad4d70` contains qualification branches at `0x02ad58fd` and
`0x02ad71bd`. They consider whether the checked player is the tracked previous
possessor and elapsed counter difference is **less than** the time threshold,
whether it is the current tracked possessor, or whether a proximity metric is
within `distance² * 0.09290304035`. That factor is `0.3048²`, consistent with a
feet-to-metres squared conversion. The actual default proximity threshold is
therefore 9.290304 in the squared metric, not a raw comparison against 10.

Additional global state checks and contact flags affect a byte at contact-object
`+0xa0`; the same function emits `GameEvent::InitialCheck` and `GameEvent::Check`.
This establishes a puck/time-related qualification stage, not the whole published
hit-total formula. The units of the elapsed counter have not been demonstrated;
**40 must not be described as 40 seconds or converted to seconds yet**.

Outstanding: trace this eligibility flag through the event consumer to the actual
team/player hit counters; identify duplicate-contact suppression, reverse hits,
teammate contacts, late contacts and post-whistle handling. Overlay threshold
`gMinHitsToShowStat` is presentation logic and is not the hit-credit threshold.

## 5. Takeaways and giveaways

Handler `0x0299c050` performs several checks before a paired counter update and a
`GameEvent::Takeaway` emission. It rejects selected game/event states, missing
participants, and same-team transfers in the conventional team case. A separate
mode-dependent branch compares player identity instead of team identity.

It queries recent event history and can apply a zone test. Runtime switches are:

* `gTurnoversInDefZone`: default true; pointer slot `0x0b905e88`.
* `gTurnoverDoTurnoverCheck`: default true; pointer slot `0x0b905ed0`.

The latter calls predicate `0x02d61bb0`. That predicate examines previous/current
possessors, team/player identity, a possession-state code, control-duration tuning
and earlier events. It can reject a possession change before counting.

Accepted paths call `0x029926b0` with codes `0x27` and `0x26` for the two players,
then increment two additional stat-storage counters and emit the takeaway event.
`0x029926b0` is demonstrably an increment helper: it increments a per-player
integer at `object + 0x29c480 + player_slot*700 + code*4`, and, when a controller
mapping exists, another controller-indexed integer at `+0x2a31e0 + controller*700
+ code*4`. It rejects player slots above 39. **That is a match-slot check, not a
40-player database maximum.**

The association of those event codes with the final displayed stat fields still
needs confirmation through the storage schema. Do not assume every giveaway must
have a matching takeaway: this is one paired handler, not proof that no other
giveaway-only path exists. The nearby `0x0299bb20` also applies coaching-feedback
rewards/penalties; those values are not the displayed giveaway totals.

## 6. Season/Franchise simulation

There is separate calendar/game/shift simulation machinery and a dedicated
`gamemodes/league/resource_kernel/simengine.brk` resource name. The loader references
`simengine.brk` at `0x025b3cb4`; another path names
`gamemodes/database/noa/nhl_gm_simulation.data` at `0x025b34ee`.

Useful executable entry points include:

* `0x081854a1`: allocation/ownership label `FeLeagueManager::mFeSimulation`.
* `0x0819ee3e`: `FeSim::FeGameInfo` allocation label.
* `0x089411d0`: SimPlayer initialization neighborhood; named rating-modifier lists
  referenced at `0x08945868` and `0x08945a60`.
* `0x08979f80`: SimEngine initialization, including `FatigueStatScale`'s
  `TimeRestedScalePercents` container at `0x0897a15e`.
* `0x0499da8e`: named game-simulation service; adjacent services distinguish chunks,
  dependent games, out-of-town games, cleanup and simulation speed.
* `Setting_Sim_ShotGeneration`, `Setting_Sim_DraftStrength` and
  `Setting_Sim_RookiePotentialGenerator`: distinct settings names.

The resource names were not found as direct named assets in the filtered base CAS
manifest inventory. That does not establish absence: they may be packed inside
another resource or the initialization filesystem. The existing initfs reader
preserves opaque encrypted contents without claiming to decode them.

No scoring/shot/penalty probability formula or qualified simulation edit is
established yet. Live-match skating/accuracy sliders must not be advertised as
calendar-simulation controls until a consumer relationship is demonstrated.
Required validation uses repeated fixed-seed or distribution-controlled seasons,
not one favorable result: goals, shots, saves, penalties, ice time, injuries,
attribute sensitivity and player/team ranking need separate checks.

## 7. Entity and asset capacities

There is no established single global maximum for the seven requested categories.
Track these distinct limits per build: packed field range, database table capacity,
runtime container capacity, save serialization capacity, resource registration,
frontend enumeration and mode-specific limits.

| Category | Established | Not established |
|---|---|---|
| Players | Some match-stat paths address slots 0–39 | Global database/save population ceiling |
| Jerseys | Asset families and database relationships require independent capacity checks | Maximum jersey identities or variants |
| Equipment | No global ceiling recovered | Per-class ID width, lookup capacities and aggregate limit |
| Teams | 48 entries in the custom competition block | Total teams across the game/database |
| Leagues | Creator has six observed tier blocks | Total league registrations and simultaneously simulated leagues |
| Player looks/head geometry | A serializer reads geometry as signed 16-bit; presentation head ID is a different unsigned 16-bit field | Number of valid/addable head definitions; valid sentinel policy and resource capacity |
| Portrait/art identities | This presentation path uses 16-bit art IDs and portrait sentinel 65535 | Maximum addable portrait resources or a universal portrait limit |

Do not copy NHL 21 table capacities into NHL 24. Likewise, an ID width is not a
tested population limit. Counts of manifest entries include duplicate dependencies,
variants and unrelated resources and must not become user-facing capacity claims.

## 8. All-Star game and season-performance selection

Dormant hooks exist: `DYNASTY_ALLSTARGAME` is registered in the mode-name table at
`0x025f5cbf`, `GAMETYPE_ALLSTAR` is referenced at `0x07e076eb`, and
`TodayIsAllstarGame` occurs in the event/popup handler at `0x081e6b34`.
`AllStarGames`, `IncludeAllStarTeams` and a selected-as-All-Stars news token also
exist. These are stronger leads than a completely new mode, but are not proof of
a complete hidden scheduling and roster-selection implementation.

Proposed implementation, not recovered behavior: create an exhibition-like event
on the mode calendar with transient rosters selected from that save's season stats.
Keep original team membership intact and keep event stats out of regular standings
and season totals. Selection needs position quotas, conference/division assignment,
injury availability, minimum games/ice time, goalie sample thresholds, tie breaks,
and an optional minimum-one-player-per-team rule. Persist selection once made so
save/reload does not silently reroll it. Preserve/restore fatigue and injury policy
explicitly, and skip or reshape the event for small historical leagues.

**Possibility assessment:** plausible with code and save/calendar changes; no
verified toggle or working automatic season-performance selection has been found.
Trace the dormant event's producer and consumers before designing a replacement.

## Evidence and reproducibility

Private evidence stays under `local/nhl24-eboot`. New outputs include
`expanded-leads.json`, selected annotated assembly, additional Ghidra C output,
`resource-inventory.jsonl`, and `slider-evidence.json/.csv`.

The read-only resource inventory completed successfully against the base install:
191,750 filtered occurrence rows, 109,869 distinct `(kind,name)` pairs. The broad
head/portrait filter includes crowd/UI/other assets; these are **not player-head
counts**. The script uses the repository's existing Frostbite readers and validates
manifest/span counts. It does not export meshes or decode EBX semantics.

Utilities: [resource_inventory.rs](../../scripts/nhl24/resource_inventory.rs) and
[build_slider_evidence.py](../../scripts/nhl24/build_slider_evidence.py). The private
standalone Cargo manifest is `local/nhl24-eboot/resource-probe/Cargo.toml`, whose
dependencies point to `crates/tdb-frostbite` and `serde_json`.

```powershell
cargo run --offline --manifest-path local/nhl24-eboot/resource-probe/Cargo.toml -- 'F:\Games\NHL 24\game'
python scripts/nhl24/build_slider_evidence.py local/nhl24-eboot
```

Before Studio exposes edits, require explicit evidence levels for every entry:
located → consumer traced → runtime observed → effect verified → editing qualified.
The present expanded findings are at the first two levels. No complete slider/
attribute encyclopedia, ONES participant patch, simulation overhaul, new-head
registration workflow, global capacity table or All-Star implementation is claimed.
