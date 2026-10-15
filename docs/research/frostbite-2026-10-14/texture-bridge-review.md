# NHL24 texture bridge archive review — 14 October 2026

Status: archive inventory, static executable-string inspection, supplied configuration,
patch XML, and log review. The executable was not launched. No Studio code or game files
were changed. Selected review files were extracted under `local/references/texture-bridge-review/`.

## Source identity

- Source: [nhl24-texture-bridge.rar](../../../Reference/New%20folder/nhl24-texture-bridge.rar).
- Archive size: 103,290,050 bytes.
- Archive SHA-256: `5792e8b9e8b048932255a2bd956fcad78231d1b68b94ef86a9194d93ba44c9e4`.
- Included `nhl24.exe`: 60,819,968 bytes.
- Executable SHA-256: `69e12d67c25a036697c56bad62188eaa14a04fc42cb4d87d59f748a2e724c07e`.

The archive is a portable emulator distribution with runtime DLLs and a populated `user`
directory. It includes shader/pipeline caches, configuration, logs, a crash dump, profile
save data, and a patch XML. These are runtime artifacts, not a standalone Studio asset
converter. The archive inventory contains no bridge source, README describing the bridge,
replacement-pack manifest, or DDS replacement payloads. Bundled user state is not a
recommended Studio configuration template; review extraction omitted saves and keys.

## What is established

### Build and supplied session

`user/log/shad_log.txt` lines 1–4 and 48 identify:

- NHL24 PC v0.8.1, shadPS4 v0.19.1 WIP.
- Branch `codex/texture-bridge`.
- Revision `7ce4ce8317b61583798cd2228e098ecbee8bdc0b`.
- Description `7ce4ce83-dirty`.
- Target session: NHL 24 PS4, `CUSA37933`, application version `01.60`.

The executable also contains the branch/revision strings. The dirty suffix means the
revision alone cannot reproduce this binary: the local changes/source snapshot are needed.
These logs document a supplied session, not an independently reproduced launch here.

### Replacement implementation evidence

Static ASCII strings in the executable include a source-path suffix
`src/video_core/texture_cache/texture_replacements.cpp` and these messages/tokens:

| Observed string | Supported inference / remaining limit |
|---|---|
| `SHADPS4_TEXTURE_REPLACEMENTS` | Likely environment-controlled activation or configuration. Accepted value/path syntax is not established. |
| `TextureReplacements` | Replacement-related identifier exists; not enough to define a configuration schema. |
| `Texture replacements: {} entries preloaded, {} MiB budget` | Preloading and budget accounting are represented in the binary. Default budget and scheduling are unknown. |
| `Texture replacement uploaded: {} -> {}x{} {} ({} mips)` | Upload diagnostics account for dimensions, format, and mip count. No successful upload was found in supplied logs. |
| `manifest version or title mismatch` | Manifest/title validation is represented. Manifest fields, version, and matching rules require source or a working pack. |
| `DDS payload length or duplicate key invalid` | DDS-size and duplicate-key checks are represented. Key construction and byte layout are not established. |
| `DDS arrays, volumes and cubemaps are unsupported` | Evidence of a deliberately restricted DDS path. Do not plan array-slice/volume/cubemap exports as accepted packs yet. |
| `texture exceeds preload budget` / `Texture replacement residency budget reached` | Both preload and residency limits are represented; enforcement and fallback behavior remain untested. |
| `Texture replacement rejected: {}` / `Texture pack rejected: {}` | Rejection diagnostics exist; recovery behavior remains untested. |

Strings establish implementation evidence in the binary, not control-flow verification,
correct validation, successful texture substitution, or visual/runtime qualification.
Do not infer supported BC formats, sRGB handling, hash algorithm, pack layout, hot reload,
asynchronous uploads, or high-resolution allocation behavior from these strings alone.

### Configuration and logs

The supplied `user/config.json` contains no texture replacement section or activation
setting. It records internal screen dimensions 1920x1080, window dimensions 3440x1440,
FSR and RCAS disabled, and anisotropic filtering 16. These settings do not demonstrate
internal-resolution scaling or a texture replacement session.

Searches of `shad_log.txt`, `shadps4.log`, and `nhl24_diag.log` found no replacement-pack
load, replacement upload, or replacement rejection receipt. The line reporting 1,194
preloaded pipelines is shader/pipeline-cache warmup, not evidence of texture replacements.
The bundled crash dump and error diagnostics cannot be attributed to the bridge from
this review. No crash dump analysis was performed.

### Executable patch context

`user/patches/nhl24.xml` supplies version-specific synchronous replay-compression patches
for application versions 01.00 and 01.60. Its comments describe bypassing an asynchronous
replay job path that can stall simulation. The patch metadata is enabled, but presence
does not prove application in the supplied session. This is a separate emulator/game
compatibility intervention, not a texture replacement or tunables schema.

## Change to our understanding

The earlier [graphics research](../../../Reference/New%20folder/RESEARCH.md) proposed a
host-side texture replacement loader. We now have a supplied emulator binary containing
replacement implementation evidence. Update its status from **proposal only** to
**binary prototype supplied; interface and runtime behavior unqualified**.

This does not qualify Frostbite archive writing, NHL22/23 support, PS5 deployment,
high-resolution rendering, or the [gameplay tweakables catalogue](gameplay-tweakables-review.md).
The catalogue's title/build and storage provenance remain unresolved.

## Proposed Studio integration work

1. Obtain/locate the exact dirty source snapshot or an interface specification and a known
   working replacement pack. Establish activation syntax, manifest schema/version, title
   identity, content-key algorithm/input bytes, DDS constraints, and budget semantics.
2. Define a versioned **emulator texture-pack export target**, separate from native CAS/TOC
   build/deploy. Studio should prepare packs while the emulator owns runtime matching,
   allocation, upload, and fallback. Do not copy bundled user profiles/configuration.
3. Connect export to the planned effective Data/Patch asset index and verified RES/chunk
   mapping. Preserve provenance, full mip chains, material role, and color-space semantics.
   A name match alone is insufficient for a cross-title replacement.
4. Qualify a minimal ordinary 2D replacement pack: unchanged control, visibly changed
   texture, multi-mip texture, and a larger-dimension candidate. Capture loader/upload
   receipts and screenshots, then test title mismatch, malformed DDS, duplicate keys,
   exhausted budget, and fallback. Confirm performance on a healthy baseline session.
5. Expand formats and cross-title transfer only after those fixtures pass. Keep arrays,
   cubemaps, volumes, partial streaming, and resolution scaling as separate qualification
   work. Reconcile kit embroidery-array workflows with the loader's stated restriction.

This is documentation ingestion only. The user's roadmap-review checkpoint remains in
effect; no pack exporter or emulator integration was implemented during this review.
