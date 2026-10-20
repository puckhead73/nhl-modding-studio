# NHL 27 → NHL 24: animation, presentation and play-by-play compatibility

Investigated against the supplied NHL 27 PS5 01.003.001 executable/install and NHL 24 PS4 base/1.60. Build hashes are in the [main dossier](nhl27-backport-2026-10-15.md). This follow-up establishes container, schema and dependency evidence. **No new animation, presentation sequence or commentary feature has been tested in NHL 24. Full port compatibility remains unqualified.**

## Findings that change the implementation approach

| Area | Verified finding | Port consequence |
|---|---|---|
| PS5 asset access | 0x80 and 0x84 both carry an eight-byte CAS identifier in the inspected records; complete span streams meet the flags table exactly | The previous game-bundle extraction blocker is removed |
| Small camera controls | Five assets normalize to byte-exact NHL 24 references by removing the extra zero EFIX word | A narrow container adaptation is proved for these unchanged assets |
| Animation/dekes | NHL 24 packages its game animation state in an ANT AssetBank; NHL 27 exposes many individual AF resources | Need graph/clip/event mapping and target-bank reconstruction; standalone-name absence is not feature absence |
| Presentation graphs | Many shared types have changed signatures, including property/event connections, logic prefabs, camera lens presets and tweens | Reconstruct compatible target graphs and bindings; do not transplant source graph bytes |
| Presentation textures | Sampled PS5 texture resources are 184 bytes; PS4 counterparts in the sample are 132 bytes | Require a target texture header/layout adapter and separately qualified chunk/swizzle handling |
| Commentary database | Six shared database types differ and two interruption-related types are new in the sampled source database | Translate sentence/event data and adapt scheduler features |
| Commentary wave resources | Sampled same-name resource records use different structures; target uses SBle banks while inspected source records are small non-SBle records | Need a source wave-data resolver and target bank construction, beyond copying audio or EBX |

These are per-layer observations, not a fully resolved effective install. Both Data and Patch occurrences are retained. Matching names, resource type numbers, GUIDs or even type signatures alone do not qualify dependencies or gameplay behavior.

## Evidence scope

The initial compatibility inventory reads game/globals manifests in both source layers and both target builds. Source game manifests contain 46354 base and 46589 patch asset occurrences. Payload sampling decoded **618 occurrences**, all matching their manifest stored-stream SHA-1 and expected decoded size. Of these, 407 EBX occurrences passed bounded RIFF/EFIX inspection; 170 source/target same-name pairs were compared.

An additional presentation pass selected intro, pregame, scoreboard, scoreclock and broadcast names across UI/content/world/movie superbundles, plus all ten NHL 24 base UI bundles. It inspected **1539 bundle occurrences / 38293 asset occurrences** and decoded **278 payload samples**, with no recorded parser, hash or size failures. This is a named sample, not a claim that all animations, movies, sound streams or presentation bundles have been decoded.

Source shared type descriptors contain 4384 types and 32386 fields. Exact runtime GUID/signature matching resolves 4244 type names. The executable stores names through ELF relocations, so raw zero pointer bytes must not be read as null. Field-name recovery additionally checks field hash and offset. The source descriptor file SHA-256 is `814e4a88f36f3245249828ef44f9067f905307928dee8e9cc0c482a893632936`.

NHL 24 initfs is an encrypted DbObject in this install. The 1.60 loader at `0x05d053f0` identifies its AES-CBC key/IV source; a hash-bound tool uses that material in memory, with no key printing or persistence. Both initfs files decrypt, pass padding checks and fully parse. The base shared table contains **2183 types**; the patch supplies **six new descriptor records plus seven references to existing signatures**. Treating all thirteen signature entries as new descriptor records would misread the patch.

Base descriptor SHA-256: `b7ec7902240484948c6c8ffcad06aa6c38100b881b25d0bbed2c993dd932100c`; patch: `6a3175a3f368cb192ee56e4a143e5d045d3c156e011bfdba0efce0f4701e8418`. Source-to-1.60 comparison yields 700 equal-signature pairs, 1012 changed-signature pairs and 2673 source type GUIDs absent from the target table. There are 4385 comparisons for 4384 source types because target signature variants are retained. These counts describe schemas, not numbers of ported features.

