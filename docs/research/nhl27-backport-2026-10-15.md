# NHL 27 → NHL 24 research dossier

Research date: 2026-10-15. Scope: supplied PS5 NHL 27 executable and install, compared with NHL 24 PS4 base and 1.60. This is an executable/asset research implementation, not a completed gameplay backport. No installed executable or save was changed. No game was launched for validation. Unresolved items below are not editor capabilities.

Follow-up: [animations, presentation and play-by-play compatibility investigation](nhl27-port-compatibility-2026-10-15.md). It supersedes the original 0x80 span-reader limitation below and adds decoded payload, schema and guarded conversion evidence. It does not qualify a working feature port.

## Identity and reproducibility

Source: `W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin`, 280842080 bytes, SHA-256 `10c7bee4c27e5260b417f9574e65cee8338019d08635b3ab9dddab30ba524126`. Install metadata: PPSA34063, `UP0006-PPSA34063_00-NHLPS5HOCKEY2027`, version 01.003.001, master 01.00. The supplied metadata identifies the title; folder naming alone was not used.

NHL 24 base SHA-256: `0118720c7e0338e20769063437be5656d1c16262c6b81a688adb659582544e17`; 1.60: `fe5dcab197256487dd47906a4b0e0601bfe77fac9f47ad3f4f91b03b9372deb9`. Prior work: [executable research](nhl24-eboot-2026-10-15.md), [expanded logic](nhl24-expanded-logic-2026-10-15.md).

Source is little-endian x86-64 ELF with 14 program headers and five PT_LOAD segments. Its section-table offset lies beyond EOF. The analysis copy clears only the invalid section-table header fields; loadable bytes are unchanged. Analysis SHA-256: `f9baeb9ae9c3493c5d2e3b5149623857087dfeef81d20c01114c4fdad897932a`. All addresses below are **NHL 27 image virtual addresses**, not file offsets, runtime addresses, or NHL 24 patch addresses.

The index contains 257279 ASCII strings, 1799595 candidate RIP-relative references and 620924 unwind starts. These are discovery aids: candidate references and preceding unwind starts are not proof of a control-flow relationship. Ghidra was used for targeted decompilation, not a claim of complete autoanalysis. Its unresolved imports, type propagation, jump-table and unreachable-code warnings constrain interpretation. REA 6.5.0's objdump integration independently inspected the analysis ELF. Native slices were checked against exact source bytes before execution.

Dynamic RELA has 1126486 records, including 1118529 relative relocations, 7929 type-1 and 28 type-6 relocations. Zero pointer bytes on disk frequently have relocation addends. `relocations.py` resolves relative relocations and leaves other kinds unresolved.

## Priority 1: team strategies

The generated CSV constructor at `0x6601d90` consumes `strategyplaybook`. Relative relocations recover its column names: `playbookartid`, `playbookdescription`, `playbookid`, `playbookname`, `playbookshortdescription`, `playbooktype`. Adjacent registration data names `strategysliders`, with `strategyid`, `assetid`, `description`, `leftlabel`, `maxvalue`, `minvalue`, `numvalues`. Names `playbook1` through higher-numbered columns are schema evidence, not a proved supported-playbook count.

Traced service functions:

| VA | Role |
|---|---|
| `0x5460690` | Playbook list, including `PlaybookIdList` |
| `0x5460d60` | Default playbook values |
| `0x5461690` | All-playbook structure list |
| `0x5461900` | Strategy structure response |
| `0x6976750` | Strategy structure initialization |

NHL 27 has `nhl-data:strategy:general:playbook:list:get`, `default:playbook:values:get` and `all-playbook-structure:list:get` service names (version suffix 01.00.00) absent from both NHL 24 indexes. NHL 24 instead includes `info:by-strategy-category-id:get`. This supports a changed data/service surface; it does not establish that copying CSV rows will implement new AI behavior.

Backport unit: schema + playbook ID mapping + frontend/service adapter + tactical consumers + save representation. First expose a read-only playbook/schema comparison. Then trace one shared tactic end-to-end in both builds before introducing a new tactic. Required validation: selection, in-game formation/decision response, per-team persistence, human/CPU teams and round-trip saves.

New breakout, forecheck, carry/dump, cycle/shoot and poke-related tuner names are indexed as leads. Their labels do not establish gameplay formulas. Conservative matching of NHL 24's 21666 catalog entries found zero unique normalized instruction matches: 18568 had no match, 2278 lacked candidate label references, 820 were curves requiring separate mapping. This method cannot port their addresses automatically; it does not prove manual ports impossible.

