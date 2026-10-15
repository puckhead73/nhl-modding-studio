# NHL19 scalp correction: mesh-only isolation test

Prepared at the user's request after rollback of the combined scalp/2K revision
restored successful NHL19 startup. No installation or game launch in this step.

Package: `H:\TEMP\nhl19-mogilny-scalp-mesh-only`.
Builder: `scripts/nhl19_prepare_mesh_only_test.py`.

The package contains the exact MCD payloads from the failed combined revision,
without rebuilding or adjusting them. Only the Eichel MCD row in each of
`game_90-008.big`, `patch_game_91.big`, and `patch_game_93.big` is retargeted.
The base archive size is recalculated to end after its MCD append; there is no
texture append and no texture row modification. Thus this retains the scalp
lowering, native lower attachment, companion eyes/lashes and inset hair changes.

The current successful 1024 diffuse remains byte-identical in its existing archive
entry. A GNF/mip copy in the test folder is solely an offline verification reference.
Its live production extraction matches SHA256
`43cf0811adb6dc5b3a6c36225f757fde823771e091e29c248b25abad92cb7dbe`.

Production checks passed for all three chunkzip/MCD payloads, including every
native position, triangle and normal; all 11 reference texture mips passed.
Production EB virtual-append checks passed with unrelated entries unchanged.
The installer's read-only `verify` completed successfully against all three live
archive full fingerprints. It repeats this preflight before any installation write.

With shadPS4 closed, `Manage-Test.ps1 apply` installs the test; `restore` returns
to the previous successful Mogilny package. Both working/test folders must remain.
First test launch, then inspect the helmet/collar fit, eyes, lips and mouthbag.
Successful launch would separate this mesh revision from the combined failure;
it would implicate the texture/package changes without proving a specific GNF bug.
If launch still fails, restore this mesh-only package before further isolation.

## User-run installation and crash timing

The user reported a crash after starting installation. The first inspection saw
an `applying` receipt with only the base archive recorded; a subsequent inspection
found `installed`, with all three installed full hashes. Receipt modification time
was 2026-09-10 13:43:39 local, after the newest Windows crash event at 13:41:37
(Qt6Core access violation in `C:\shadPS4\shadPS4.exe`). Therefore that launch is
not recorded as a failure of a completed mesh-only installation. The initial
assessment of an interrupted installer was superseded by its completed receipt.

Production extraction after completion verified all three live MCD payloads
against the mesh-only manifest, and the live diffuse against the exact successful
1024 texture hash. Results: `live_readback.json`. No rollback performed in this
inspection; the completed mesh-only package remains installed for a fresh launch.

The user subsequently confirmed: **"Mesh tweak successful, now the texture"**.
The scalp/neck revision is therefore user-confirmed in game with the original
working diffuse. Next change is isolated to the supplied 2K diffuse.
