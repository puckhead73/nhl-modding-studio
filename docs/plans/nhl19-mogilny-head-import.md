# NHL19 Mogilny over Eichel: experimental import plan

Prepared for Claude, 2026-09-09. Scope: import the supplied Alexander Mogilny GLB into NHL19 PS4 by replacing Jack Eichel's existing head asset. This is a bounded research detour, not general mesh-import support. No importer, game modification, or runtime validation was performed during scoping.

## Recommendation and completion criteria

**Latest user steering and implementation:** [coordinated eye-unit fit progress](../validation/2026-09-10-n19-coordinated-fit-progress.md). Absolute eye-position freezing was limiting likeness. An explicit offline experiment now translates each native eye/socket/lash unit toward source landmarks while retaining its internal shape. The mouth remains unchanged until its source landmark is established. Earlier absolute-freeze wording describes the conservative baseline; it must not silently override this user-requested direction. Animation compatibility is still unproven.

User-directed revision: prioritize the easiest route to an accurate likeness with working eyes, mouthbag, and lip separation. **Path B, constrained donor deformation, is the primary and sufficient route for this test.** Mogilny's GLB supplies the reference shape and texture; the output retains Eichel's native topology, vertex order, UVs, weights, and facial-data indexing. Shipping the source topology is not required. This supersedes the initial topology-replacement requirement.

Use Eichel's native RSF container as the template. First prove a lossless no-op and a small, same-size geometry edit through the actual game-loading path. Then fit the external donor face to Mogilny with protected eye and mouth structures, regenerate shading attributes, and bake Mogilny's texture onto the donor UVs. Keep geometry, textures, and archive delivery experiments separate.

The intended result is a recognizable Mogilny likeness under Eichel's identity, with stable neck attachment, eyes that blink without socket collapse, and lips that separate around a correctly seated mouth interior. This minimizes new rigging and binary-layout work; it does not guarantee exact source-surface reproduction. Frozen donor anatomy may limit likeness around the eyes and mouth. Measure and show that tradeoff before relaxing constraints. Actual source-topology replacement and weight transfer are deferred research, not automatic fallback or acceptance requirements.

## Verified inputs

Source directory: `S:\Downloads\Mogilny Test`.

| Input | Observed facts |
|---|---|
| `alex_mogilny.glb` | 626,300 bytes; glTF 2.0; generator `Khronos glTF Blender I/O v1.6.16` |
| Head mesh | `head_3_0_head_mesh0`; 17,400 vertex records and 17,400 indices: 5,800 triangles; 2,924 distinct bitwise position triples |
| Eyes mesh | `head_3_1_eyes_mesh1`; 960 vertex records and 960 indices: 320 triangles; 198 distinct bitwise position triples |
| Attributes | POSITION, NORMAL, TEXCOORD_0; unsigned-16 indices; no skin, joint/weight attributes, morph targets, or tangents |
| Scene | Two mesh nodes, each with quaternion approximately `(0.70710683, 0, 0, 0.70710671)`, a +90-degree X rotation |
| Material | Both primitives use one material named `None`; no image/texture references in the GLB |
| `head_alex_mogilny_cm.png` | 1,567,511 bytes; 1024 x 1024, 8-bit RGB |

Source SHA256:

```text
alex_mogilny.glb
E983E37218E99AAB383FCA349229B33899FC0CDC8923385707F6EE712D2D2B29
head_alex_mogilny_cm.png
B89922F7CAEEA69F9170A3005F831865C06185F1222013B57F6D62E9D0C07C64
```

The position counts indicate considerable duplication. They are **not** safe welded vertex counts: UV and normal seams must survive. No provenance beyond the supplied file metadata is assumed.

Head accessor local bounds are approximately `(-0.104818, -0.067528, -1.842376)` to `(0.104813, 0.157603, -1.489029)`. These are pre-node-transform coordinates, not an established NHL19 placement. Apply the scene transform exactly once, then measure alignment to Eichel; do not guess a unit multiplier or recenter away the neck attachment.

## Verified target

