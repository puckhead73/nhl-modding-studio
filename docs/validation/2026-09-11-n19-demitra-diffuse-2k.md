# NHL19 Demitra: isolated 2K diffuse test

**Runtime outcome:** the user confirmed "The 2k texture was a success" on
2026-09-11. The successful baseline is revision2 geometry plus this 2K diffuse.

Requested after the user-confirmed successful revision 2 mesh/1K diffuse import.
Package: `H:\TEMP\nhl19-demitra\diffuse2k`.
Baseline/rollback target: `H:\TEMP\nhl19-demitra\revision2`.

`scripts/blender/nhl19_demitra_diffuse_2k.py` samples the original CC5 4096-square
diffuse through the same recorded UV warp, at 2048 square. It retains the native
oral atlas regions, resampled from the donor image. This is not a 1K upscale.
`bake_report.json` records source and mapping fingerprints.

The package builder's `texture_only` recipe option skips mesh generation and
creates one archive replacement: `head_marian_gaborik_cm.gnf` in `game_90-008.big`.
The successful MCDs, patch archives, eyes, ears, neck and hidden hair are untouched.
BC1 format and material bindings remain; CC5 normal/PBR maps are not added.

The GNF uses 12 mip levels. Only the measured resolution-dependent header fields
(dimensions, pitch, mip count, total/data sizes) change from the 1K descriptor.
Production GNF detile, chunkzip and EB append checks are required before install.
Full-archive baseline fingerprints and installed texture payload comparison must
match revision 2. `installation.json` is authoritative for deployment status;
runtime success is not inherited from the successful 1K test.

Test NHL19 launch and Demitra's appearance by selecting Marian Gaborik on Slovakia.
Compare diffuse detail and mip behavior while keeping geometry fixed. Restore this
texture package before restoring revision 2 or older packages.

Installed 2026-09-11: installer completed successfully. All 12 production GNF mip
detile checks, chunkzip and single-row EB append validation passed. Live readback
matches the 2K package; all three live Gaborik MCD payloads still match successful
revision 2 exactly. Evidence: `installation.json` and `live_readback.json` in this
test folder. The subsequent user test confirmed success as recorded above.