## Priority 2: Franchise frontend and functionality

Seven `nhl-data:franchise:conversation:` and four `nhl-data:contracts:player-clause:` service names are absent from both NHL 24 indexes. Evidence spans conversation lists, demands, promises, negotiation info/table operations, revealed information and per-player/per-year clauses.

`0x4f5bb50` reads PlayerId/NegotiationContext and returns conversation and entity display information. `0x4f5cad0` reads PlayerId/TableLength and routes through `0x3af2b90`, `0x3e4fba0`, `0x3e48300` to the state writer `0x3e48310`. The slice `0x3e483ce..0x3e483de` clamps a signed input to 0..255. **Its units are not established; do not label this “255 days.”** The isolated clamp passed 3075 inputs, including signed extrema. This does not validate the entire negotiation state transition.

`0x4ddcaf0` constructs clause services; `0x4ddde70` reads PlayerIdList and produces PlayerClausesList, CurrentYear, ClausesInfo and AllowedTeamsInfoList. Clauses therefore require state and roster/contract integration, not only a new panel.

Frontend bundle names include `ui/ecs/features/connectedfranchise/...` hubs, alignment, league creation, roster, settings and activity feeds. Connected-Franchise assets are inventoried separately from offline Franchise feasibility. Their presence does not establish usable backend services or an offline equivalent.

Backport unit: NHL 24-native frontend bindings + conversation/context handling + contract/clause state + AI decisions + versioned save conversion. No raw frontend bundle transplant is qualified. Begin with read-only contract/conversation inspectors, then one reversible new-save-only feature.

## Priority 3: PWHL and Jailbreak

Two distinct namespaces are confirmed. The league display switch `0x3d3b1c0` maps case `0x59` to `TXT_PWHL`. Gameplay predicate `0x470a1f0` checks classification **0x1b** in either of two team rows. They are not interchangeable IDs. The predicate's inner slice passed 735 synthetic cases across row strides and base offsets; this validates the predicate only.

Three traced runtime tuners have default true:

| Tuner | Object / live-pointer slot | Representative consumers |
|---|---|---|
| `gBoardPlay_DisableBoardPlayPWHL` | `0xfac7528` / `0xfac7568` | `0x46ebb20`, `0x4739ac0`, `0x4757b70` |
| `gPWHLRule15_5_NoChangeOnPenalty` | `0xface0d0` / `0xface110` | `0x4658430`, `0x47c73d0`, `0x484d4d0`, `0x4894430` |
| `gChecking_TreatPWHLAsIIHFIntent` | `0x103f3578` / `0x103f35b8` | checking functions around `0x4752350..0x475c960` |

Constructor `0x13139e0` stores the default at object+0x38; object+0x40 points to the live registration whose bool is at +0x29. This is build-specific. Some checking callers execute the classification predicate but discard its result: a reference alone is insufficient to establish a behavioral effect.

Jailbreak has several distinct representations:

| Representation | Evidence |
|---|---|
| `Rule_JailBreak` CSV property | constructor `0x4676650`; descriptor+0x1f0, data+0x1f8, rows+0x210, stride+0x218 |
| Flat Rule row | reader `0x4677fc0` copies it to index 10 / +0x28 |
| Rule snapshot cache | caller `0x7d121a0`; storage is not a proved enforcement consumer |
| Settings bridge | `0x67859a0`, setting ID `0xc70` |
| Frontend setter | `0x52e7b50`, named Jailbreak |
| Mode initialization | `0x3802ac0` → `0x38e9950`, slot 4 |
| Core `NHL_Jailbreak` setting | ID `0x436`, registered by `0x3689830` and `0x6073f20` |
| Serialized fantasy setting | `0x75c63d0`, lowercase protocol name at `0x76c1180` |

**Goal-triggered penalty release, eligibility, coincidental penalties and simulation enforcement remain untraced.** No Jailbreak patch is qualified. Required port includes rule defaults, settings conversion, penalty-event consumer, simulation handling, UI and save state. Acceptance scenarios must include enabled/disabled, eligible/ineligible goals, multiple penalties, delayed penalties, coincidentals, overtime and reload.

PWHL-related art paths mention twelve club prefixes, including Boston, Minnesota, Montreal, New York, Ottawa, Seattle, Toronto, Vancouver, Detroit, Hamilton, Las Vegas and San Jose. These may include placeholders; they do not prove twelve active teams. Team registration, schedules, commentary, presentation, equipment, roster identity and competition participation are separate dependencies.