**2026-09-10 H0 review correction:** the extracted mesh below is a concatenation artifact, not a valid donor. Use a separately decoded archive entry. See [H0 review and H1 dispatch](../validation/2026-09-10-n19-h0-review-h1-dispatch.md). H0 is partially complete: offline fixture identification accepted; database/shared-reference binding and runtime precedence/baseline remain open. Offline H1 is authorized without waiting for runtime work.

Install located at `H:\Emulators\ShadPS4\Games\CUSA11126`.
Read-only parsing of its `sce_sys\param.sfo` returned:

```text
TITLE=NHL® 19
TITLE_ID=CUSA11126
CONTENT_ID=UP0006-CUSA11126_00-NHLICEHOCKEY2019
APP_VER=01.60
VERSION=01.00
```

This establishes this local installation's identity; it does not resolve all discovery-table regional variants. Runtime destination (PS4 console or shadPS4) remains to be recorded.

Extracted player directory:

```text
H:\Emulators\ShadPS4\Games\CUSA11126\databigs\Extracted\nhl\data\kettle\rendering\characters\player
```

| File | Bytes | Observation |
|---|---:|---|
| `jack_eichel.mcd` | 3,129,504 | Extracted concatenation artifact; prohibited as a writer donor |
| `head_jack_eichel_cm.gnf` | 701,696 | 1024 x 1024; BC1 data format 35; numeric-format field 9; tile index 13; last mip level 10 |
| `head_jack_eichel_nm.gnf` | 354,560 | 512 x 512; BC5 data format 39; numeric-format field 1; tile index 13; last mip level 9 |
| `hair_jack_eichel_cm.gnf` | 1,403,136 | Present; not decoded during scoping |
| `hair_jack_eichel_ab.gnf` | 701,696 | Present; not decoded during scoping |
| `hair_jack_eichel_sm.gnf` | 177,408 | Present; not decoded during scoping |

Extracted-file SHA256 (mesh hash identifies the artifact, not a native donor):

```text
jack_eichel.mcd
BEB038171322B417FC46CE07578F5B0DF9B1051B60D4E5F2BC33E71AD0021672
head_jack_eichel_cm.gnf
A837271B508A8ED67CFA67A3BA8BABBE3F876AB8080F17CE0B5DACEC07FD895F
head_jack_eichel_nm.gnf
3DAF6632D6B45DDCD06F33EC382C15F42CA81FEA7AA675B4A032BE25EC14FD4E
```

Initial marker inspection found three `TIEW` markers at decimal offsets 16, 1,043,184, and 2,086,352. H0 established that these belong to three concatenated 1,043,168-byte containers, not three views of one head. Codex independently compared each third against the decoded scratch archive entry: all three matched exactly.

Each native entry has one VIEW and a 3,157-record stride-40 face stream. H0 reports seven submeshes including separate left/right eyes, lashes, haircap, and hair cards. Stream/index ownership remains an H1 gate: the preview decoder produces asymmetric eye triangle counts and is not authoritative for writing.

Corrected donor candidates: `game_90-008.big` entry 62195 and `patch_game_93.big` entry 3036 both decode to SHA256 `4DA8F5D5C9EAA64F3AAA38B7E9E4CB0B0AEA74B91A1884FF863BF0CC61243195`; `patch_game_91.big` entry 2608 decodes to `3ADDE6DF160DB75240FC9FF907AF8D05F4BE41CD6866150AEB08435F8761932D`. All are 1,043,168 bytes. These hashes were independently checked. Use base/patch93 bytes for the offline profile and preserve archive identity separately; runtime winner remains unproven. The revision difference is confined to offsets 203972 through 211545 inclusive (zero-based); reported facial-name ordering must be bound to actual native references before it can guide vertex masks.

Base and patch `.big` archives coexist in `databigs`, including `game_70.big` (>5 GB), `game_90-008.big` (>13 GB), and `patch_game_91.big` through `patch_game_96.big`. Eichel's authoritative archive entry and patch precedence were **not determined** in this scoping pass. The extracted tree is an inspection source, not proof of a runtime override path. No player `.skel` was found under the extracted `rendering/characters` tree; resolve the actual dependency instead of inventing a skeleton path.

## Existing code to reuse, and gaps to respect