Source and target field-name hashes differ. They cannot be joined by hash alone. A conservative static pass over four target registration intervals recovers 396 literal name/hash candidates covering 171 unique hashes, enough to identify the key commentary field rearrangements below. This is targeted registration evidence; unnamed fields remain unmapped, and no source layout is substituted for a missing target layout.

## Animations and dekes

Follow-up: the [DCT conversion stage](nhl27-animation-conversion-2026-10-15.md) now exports 20 sampled source clips to neutral motion buffers. Target bank writing and runtime playback remain unqualified.

NHL 24 base's `animations/antanimations/levels/game/game_gen4b_antstate` resource, type `0x51a3c853`, decodes to **97483523 bytes**; 1.60's version is **93993485 bytes**. Both stored hashes and decoded sizes validate. The source AF resources are independently addressable EBX/resource records, including animation resource type `0x4088bf7e` in the sampled game bundle.

The sampled source inventory has 1039 deke-named `(kind, name)` records. Fifty have exact case-insensitive full/path-tail name matches in the target ANT string sets. Examples include bank-deke left/right and DK18 missing-FH-to-FH clips. Those fifty are name correspondence leads, not matching motion curves. The remaining 989 are **not** proven new moves: ANT names, generated configuration names and packing differ, and the comparison is deliberately conservative.

The target executable already contains `DekeControllerAsset` and registry/type consumers (base examples `0x564175`, `0x5886d9`, name accessor `0x636a60`). Source `DekeControllerAsset` has size 88 and fields DekeCoords (+56), DekeControllerSyncPointBlendSpaceRef (+64), DekeControllerAPI (+72), and DekeControllerBlenderPriority (+80). These source offsets are not NHL 24 write addresses. The previously indexed `DekeSpeedEffect` function at `0x47f69c0` only returns a name; it is not evidence of the speed-effect formula.

The source DekeControllerAsset GUID is absent from the recovered target shared table despite the target executable containing that class name. This reinforces the need to distinguish ANT's class registry from the source's standalone EBX schema; a same-name class is not a ready-made compatible loader.

Port feasibility: individual animations/moves remain plausible conversion candidates, but neither a raw AF bundle transplant nor direct machine-code transplantation is qualified. A first move needs: source graph and referenced clips, target ANT entry mapping, skeleton and track compatibility, root/puck/stick motion, synchronization markers, input/CPU eligibility, interruption/collision handling and save/config references. Existing-name moves provide useful controls before attempting a source-only behavior. The ANT record/track writer and complete source graph closure are still missing.

## Pregame and in-game presentation

Five controls converted to byte-exact target payloads in both base and 1.60, yielding ten successful pair checks:

- `content/camera/camera_body`
- `content/camera/camera_body_nis`
- `content/camera/flexcamtransformvariable`
- `presentation/icechannels/nhlicechanneldepthoffield`
- `presentation/icechannels/nhlicechannelenvironmentglare`

For these controls, EBXD and EBXX already agree; EFIX differs only by one extra zero word. The guarded converter updates RIFF/chunk lengths and refuses any other difference. The other 160 sampled pairs were rejected. This is a format control: it adds no new visible feature and is not a general EBX downgrade recipe.

In the initial 60 presentation EBX pair comparisons, 50 have changed shared-type signatures and 14 have source-only type references (categories can overlap). Changes include `PropertyConnection`, `EventConnection`, `DataField`, `LogicPrefabBlueprint`, `FloatTweenEntityData`, `CameraLensPreset`, `EventAndGateEntityData`, `VisualEnvironmentEntityData`, `PropertyCastEntityData`, `CallFunctionEntityData`, and `DelayEntityData`. These affect graph wiring, parameter access and sequence behavior. Reusing only a camera path does not establish that its event/timeline graph will work.