## Priority 4: Be a Pro

`0x3de3720` constructs BeAProState with ten storage/cache components. EndorsementState, ModesProgressionState and ModesConversationEvents names are absent from the NHL 24 indexes. Existing-looking names include XFactorAbilityState, XFactorStats, milestones, salary perks, league entities and conversation impacts; equal names do not establish equal layouts.

`0x3d58c40` calculates an activity display percentage using range segments of stride 0x48 and min/max fields +0x30/+0x40. It interpolates within a segment, handles segment classes and normalizes over ranges. This is a display path, not a proved energy-to-performance formula. Constants and full edge behavior require further typing before publishing a formula.

`0x3d55b20` queries health activity impacts. `0x3dc2120` auto-renews salary perks through owned, available and mandatory perk collections. `0x3dc79a0`, `0x3dc81e0`, `0x3dc8e70` provide challenge/objective leads. `0x3d59320` and `0x3ddb590` have decompiler control-flow warnings; their complete semantics are unresolved.

Backport unit: new-save state schema + progression/objective evaluators + endorsement/perk transaction rules + frontend/narrative assets + voiceover transitions. Begin with a state dependency inspector. A schema-compatible NHL 24 implementation is required before enabling controls; no existing career migration is qualified.

## Additional port scope: animations, presentation and play-by-play

Added at the user's request after the initial dossier. These are required research deliverables, not optional polish. Initial string/reference and bundle-name discovery is included in checklist groups 21–23; no compatibility or working port is established by those matches.

**Updated animations and dekes:** inventory additions and replacements against both NHL 24 builds, covering skating, transitions, puck handling, dekes, shooting, checking, goalie actions and celebrations. Trace input/AI selection through move IDs, eligibility gates, animation graphs, clips, skeleton/rig bindings, root motion, IK and gameplay event markers. Determine which moves require new executable behavior versus compatible assets, including puck attachment/release, collision windows, handedness and multiplayer actor synchronization. Deliver a per-move dependency and feasibility matrix plus a single-move prototype recipe. Qualify human and CPU execution, contact/interruption, left/right handedness, possession changes, replay and frame-rate behavior before expanding the set.

**Pregame and in-game presentation:** map intros, team/player lineups, camera direction, arena sequences, broadcast overlays, scoreboards, stat panels, replays, transitions, celebrations and mode-specific presentation. Separate movies from real-time sequences and identify timeline/event bindings, UI services, localization, roster/team/league identifiers, camera rigs and renderer dependencies. Deliver a sequence/overlay compatibility matrix and NHL 24-native adapters where direct asset reuse is unsuitable. Validate custom leagues and teams, skip/resume, gameplay handoff, goals, penalties, intermissions, overtime and shootouts. A visible overlay alone does not qualify the associated sequence.

**Play-by-play:** inventory commentary audio banks and codecs, phrase/event IDs, speaker assignments, player/team/league pronunciation mappings, phrase assembly, contextual selection, priorities, queue interruption, repetition suppression and subtitles where present. Distinguish broadcast commentary from arena announcements and Be a Pro narrative voiceover. Trace gameplay/stat events into the speech scheduler and bank lookup; identify NHL 27-only events that require an NHL 24 event adapter. Deliver an audio-bank/event/name mapping and feasibility matrix. Validate timing and attribution for goals, penalties, hits, saves and stoppages; missing-name fallback; custom teams/leagues; language/bank selection; and playback under interruption and replay. Copying sound banks alone is not a complete port.

For all three areas, classify each candidate as asset conversion, data/event mapping, executable adaptation, or unsupported pending evidence. Record exact build and asset identities, dependencies and test receipts in the editor contract. Current priorities 1–4 remain intact; these additions extend the required scope.

## Full checklist coverage