| Location | Use / limitation |
|---|---|
| `crates/studio-domain/src/assets.rs` | Native asset resolution and `.mcd` dispatch; geometry-only GLB export |
| `crates/tdb-rx2/src/mcd.rs` | RSF decoder, VFMT and stream formats; currently heuristic matching by stride and index validity, not a lossless editable representation |
| `crates/tdb-rx2/src/playermesh.rs` | Shared preview/export geometry; insufficient to preserve the entire native container |
| `crates/tdb-headmesh/src/glb.rs` | Reusable accessor logic, but current primitive ingest does not bake mesh-node transforms; material-first naming makes both supplied parts `None`; normals/tangents are not retained in `GlbPrimitive` |
| `crates/tdb-headmesh/src/placement.rs`, `transfer.rs` | Inspect for reusable fitting/transfer mathematics; do not inherit Legacy rig assumptions |
| `crates/tdb-headmesh/src/pack_rw.rs` | RW output only; **not** an NHL19 RSF writer |
| `crates/tdb-headmesh/src/pack_mcd.rs` | Existing PS4 packer entry point is a stub returning `Unimplemented`; use it as the target adapter rather than building a disconnected conversion pipeline |
| `crates/tdb-headmesh/src/morph.rs`, `crates/blender-job/`, `defs/appearance/morph-donors.toml` | Path B profile/job machinery; current profiles are Legacy donors, so measure a new Eichel profile and verify the actual job implementation before reuse |
| `docs/MESH_IMPORT_EXCLUSIVE_PATH.md` | Supplied Legacy method: freeze eyelid pockets and boundary loops, retain donor eyes/mouthbag/UVs, recompute normals. Transfer the constraints, not Legacy coordinates/radii or platform encoding assumptions |
| `crates/tdb-headmesh/src/facial.rs` | Documents upper/lower-lip confusion in spatial transfer; use as a warning and source of semantic checks, not a reason to transfer weights in this donor-preserving route |
| `crates/tdb-rx2/src/gnf.rs` | Native base-mip replacement preserves header and trailing mip bytes; does not regenerate the mip chain. `encode_gnf` is a synthetic fixture builder, not a game-qualified encoder |
| `crates/tdb-archive-big/src/eb.rs` | Archive readers and experimental in-memory writers; large-file streaming write and exact-target qualification remain work |
| `crates/tdb-archive-big/examples/eb_extract.rs` | Read-only entry extraction probe; selects the first substring match, so do not reuse its selection policy for replacement |
| `docs/formats/ps4-rsf-mesh.md` | Starting format notes; some open-item descriptions lag code (index reuse now has a fallback). Check actual implementation and this NHL19 fixture |
| `docs/plans/project-completion-roadmap.md`, C3a/C4 | Required archive identity, payload, restoration, and GNF constraints |

The PS4 preview/export path does not supply a complete usable native bone palette or full morph representation. A GLB export/import roundtrip cannot reconstruct information that export omitted. Retain the native donor alongside the GLB throughout.

## Work packages for Claude

**Latest dispatch:** [H2b review and H2c likeness refinement](../validation/2026-09-10-n19-h2b-review-h2c-dispatch.md). H2b is accepted as the conservative baseline; refine only non-anatomical visibility exclusions, compare on fixed samples with consistent shading, and provide a draft donor-UV colour bake. Preserve anatomical protections. Select the fit before final native shading/GNF packaging.

**Current dispatch (2026-09-10):** [H2a review and H2b constrained-fit package](../validation/2026-09-10-n19-h2a-review-h2b-dispatch.md). H1R fingerprinted writer corrections are accepted offline (31 targeted tests rerun). Complete cavity/source-coverage masks, align, and produce one conservative Path B candidate. H2a reports an integrated oral cavity and body-shell attachment rather than a separate mouthbag/neck ring; preserve those measured structures. Zero stored weights, a 65-name SKTN table, and small values in undecoded blocks do not establish the deformation mechanism or absence of absolute targets. Runtime compatibility remains unknown; final shaping requires regenerated normals/tangents. The original H2/H3 sequence below is gated by this dispatch.

Execute sequentially. Produce a short receipt after each package with files, exact commands/results, artifact hashes, unresolved facts, and the next gate. A blocker in archive deployment does not prevent independent mesh inspection or offline validation. No commit/push is authorized by this plan.

### N19-H0 — Lock the fixture and loading path