The wider pass finds real-time `PartitionTrackRootData` and timeline-related types, shader/texture dependencies and movie-related `MovieTexture2Asset`, `VideoStream`, and `MovieFrameInfo`. The presence of a MovieTexture asset does not prove the encoded video stream is compatible or decoded. Texture resource type `0x6bde20ba` is shared, but header sizes differ as described above; preserve this distinction in the editor.

Executable evidence: source `0x00e17130` constructs CameraFramework toolkit/container/pipeline/director objects; `0x07c27200` binds presentation data such as announcer and first/second commentary voice busy/free/remaining-time values through hashed data lookups. These are constructor/binding paths, not proof of identical target implementations. Source `0x035e2020` binds MainCommentaryDb and NhlAudioEvents, including AudioTimelineTag and replay reaction events. Presentation and commentary therefore have joint event dependencies.

Implementation route: port one independent visual/camera element through target schemas, then reconstruct a complete intro or replay sequence with target-supported nodes. Qualify custom teams/leagues, missing assets, skip/resume, cut-to-gameplay, stoppages, goals, intermissions and overtime. Target renderer texture/shader support and external-reference closure must precede a whole sequence test.

## Play-by-play

The sampled main commentary database retains the same partition GUID but changes these shared type signatures:

| Type | NHL 27 | NHL 24 base/1.60 |
|---|---|---|
| SentenceDatabase | `ea207f87` | `98a61571` |
| SentenceEvent | `6b134ec6` | `4cb583ab` |
| SentenceSoundPatchMapping | `91edbcf9` | `a4b65d4f` |
| SentencePlayerVoiceCategory | `6ab548a6` | `db0f6813` |
| Phrase | `967d7db5` | `ec8ec079` |
| SingleInstancePerSoundSettings | `ee7a9614` | `5e9a7fda` |

Source-only database type references include `SentenceInterruptMarkerBasedFeature` and `SentenceInterruptMarkerBasedSettings`. Source fields expose CurrentSpeaker, NextSpeaker, AllowInterrupt, AllowOverlap and InterruptIfTimeUntilNextSilenceGreaterThan. The feature also holds ForceInterruptOffsetTime, InterruptFadeTime and SilenceRegionInterruptFadeTime. These identify concrete scheduler dependencies; actual timing units/behavior beyond names require consumer tracing.

`SentenceEvent` contains EventParameters, Name, Sentences, EventId and SoundMappingParameterIndex. `SentenceDatabase` links Transcriptions, Pickups, Events, AlwaysLoadedCollections, EventSystem, OverflowPartitions, Keywords, OptionalFeatures and PlaybackSettings. A port must preserve their relationships rather than just preserve sentence IDs.

Recovered layouts give concrete migration requirements (decimal object offsets, not executable patch offsets):

| Field | NHL 27 offset | NHL 24 offset |
|---|---:|---:|
| SentenceDatabase.EventSystem | 64 | 32 |
| SentenceDatabase.Events | 48 | 80 |
| SentenceDatabase.Transcriptions | 32 | 96 |
| SentenceDatabase.OptionalFeatures | 88 | 64 |
| SentenceDatabase.PlaybackSettings | 104 | 88 |
| SentenceEvent.Name | 32 | 24 |
| SentenceEvent.Sentences | 40 | 32 |
| SentenceEvent.EventParameters | 24 | 40 |
| Phrase.Parameters | 32 | 56 |
| Phrase.PlaybackMessage | 56 | 32 |
| NewWaveAsset.ParentInstanceGuid | 108 | 80 |
| NewWaveAsset.DefaultSelectionBehavior | 72 | 64 |
| NewWaveAsset.StreamPool | 80 | 72 |
| NewWaveAsset.ControlParameterCount | 126 | 96 |

SentenceDatabase remains 120 bytes in both builds, SentenceEvent 56 and Phrase 64. Equal sizes therefore conceal real field permutations. Relative pointer fixups, external imports and array tables must be rebuilt with the target layout. Blindly changing the type signature or truncating the container would corrupt these relationships.