| Requested topic | Current evidence and remaining requirement |
|---|---|
| 1. League sizes, up to 48 | `0x3bb8f60` guards count < 0x30 and writes parallel 48-element arrays at +4/+0xc4; block stride 0x184. Creator `0x37da810`, factory `0x39c28e0`. Same structural ceiling as NHL 24. Schedule/playoffs/UI/runtime qualification remains required for each size. |
| 2. Additional leagues | Registry/factory/service leads indexed. No proved arbitrary league-count expansion or all dependent capacities. |
| 3. Small rosters | `0x590b180` publishes 18 skaters, two goalies, 20 playable minimum; 23 playable and 27 total maximum. `0x6274340` copies fields. Same as NHL 24. Removing validation alone does not prove safe lines, fatigue, injuries or simulation. |
| 4. Full league control | Team-control/user-team leads indexed. Franchise all-team control and CPU/human transition semantics unqualified. |
| 5. Historical rules | Jailbreak settings/storage traced above; two-line/offside leads indexed. No-forward-pass enforcement not established. |
| 6. New leagues in Franchise | Factory and custom-league evidence; schedules, drafts, financial systems and saves need integration. |
| 7. New strategies | Schema and service paths traced above; tactical effects and IDs need mapping. |
| 8. AI behavior | PWHL checking branches and new tuner leads. Poke/deke/pressure decision formulas and frequency effects unresolved. |
| 9. Disable pressure systems | Pressure-related leads cross-indexed; NHL 24 prior attribute-only work is not a full system removal or NHL 27 result. |
| 10. Attribute influence | Attribute/curve leads indexed; no complete per-rating formula dictionary. |
| 11. Heads and portraits | HeadGeometry/Texture/Variant, PortraitId/HasPortrait and Player_Art leads indexed. New mesh registration and per-player runtime resolution are not yet proved. |
| 12. Every gameplay slider | Labels and references cross-indexed. Full code-derived descriptions require consumer, curve, units, clamp and interaction traces; labels are insufficient. |
| 13. Offline Ones | ones_now/ones_h2h/gOnesCPU leads. Removing AI or changing participants unqualified. |
| 14. Hits | TimeAfterRelease/ProximityToPuck/InitialCheck leads. Complete hit credit event conditions not yet derived. |
| 15. Takeaways/giveaways | Turnover and event leads. Possession, attribution and stat-commit logic unresolved. |
| 16. Simulation | simengine.brk/nhl_gm_simulation/SimPlayer leads. Statistical formulas, random draws and season aggregation unresolved. |
| 17. Capacity limits | 48-team block and roster service constants confirmed. Global player, jersey, equipment, league, looks and team capacities not established. Field width alone is not capacity. |
| 18. All-Star integration | DYNASTY_ALLSTARGAME/TodayIsAllstarGame/GAMETYPE_ALLSTAR leads. Scheduling, performance selection and season-state persistence unqualified. |

The machine-readable `local/nhl27-eboot/checklist-evidence.json` retains cross-build string VAs and candidate references for all topics, plus PWHL, Be a Pro, animations/dekes, presentation and play-by-play. It deliberately labels untraced items as leads. Broad voiceover, animation and presentation matches can include unrelated engine or narrative systems and require classification.

## Asset evidence and tooling boundary

All 38 source TOCs were inventoried: 372276 bundle occurrences, 362315 unique names and 9961 exact Data/Patch name overlaps. These are bundle-name counts, not effective resource counts. NHL 24 base has 129715 bundle occurrences; update has 20187. Patch precedence is not inferred from matching names.

Initial bounded PS5 span decoding succeeded for `Data/Ps5/globals.toc`: 249 bundles, 96485 asset occurrences. The observed identifier is 16-bit layer / 32-bit persistent install index / 16-bit CAS number; it resolves through Data/layout.toc. Flags 0x84 and, in the follow-up, 0x80 supply an identifier; flag zero inherits it. Manifest count, bounds, names and asset/span correspondence were checked. The familiar manifest magic 0x9d798ed6 is present. Both game-level manifests now parse. This is not proof of resource compatibility. Actual strategy CSV payloads remain unextracted.

Private outputs remain under ignored `local/nhl27-eboot/`: identity, indexes, relocations, decompiler output, inventories, manifests, cross-build comparisons and native-slice results. Tools and findings can be versioned without distributing executable bytes or extracted proprietary resources.

## Validation and editor handoff

Thirteen synthetic reader tests pass. The two exact-byte native slices pass 3810 synthetic cases total. Globals manifest parsing provides a real-file structural check. None of those checks is a gameplay test, a save compatibility test or proof of a working backport.

See [editor contracts](nhl27-editor-contracts-2026-10-15.md) and [tool runbook](../../scripts/nhl27/README.md). The [compatibility follow-up](nhl27-port-compatibility-2026-10-15.md) resolves the observed asset span encoding, extracts both target shared-type tables, and documents schema and external-reference barriers. Playbook payload extraction and tracing a single strategy through both games remain outstanding. Jailbreak requires the actual penalty-release consumer before any editor toggle can claim to implement it. Broader cross-title work remains parked in [the deferred plan](../plans/cross-title-executable-editor-deferred-2026-10-15.md).