1. Read `CLAUDE.md` and the references above. Record the existing dirty tree; preserve unrelated work. Use the current feature-bearing working tree as the baseline rather than assuming HEAD contains `tdb-headmesh`.
2. Create a private scratch workspace outside the repository for proprietary inputs and generated game assets. Keep vanilla inputs immutable; hash copies and all dependencies. Record source hashes above and reject drift until re-inspected.
3. Build a read-only inspection command accepting explicit mesh, texture, install, and output paths. Report source scene/accessors and donor ownership candidates with offsets, encodings, and uncertainty. Promote its reusable parts into the appropriate crate; keep real fixtures out of committed tests.
4. Resolve Eichel through the actual database/appearance resolver, recording player identity, asset slug, shared references, and any alternate head/helmet variants. Do not edit roster identity just to route the replacement.
5. Enumerate matching full internal paths in all relevant base/patch archive tables. Record archive, entry index, raw row fields, stored/decoded sizes, compression, and decoded hash. Match extracted bytes against candidate entries. Establish runtime precedence and the exact entry that wins; basename and first substring matches are insufficient.
6. Record the intended runtime and baseline ability to display Eichel in the unmodified game. A crash before asset loading is a runtime baseline blocker, not mesh evidence.

Gate: exact target manifest, baseline image/log if runtime available, and a concrete deployment method. If loose-file overrides are proposed, prove their precedence experimentally. Do not assume the `Extracted` folder is consumed by the game.

### N19-H1 — Native structure and minimal writer proof

1. Add an RSF inspection/edit representation in `tdb-rx2` (suggested new module `mcd_write.rs`). Preserve original bytes and opaque spans; identify explicit VIEW/mesh/format/stream/index/material/morph relationships and all size, reference, and bounds fields affected by edits.
2. Start with the exact Eichel layout. Fail closed on ambiguous ownership or unsupported variants. The display decoder's stride/first-valid-index heuristics are not authorization to patch a stream.
3. Prove no-op output is byte-for-byte identical. Test truncation, overflow, ambiguous references, and a wrong donor fingerprint with synthetic fixtures and explicit private-fixture runs.
4. Make a small, visibly identifiable position-only edit within a confirmed face region, away from eyelids, lips, neck, and mouth interior, retaining counts/topology/weights/UVs. Update any required bounds; provide a byte-span diff proving unrelated data remained unchanged. Apply consistently to confirmed views, or document precisely which view the diagnostic affects.
5. Deliver a staged native file, reparse it, and run it through the qualified copied-archive/override path. Demonstrate the geometric change on Eichel, then restore original hashes. Keep this a modest writer diagnostic; the likeness fitting belongs to H2.

Gate: writer and loading-path proof. Offline reparse alone is `automated-tested` evidence, not `game-verified`.

### N19-H2 — Fit Mogilny's likeness while retaining Eichel's facial mechanics

