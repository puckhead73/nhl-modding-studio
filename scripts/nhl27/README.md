# NHL 27 executable research tools

Research-only tooling; no game patcher is included. Python 3.12 with Capstone is used for executable tracing. Existing `scripts/nhl24` supplies SELF/indexing and Ghidra helpers. Private outputs belong under ignored `local/nhl27-eboot`. See the [dossier](../../docs/research/nhl27-backport-2026-10-15.md) for evidence and limitations.

Run from the repository root in PowerShell (substitute your Python executable if necessary):

```powershell
$py = 'C:\Users\puckh\AppData\Local\Programs\Python\Python312\python.exe'
$game = 'W:\Downloads\NHL 27\PPSA34063-app0'
$out = 'local/nhl27-eboot'
& $py scripts/nhl27/research.py index "$game/eboot.bin" $out
& $py scripts/nhl27/research.py survey $out local/nhl24-eboot/base local/nhl24-eboot/update
& $py scripts/nhl27/relocations.py $out
& $py scripts/nhl27/bundle_inventory.py $game Ps5 "$out/assets"
& $py scripts/nhl27/dbobject.py "$game/Data/layout.toc" "$out/layout-data.json"
& $py scripts/nhl27/asset_manifest.py $game Data/Ps5/globals.toc '.*' "$out/globals-manifests.json"
& $py scripts/nhl27/trace.py $out function 0x3bb8f60
& $py -m unittest discover -s scripts/nhl27 -p 'test_*.py'
```

`research.py index` verifies the ELF and creates a normalized analysis copy, identity record, string/reference/unwind index. It does not change the source. `survey` compares names to both prior NHL 24 indexes. `relocations.py` indexes RELA and can inspect a pointer table with additional VA/count arguments. `trace.py` provides disassembly, immediate/displacement searches, candidate callers and decompiler string annotation; candidate matches require manual control-flow confirmation.

`decompile.ps1 -Targets FILE` uses the locally installed Java/Ghidra project and existing NHL 24 TargetDecompile helper. It assumes the NHL27 project has been imported; it does not install Ghidra or perform whole-program analysis. Its paths reflect this workspace's existing tool installation.

`bundle_inventory.py` reads bounded names from both layers and retains occurrences. `dbobject.py` reads install metadata and summarizes opaque blobs by default. `asset_manifest.py` resolves the observed PS5 0x84/0x80/zero CAS span encoding and validates manifests. Other flags intentionally fail. This tool does not infer patch precedence or guarantee PS4 compatibility.

`compare_tweakables.py BASE ROOT24 ROOT27` tests conservative normalized-instruction matches against the existing NHL 24 catalog. Its result is not a patch recipe. `build_evidence.py ROOT27 ROOT24` combines the existing indexes and three asset inventories (NHL27 assets, nhl24-base-assets, nhl24-update-assets) into checklist/service/asset comparisons. Generate those inventories before running it.

`validate_slices.py EBOOT OUTPUT_JSON` executes two hash- and byte-guarded source-code slices with synthetic data on Windows x64 (BMI1 required). It validates only the team-classification predicate and signed clamp; it is not an emulator/game integration test. Do not run it against other builds or interpret passing results as feature portability.

The current workspace contains the private outputs and Ghidra project. They are deliberately excluded from source control. The scripts do not download tools, redistribute game code, edit installs or save files, or publish extracted assets.

## Compatibility follow-up

```powershell
& $py scripts/nhl27/compatibility.py "$out/compatibility"
& $py scripts/nhl27/riff_evidence.py "$out/compatibility"
& $py scripts/nhl27/type_evidence.py $out
& $py scripts/nhl27/container_probe.py "$out/compatibility"
& $py scripts/nhl27/ant_probe.py "$out/compatibility"
& $py scripts/nhl27/presentation_probe.py "$out/compatibility"
& $py scripts/nhl27/dependency_probe.py "$out/compatibility"
./scripts/nhl27/target_initfs.ps1
& $py scripts/nhl27/schema_compare.py "$out/compatibility"
& $py scripts/nhl27/target_field_names.py "$out/compatibility"
& $py scripts/nhl27/schema_compare.py "$out/compatibility"
```

`compatibility.py` uses explicit NHL 24 base/update and NHL 27 install paths declared in ROOTS. A reference from the NHL 24 update to layer zero resolves explicitly to the base install. It retains all sampled layer occurrences, compares **stored** hashes, verifies CAS framing, and uses the already installed `E:\Repositories\UEViewer\oo2core_8_win64.dll` to decode codec 0x11. The decoder's path and SHA-256 are recorded. It is not downloaded or redistributed. Decoded size is checked separately from the manifest's stored-stream SHA-1. Missing decoder/install files are errors, not silently substituted sources.

`riff_evidence.py` reads EBX/EBXS containers and EFIX type GUIDs/signatures/imports. Missing signatures remain unresolved. `type_evidence.py` extracts the source SharedTypeDescriptors from initfs and joins them to relocated executable names using exact GUID/signature pairs; field names additionally match descriptor hashes and offsets.

`target_initfs.ps1` verifies the NHL 24 1.60 executable hash, reads its loader's build-specific key/IV source into memory, and decrypts/validates both target initfs files into the private output directory. Key material is neither printed nor saved. `schema_compare.py` reads both target descriptor tables, including patch references to existing base signatures. `target_field_names.py` conservatively collects literal registrations from selected target constructors without executing them. Re-running schema comparison then includes those name correspondences; unrecognized fields remain unmapped. The scripts do not infer semantic identity from equal field offsets or equal object sizes.

`container_probe.py` only emits a conversion when dropping one extra zero EFIX word reproduces an existing target payload byte-for-byte. It rejects all other differences. These outputs are unchanged-feature controls, not new feature ports. `ant_probe.py` extracts the two target ANT state resources and records internal name matches without equating names with semantics. `presentation_probe.py` expands the investigation to selected intro/scoreboard/UI bundles and records unsupported cases explicitly.

`dependency_probe.py` checks imported partition/instance pairs against exported GUIDs in decoded samples. It retains all variants and distinguishes exact-instance matches from partition-only matches and references outside the sample. It does not interpret unsampled references as missing game assets or claim transitive dependency closure.

## Animation conversion stage

```powershell
& $py scripts/nhl27/animation_stage.py "$out/compatibility" --scan-all-af
```

`animation_dct.py RESOURCE OUTPUT_JSON` independently decodes the observed NHL27 DCT format to unbound motion samples. `animation_stage.py` additionally verifies source EBX schema, codec, resource ID and channel counts, exports float32 channels with precision measurements, resolves sampled binding candidates and checks target bank IDs. Without `--scan-all-af`, the dependency search is limited to rig/DOF/channel/layout names. Results retain the exact sampled source TOC rather than infer layer precedence. Outputs are intermediate motion data, not target-ready ANT assets. See [animation findings](../../docs/research/nhl27-animation-conversion-2026-10-15.md).
