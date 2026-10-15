# NHL19 Mogilny scalp alignment and 2K texture revision

**Runtime follow-up: combined revision failed launch.** The user reported shadPS4
crashing immediately when launching NHL19. The receipt-driven rollback restored
all three archives to their previous successful full hashes; the user confirmed
NHL19 launches again. Both mesh and texture changed, so the failing component is
not isolated. Windows reported Qt6Core access violations without a new game log;
those reports alone do not establish the cause. Do not reinstall this combined
package as a known-good build. Next: the mesh-only isolation package below.

User's in-game review requested crown alignment, retained Eichel lower neck/chest,
the supplied 2K diffuse, and export of all original Eichel material maps. This
supersedes the first package's placement/texture, not its UV correspondence result.

## Geometry

`scripts/blender/nhl19_finalize_game_mesh.py` now applies the reusable
`head_scalp_placement.py` after the verified UV fit and seam closures. The old
mouth placement remains reproducible with `--keep-mouth-height`.

- Donor crown: Blender Z 1.9450141191482544 m.
- Incoming fitted crown: 1.9628503322601318 m.
- Rigid upper translation: Z -0.01783621311187744 m (native Y).
- 280 native vertices at donor Z <= 1.68 m retain exact original positions.
- Smoothstep transition through donor Z 1.68-1.72 m; 2710 upper vertices move
  exactly by the translation relative to the previous successful package.
- Crown equality is exact in exported native floats. Native counts, UVs and
  topology unchanged. Zero collapsed triangles; the same 38 orientation changes
  relative to vanilla as before, with no new IDs introduced by this revision.
- Eyes move with the head, lashes follow the placed eyelids, all three hair
  layers remain conformed and inset. Minimum radial scalp clearance > 6.27 mm.

Three matching-camera untextured donor/revision pairs and three textured views
are in `H:\TEMP\nhl19-mogilny-scalp-2k`. Textured Blender review uses the supplied
face image; preview eye materials are untextured. Game eye bindings are preserved.

## Textures

Input: `S:\Downloads\Mogilny Test\head_alex_mogilny_cm.png.png`, 2048x2048.
Output BC1 GNF: 2,798,848 bytes, 12 levels down to 1x1. Linear-light mip averaging,
original tile mode/format/numeric format preserved. Only size-related original
header fields change: file length, width/height, last mip, pitch, data length.
512/1024 vanilla map headers corroborate these fields; the 2K descriptor still
needs runtime review. The original BC5 head normal remains installed unchanged.
Independent decoded diffuse RMS vs input is 0.00937177 on a 0-1 RGB scale.

Original texture exports are under `Eichel_Textures`, each with original GNF,
lossless full-chain DDS, base PNG and R/G/B/A previews:

| Asset | Size | Format |
|---|---|---|
| head_jack_eichel_cm | 1024x1024 | BC1 |
| head_jack_eichel_nm | 512x512 | BC5 signed normalized |
| hair_jack_eichel_ab | 1024x1024 | BC1 |
| hair_jack_eichel_cm | 1024x1024 | BC3 |
| hair_jack_eichel_sm | 512x512 | BC1 |

`eb_find eichel` scanned all 25 installed archives: eight matches (three MCD,
five texture entries), no errors. No separately named Eichel head specular map.
Do not label the hair `sm` as a head map or infer its shader channels from its name.
All exported original texture hashes match the preserved vanilla copies.

The normal's numeric-format descriptor is 1. The exporter handles signed BC5,
preserves the compressed channels in DX10 BC5_SNORM DDS, and reconstructs blue
for the PNG. The older generic `gnf::decode_bc5` is UNORM-only; using it here
produced an incorrect preview, so the new exporter explicitly selects SNORM.
PNG R/G represent signed values remapped to 0-255, blue is reconstructed, alpha
is synthetic opaque; GNF/DDS preserve the original compressed data. No channel
flip is assumed. Format references: [AMD register specification](https://docs.amd.com/api/khub/documents/XFXWgxEsHO9Zh0EMGkPACA/content)
and [Microsoft block compression](https://github.com/MicrosoftDocs/windows-dev-docs/blob/docs/uwp/graphics-concepts/block-compression.md).

## Validation and deployment

- Two synthetic crown-placement cases passed (longer and shorter incoming head).
- Signed-BC5 endpoint/neutral-normal test passed.
- Native profile-gated writer checked all seven submeshes in both donor revisions.
- Production chunkzip decoded all four packaged payloads exactly; production MCD
  reparse verified every position, topology and packed normal.
- Production GNF detiler matched the intended blocks for all 12 mip levels;
  file/data size, pitch and mip-count descriptor assertions passed.
- Production EB virtual append checks passed for all four entries, with unrelated
  entries unchanged. Live baseline full hashes match the previous installed receipt.
- `revision_checks.json` contains exact retained-vertex, upper-translation and
  triangle checks; `mesh_report.json` contains placement and companion geometry.

Package root: `H:\TEMP\nhl19-mogilny-scalp-2k`.
Installation uses append-and-retarget on the same three archives. Its rollback
returns to `H:\TEMP\nhl19-mogilny-game-test` (the previous successful version).
To return all the way to vanilla, restore this revision first, then the original
package. Keep both folders and installation receipts. Do not restore the original
package while this revision is installed; its fingerprint guard will refuse.

Installed after the user closed shadPS4. `installation.json` status is `installed`;
all three full installed archive fingerprints were recorded. Production `eb_extract`
read back all four replaced payloads from the actual archives and their SHA256
hashes matched staging exactly. The original head normal and three hair maps also
read back byte-identical to vanilla. `live_readback.json` records these eight checks.
The new sizes are 13,547,381,667 / 759,915,131 / 1,379,690,219 bytes for base,
patch91 and patch93 respectively. No game launch, runtime validation, commits or
pushes were performed for this revision; it is ready for the user's next game test.