1. Extend or wrap GLB ingest to follow the selected scene and accumulated node transforms, retain mesh/node/primitive identity independently of material names, and preserve normals/UVs. Test this exact two-node structure synthetically. Keep source head and eyes separate; the eyes are alignment references, not replacement components.
2. Prepare a fitting-only source surface if triangle duplication or open eye holes interfere with projection. Preserve the original textured GLB for baking. Do not indiscriminately weld, subdivide, or cap the mouth; any eye-hole patch is confined to a measured eye region. Eichel's output topology and vertex order must never change.
3. Measure an NHL19 Eichel donor profile: actual eye centers and sockets, upper/lower lid pockets, eyelashes, lip contact seam, oral opening, inner lip surfaces, mouth interior/teeth/tongue where present, and neck boundary. Locate the mouthbag or equivalent via geometry and bindings; no separately named mouthbag has yet been established for this fixture. Do not copy Legacy vertex IDs, radii, coordinates, or assume boundary-loop detection finds a closed lip seam.
4. Fit source-to-donor alignment using eyes, mouth, neck, nose, ears, and jaw, then deform only the donor's permitted outer surface. Start with rigid/similarity alignment; save the transform and front/profile overlays. Use anatomically constrained correspondence, not whole-head nearest-surface shrinkwrap. Bound displacement and smooth only within allowed regions, reapplying protection masks after every operation.
5. **Eyes:** freeze the actual eyelid-pocket geometry and eyeball positions as a working unit, plus attached lash geometry where needed. Blend the deformation into the surrounding brow/cheek using a measured falloff. Hard-protected position displacement must be exactly zero in the packed result. Do not translate eyeballs independently to compensate for a collapsed or misplaced lid. Protected Eichel eyes take precedence over matching the source's eye spacing in this first test.
6. **Mouth and lips:** initially freeze the mouth interior, inner lip rim/contact seam, corners, and a measured transition region. Keep upper and lower lip vertices distinct even when coincident in the neutral pose. Do not weld across the seam, project one lip onto the other, bridge the oral opening, or smooth across upper/lower region boundaries. Retain their native weights and facial indices verbatim. Fit the surrounding visible lip surface only with separate upper/lower correspondence and limited displacement; exclude mouthbag/teeth/tongue from projection targets.
7. **Mouthbag:** preserve its native position and geometry along with the protected oral opening. Check depth and occlusion in a mouth-open pose. Do not use an independent forward/backward nudge to hide a lip mismatch. If likeness requires moving the mouth unit, first show the fit error, then investigate a coordinated cavity/lip/deformation update as an explicitly escalated research step; it is not part of the initial wrap.
8. Retain donor UVs, topology, vertex order, weights, and all face-indexed data. Establish whether native facial data stores relative deltas or absolute target positions: unchanged indexing alone does not prove correct animation after moving the base surface. Preserve relative data initially and test it; absolute targets or dependent bounds may require consistent position updates. Unknown mechanics limit animation claims, but do not require replacing the rig to perform an offline fit.
9. Fit the single VIEW in the corrected native entry. Reject the concatenated extracted artifact rather than interpreting it as multiple LODs. Preserve the neck seam and inspect retained haircap/hair/helmet intersections. General multi-view support is not required by this fixture.

Gate: same-topology fitted donor plus region-mask visualization, front/profile likeness overlays, displacement/error statistics outside protected regions, exact-zero protected-position checks, unchanged topology/UV/weight checks, and neutral/blink/jaw-open comparison to the vanilla donor. Do not maximize a whole-face distance score by sacrificing a working eyelid pocket or mouth cavity. If native pose evaluation is unavailable, record that gate as pending runtime evidence rather than inventing a rig test.

### N19-H3 — Pack the fitted donor without changing topology

1. Implement the NHL19 target adapter through `pack_mcd.rs` and the bounded native editor from H1. Accept the donor and same-count fitted positions; reject changes to vertex order, indices, UVs, weights, and unsupported layouts. Preserve names, bindings, stream descriptors, opaque bytes, and unaffected components.
2. Patch only verified position/shading/bounds fields and any proven dependent facial-position data from H2. Native counts and stream lengths remain unchanged. A request requiring container growth or topology substitution is outside this package. Archive compression may still grow and is handled separately in H5.
3. Recompute normals and tangents for the fitted surface using the unchanged donor UVs, preserving winding, seam behavior, required fourth components, and secondary UV semantics. Frozen positions do not prohibit correctly recomputed shading attributes. Avoid smoothing normals across deliberate lip/interior seams.
4. Test byte-identical no-op, exact-count enforcement, wrong-donor refusal, protected-region edits, split streams, shared indices, multiple views, malformed input, and unsupported fields with synthetic cases plus explicit private-fixture checks. Inspect byte ranges independently of the display decoder. Reparse and verify all invariants and the intended fit.

Gate: a structurally valid staged `jack_eichel.mcd` carrying Mogilny's fitted likeness on Eichel topology, with unchanged native facial connectivity and recorded pose evidence/limitations. This donor-preserving output is the accepted import method for the test.

### N19-H4 — Texture compatibility