The source NewWaveAsset schema signature is `d8795edc`, size 128; the sampled target signature is `86db9ddf`, and the base registration at `0x00d570c7` encodes size 104 (signature construction at `0x00d570e6`). Same-name wave resource type `0xb2c465f6` is therefore insufficient to establish compatibility. For example, `b_incidental_puck_collision_dummy` is a 32-byte non-SBle source resource versus a 344-byte SBle target bank. Another inspected source record is 40 bytes versus a 400-byte target bank. The actual source stream-locator grammar and codec payloads are not yet established; these small records must not be presented as decoded speech.

Some supporting assets, including keywords, sentence pickups, player sample metadata/probability and transcription collections, retain matching referenced type signatures in the sample. Their content and references still differ. They are better data-migration candidates than the entire database, but need target schema validation and dependency mapping.

Port route: resolve source sound streams and variation metadata; map sentence/event/parameter and pronunciation identities; construct target-compatible banks and wave objects; translate database records; then adapt source-only events and interruption rules. Use existing target events first. Arena announcements and Be a Pro voiceover remain separate systems. No claim is made that all NHL 27 voices, languages or player names can currently be imported.

## Qualification status and remaining blockers

Twenty synthetic reader/converter tests pass, and all research scripts compile. Real-file evidence adds 896 decoded payload occurrences across the two sample passes, two large ANT controls, both target initfs/shared-type tables, and ten exact-target EBX control conversions. These checks validate parsing, identity and a narrow conversion; they do not validate gameplay, audio playback, rendering or saves.

The dependency probe reads exported instance GUIDs using bounded EBXD offsets and compares complete partition/instance pairs. It parsed 568 unique sampled EBX payloads without errors: 285 NHL 27, 145 NHL 24 base and 138 NHL 24 1.60. Across 6377 source import occurrences, 181 resolve within the source sample; 50 resolve within each target sample. The remaining 6196 source / 6327 target references are **outside the decoded sample**, not proven missing from the installs. Counts retain source variants and repeated imports; they are not unique dependency counts. Every candidate variant is retained, with no assumed patch winner. Full dependency closure remains unqualified. Private evidence: `dependency-evidence.json`.

Post-analysis SHA-256 verification of all three original executables matches the recorded input identities. The receipt is `post-analysis-executable-hashes.json` under the private compatibility directory.

Still required for full port compatibility:

1. Complete per-feature field-name mapping and external dependency closure, including actual effective layer resolution. Source and target shared layouts are now recovered; interpreting every field and engine consumer is separate work.
2. ANT graph/track decoding and writing, plus animation synchronization and gameplay-event mapping.
3. PS5 texture/chunk and movie stream conversion into target-supported layouts/codecs.
4. Source NewWave stream/variation resolution, target bank generation and sentence/event translation.
5. Isolated feature packages and runtime tests on the specific NHL 24 CPU-native build, including rollback and saved-mode checks.

No install, executable or save was modified. Converted controls and extracted data remain private under `local/nhl27-eboot/compatibility`. Production Studio write controls remain disabled by contract. The scripts are in `scripts/nhl27`; the [runbook](../../scripts/nhl27/README.md) reproduces this work. The original strategy, Franchise, PWHL and Be a Pro requirements remain in scope; this follow-up does not silently mark those ports complete.

## Format references

Local repository readers supplied the TOC/manifest and CAS framing baseline. The upstream [Frosty CAS reader](https://github.com/CadeEvs/FrostyToolsuite/blob/1.0.6.3/FrostySdk/IO/CasReader.cs) identifies compression type 0x11 as Oodle. Its [RIFF fixup reader](https://github.com/FrostyToolsuite/FrostyToolsuite/blob/master/FrostySdk/IO/RiffEbx/EbxFixup.cs) and [shared type reader](https://github.com/FrostyToolsuite/FrostyToolsuite/blob/master/FrostySdk/IO/RiffEbx/EbxSharedTypeDescriptors.cs) provided format cross-checks; observed NHL files determined the accepted variants. These references describe formats, not NHL 27 → NHL 24 compatibility.
