# NHL19 Mogilny native game-test package

**Runtime update, 2026-09-10:** The user reported **"Full success"** after testing
this installed package in game. This records user-reported success for this exact
fixture; individual blink/jaw cases, screenshots and runtime logs were not separately
provided. The offline-only status statements below describe the pre-test receipt.
The reusable method is saved in [shared-UV template transfer](../MESH_UV_TEMPLATE_TRANSFER.md).

Package: `H:\TEMP\nhl19-mogilny-game-test`.
Target: `H:\Emulators\ShadPS4\Games\CUSA11126`, NHL19 APP_VER 01.60.
User authorized finishing the mesh for an in-game test and physically hiding
Eichel's hair under Mogilny's scalp. Installation requires the filesystem escalation
for the game directory. `installation.json` is the authoritative install receipt;
this document alone does not establish installation or runtime success.

## Mesh completion

- Started with the UV-template fit, retaining all native IDs and all seven submeshes.
- Closed the four remaining originally coincident seam pairs. Maximum per-vertex
  adjustment was 0.420 mm. Separate vertices and upper/lower lip topology survive.
- Native eyeballs scaled per axis to the supplied source's respective eye bounds;
  lashes follow interpolated donor-to-fitted eyelid-surface displacement.
- `geo_haircapShape` (606 vertices), `geo_cardsAShape` (106), and `geo_cardsBShape`
  (1597) conformed radially to the new scalp dome, then shrunk to 92% radius.
  Minimum measured radial clearance is 6.276 mm. Hanging cards are folded onto
  the upper dome before shrinking. All three remain enabled in the inspection
  renders, so their absence is geometric rather than a viewport visibility flag.
- Head retains 3157 vertices and 6014 triangles; no collapsed head triangles.
  38 donor-relative triangle-normal reversals remain as a diagnostic, not proof
  that these triangles self-intersect. Static/animated cavity behavior still needs
  the game test; unknown native deformation blocks are preserved, not reinterpreted.

## Native packing

`mcd_game_test_pack` accepts only the two fingerprinted Eichel revisions. It checks
every submesh's original positions, native triangle indices and UVs against the
Blender export before packing positions and bounds. The subsequent bounded shading
step regenerates normal/tangent XYZ in the measured `4f16` fields. Original fourth
components are retained (head/eye tangent W is zero; hair/lash W is one), rather
than assuming a generic glTF handedness convention. Other vertex attributes,
indices and opaque bytes are preserved.

Base/patch93 and patch91 are packed separately, so each retains its own original
skeletal-name ordering. Containers remain 1043168 bytes. Final hashes are recorded
in package.json; the base result is
`5460dd35c6077cf4718b798d9a51e3d474338cb749d3623082ba0908e97ad92f`.

## Texture

The supplied 1024x1024 PNG replaces the donor's BC1 diffuse texture. All 11 mips
are generated with linear-light averaging, encoded to BC1 and micro-tiled into
the original GNF. Original header, format, dimensions and padding are preserved.
Measured mip extents consume the 701696-byte donor exactly. Every mip independently
detiles through production `gnf::parse` to its intended linear BC1 bytes.
Decoded base-mip RGB error against source is 0.007418 RMS on a 0..1 scale.

The source atlas contains painted hair; only Eichel's separate cap/card geometry
is hidden. No source normal map was supplied, so the original Eichel BC5 normal
map is retained as an explicit first-test limitation.

## Archive delivery and restoration

Four payload replacements across three archives:

| Archive | Entry | Asset |
| --- | ---: | --- |
| game_90-008.big | 62195 | jack_eichel.mcd, base revision |
| game_90-008.big | 33208 | head_jack_eichel_cm.gnf |
| patch_game_91.big | 2608 | jack_eichel.mcd, patch91 revision |
| patch_game_93.big | 3036 | jack_eichel.mcd, base revision |

Each chunkzip preserves the original chunk count/size, header and per-chunk gap
words; raw deflate payloads are regenerated and production-decoder verified.
The new mesh compression exceeds its original slot, so delivery appends aligned
payloads and retargets only these table rows, plus each archive-size field. It does
not reflow unrelated entries. Both unidentified per-entry words remain unchanged.
Their runtime validation semantics remain unproven.

The production EB reader verified each proposed retarget, preserving all unrelated
entry records (79390 in base per individual check, 3280 in patch91, 3775 in patch93).
The base archive is 13.54 GB, so full pre/post SHA256 passes take appreciable time.
Original payloads remain on disk. The package records original headers/rows/lengths
and whole-file fingerprints, then installed fingerprints. Restoration reverts those
bytes, truncates only the appended tails and verifies the original complete hashes.
It refuses restoration if installed archives have subsequently changed.

Close shadPS4 before install/restore:

```powershell
& 'H:\TEMP\nhl19-mogilny-game-test\Manage-Test.ps1' restore
```

## Executed checks

- 31 production `mcd_write::tests` passed.
- Production chunkzip decode matched all four staged native payloads byte-for-byte.
- Production MCD reparse matched all seven position arrays/indices and decoded
  regenerated normals within 0.001 per component, for all three mesh occurrences.
- All 11 GNF mip production-detile comparisons passed.
- Four proposed EB retarget checks passed.
- Three synthetic installer tests passed: exact apply/restore and duplicate refusal,
  corrupt payload refusal, and externally changed archive refusal before apply/restore.
- Targeted Rust examples passed clippy with warnings denied.
- Front/three-quarter/profile meshes rendered with all components enabled.

The test remains experimental: cold-start the game and inspect Eichel's real 3D
head, both eyes/lashes, mouth opening, blinking, head turns and near/far views.
No game launch or blink/jaw qualification is implied by offline checks.

## Installation receipt

Installation completed successfully on 2026-09-10 after filesystem approval.
`installation.json` reports `installed` and carries all three installed whole-file
SHA256 fingerprints alongside their original restoration metadata. Production
`eb_extract` then read all four entries from the live game archives: base mesh,
base diffuse, patch91 mesh and patch93 mesh each matched the staged decoded asset
byte-for-byte. Patch91/base output differences were independently checked to be
exactly the original revision differences, with no new revision divergence.
The game was not launched. Runtime visual/animation acceptance remains the next test.