1. Bind the provided PNG explicitly; the GLB does not bind it. Bake source colour from the aligned original Mogilny surface onto the fitted donor's unchanged UV layout, using region-aware projection/cages. Prevent eye/inner-mouth pixels bleeding onto face/lips; inspect upper and lower lips separately and use donor interior appearance where the source has no valid coverage. Include UV-island dilation for mip safety. Preserve native donor UV bytes; determine the image bake orientation visually rather than inheriting the Xbox convention or flipping donor UVs.
2. Preserve the target colour texture's dimensions, BC1 format, numeric-format field, and native header/layout. Decode the result for visual comparison. Qualify all referenced mip levels, including padding/offsets and colour handling; do not leave Eichel in lower mips and claim a complete texture swap.
3. No source normal map was supplied. For an initial diagnostic keep Eichel's normal map only as a recorded limitation; for a coherent result bake or create a compatible neutral map after verifying the shader's channel/sign convention and BC5 encoding. Do not assume a filename suffix establishes the full shader contract.
4. Do not use `gnf::encode_gnf` to fabricate a shipping header. Extend the donor-preserving mip path as necessary. A base-mip-only result is a close-up diagnostic, not distance/mip validation.

Gate: matching face texture with validated mip chain, and an explicit normal-map decision. Keep texture-only and geometry-only diagnostic artifacts to isolate faults.

### N19-H5 — Archive delivery and in-game acceptance

1. Produce a manifest-driven replacement against copied archives or a proven override path. Native file size preservation does not imply compressed payload fit. Qualify the exact payload codec and table fields before mutation; refuse unsupported wrappers or overlapping/shared spans.
2. For archives, use checked 64-bit offsets and bounded-memory streaming. Support fit-in-slot or qualified append-and-retarget as required; preserve unrelated entries/metadata and old payload spans. Do not load a 13 GB archive into `EbFile` just to change one head. Re-extract replacements and compare native hashes before deployment.
3. Retain recoverable originals, full archive identity, original length/table fields, and all replacement hashes. Recheck source identity before installation. Restore the complete original archive/file set, including EOF after appended payloads. Do not deploy automatically as a side effect of the importer; prepare a concrete artifact for the user's runtime test.
4. Test unchanged baseline, the H1 geometry marker, imported geometry, then imported geometry plus texture. Record exact build/runtime, staged and installed hashes, and screenshots/logs. Inspect close-up and gameplay distance, front/profile/back, neck seam, head turns, blink/jaw behavior where triggered, helmet on/off, lighting, replay, and reload. Trigger customization/damage only if supported by the final recipe and record untested behavior.
5. Confirm other players/assets remain unchanged and restoration returns to the baseline. If shadPS4 cannot reach the same baseline scene, retain offline evidence and report runtime validation blocked; do not attribute an emulator failure to the mesh without evidence.

Gate: user-observed Mogilny likeness on Eichel's player using the accepted donor topology, stable attachment, correct textures across tested views/distances, intact eyelid pockets, properly seated mouth interior, lips that open without sealing/stretching through one another, and verified restoration. Compare expressions with the vanilla baseline and record any protected-region likeness compromise. Report static rendering and facial-animation evidence separately.

## Limits and handoff

No general UI, new player IDs, database migration, other game/platform support, generalized RSF authoring suite, or capability promotion is part of this detour. Reuse narrow engine modules and a reproducible CLI first. The supplied tester reference has a recorded C3a archive-work exception; this is not permission to inspect unrelated repositories or bundled third-party implementations indiscriminately.

Commit only synthetic fixtures, code, and concise receipts when separately authorized. Keep real game assets, source art, staged archives, and screenshots in private scratch storage. Do not run QA inventory generators or change support matrices as a side effect.

Suggested first instruction to Claude:

> Read `docs/plans/nhl19-mogilny-head-import.md` and execute N19-H0, then implement N19-H1's offline inspection/no-op/position-patch work. The accepted route is Path B: fit Eichel's native donor to Mogilny while retaining topology, UVs, weights, eyelid pockets, mouth interior, and lip separation. Prepare reviewable artifacts and continue independent work if runtime evidence is pending. Preserve the dirty working tree and original game files. Report exact ownership/loading-path blockers; do not introduce topology replacement or weight transfer. No commits, pushes, or live deployment in this first package. Return a receipt of at most 100 lines with commands, results, hashes, and next gate.

Scoping validation: read current source/documentation; directly inspected GLB JSON, position duplication, PNG IHDR, SFO metadata, GNF descriptors, and RSF marker/stream headers. No cargo tests, exporter execution, archive-entry resolution, or game launch was performed. All layout conclusions beyond those direct observations remain work-package gates above.
