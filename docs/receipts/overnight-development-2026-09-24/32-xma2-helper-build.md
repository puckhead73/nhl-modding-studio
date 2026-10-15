# Receipt 32 — the XMA2 helper is built: pinned LGPL FFmpeg 8.1.3, reproducible, 83 / 83 vectors bit-identical

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing listened to; nothing
game-verified; nothing published.** HEAD `8d60a6d582c1bf208e0a5bc590b28d5add7d5051` (unchanged; nothing
staged or committed). Claim `dev-xma2-helper-build`. Direction:
[overnight prompt after receipt 31](../../plans/development-20260927-overnight-after-receipt31.md) §1, under the
user's toolchain approval quoted there. Supersedes receipt 30's "helper build blocked".

## 1. Plain-language summary

- Studio now has its own XMA2 helper: `nhlms-xma2-helper.exe`, the `ffmpeg` program from the official
  FFmpeg 8.1.3 release, unmodified, configured down to one decoder, one input format, one output format
  and pipes only. Its own licence report says **LGPL 2.1 or later**; no GPL, "version 3" or non-free
  parts, no network, no hardware or device code.
- It runs on a plain Windows machine: it needs only Windows' own DLLs, nothing from the compiler.
- On all 83 research samples it produces **exactly** the declared number of samples and PCM that is
  **bit-identical** to the research decode. The test now fails on any PCM difference (no tolerance).
- Building it twice from the same inputs gives the same bytes (SHA-256 `dc3533ed…`).
- To build it I installed a free toolchain (MSYS2 with GCC 16.2) in its own folder,
  `C:\nhlms-toolchain\`, after checking the download's signature. Nothing was added to the system PATH,
  no service or restart. Removing that folder removes it completely.
- The internal review bundle keeps the helper with its licences, the exact FFmpeg source it was built
  from and the build record. Publishing it still has open obligations (§6); nothing was published.

## 2. Files

| Path | Change |
|---|---|
| `tools/xma2-helper/README.md` (**new**) | pinned inputs and their verification, toolchain location / removal, build commands, configure line, what the binary contains, runtime code linked in, release obligations |
| `tools/xma2-helper/msys-run.ps1` (**new**) | runs one command in the isolated MSYS2 UCRT64 shell with a process-local minimal environment |
| `tools/xma2-helper/build-helper.sh` (**new**) | checks the tarball hash, extracts fresh, configures (fails on any unmatched component name), builds `ffmpeg.exe` only, records config / toolchain / hash; fixed `SOURCE_DATE_EPOCH` for a reproducible PE header |
| `tools/xma2-helper/package-helper.sh` (**new**) | assembles `helpers\nhlms-xma2-helper.exe` + `helpers\nhlms-xma2-helper-notices\` (licences, corresponding source + signature, build record, `README.txt`) + `MANIFEST.sha256` |
| `crates/tdb-snd/tests/ea_xma_real_vectors.rs` | PCM must equal the research hash (was: reported only); prints the helper's SHA-256; adds the executable-adjacent `helpers\` route with the variable removed, and the missing-helper message |
| `crates/tdb-snd/tests/helper_runner.rs`, `src/bin/nhlms-helper-stub.rs` | cancellation raised **while** the helper runs and a timeout, each followed by a check that the helper process is gone (with a positive control); an unusable (non-executable) helper file refuses by name; stub mode `pidsleep` (test-only, never shipped) |

## 3. Toolchain and helper provenance

| Item | Identity |
|---|---|
| MSYS2 base | `msys2-base-x86_64-20260611.tar.xz`, SHA-256 `a2d047e8…c34221`, signature Good, primary key `0EBF 782C 5D53 F7E5 FB02 A667 46BD 761F 7A49 B0EC` (msys2.org installer guide); extracted to `C:\nhlms-toolchain\msys64` (≈ 0.9 GB), keyring initialised, `pacman -Syuu` |
| Packages | `mingw-w64-ucrt-x86_64-gcc 16.2.0-4`, `binutils 2.47-3`, `crt` / `winpthreads 14.0.0.r426.g4564ee4b5-1`, `nasm 3.02-1`, `make 4.4.1-3`, `diffutils` (full list in the bundle's `toolchain.txt`) |
| FFmpeg source | `ffmpeg-8.1.3.tar.xz`, SHA-256 `7138d28c96d9d3e3af4ee3d8cad72741f8ffb40da90c1112235dea3ecd3178a3`, signature Good, primary key `FCF9 86EA 15E6 E293 A564 4F10 B432 2F04 D676 58D8` (ffmpeg.org download page); no patches |
| Helper | `C:\nhlms-toolchain\out\xma2-helper-bundle-8.1.3\helpers\nhlms-xma2-helper.exe`, 1 510 912 bytes, SHA-256 `dc3533ed3234782703ebeab72034d9298b920b3e1fbb7c3fb9a9d2adfd818c21`; two consecutive clean builds identical |

Receipt 30's recipe needed three corrections, found by building it: `--disable-postproc` no longer exists
in FFmpeg 8; `--enable-w32threads` is required because `--disable-autodetect` otherwise leaves the program
without threads; `--extra-ldflags=-static` keeps the GCC / winpthreads runtime inside the executable. A
first attempt with the muxer written `s16le` built a helper with **no** muxer (configure only warns); the
script now fails on "did not match anything". Reproducibility needed a fixed `SOURCE_DATE_EPOCH` (the
three differing bytes were the PE time stamp and checksum).

## 4. Evidence (`docs/validation/evidence/overnight-development-2026-09-24/logs/`)

- `r32-helper-inspection.log` (run from an empty directory with `env -i`, `PATH` = System32 only):
  `-L` LGPL 2.1+; `-version` shows the configure line; decoders `xma2`, `wmapro` (XMA2 lives in the WMA Pro
  decoder; configure adds it); encoder `pcm_s16le`; demuxer `wav`; muxer `s16le`; protocols `pipe` only;
  filters = the five requested + the ones the `ffmpeg` program itself forces (`atrim`, `crop`, `format`,
  `hflip`, `null`, `rotate`, `transpose`, `trim`, `vflip`, `buffer`, `buffersink`); no bitstream filters,
  parsers, devices or hardware acceleration. Imports: `KERNEL32`, `SHELL32`, `bcrypt`, `api-ms-win-crt-*`.
  `config.h`: `CONFIG_GPL 0`, `CONFIG_VERSION3 0`, `CONFIG_NONFREE 0`, `CONFIG_NETWORK 0`.
- `r32-xma-helper-vectors.log` (`--ignored`, read-only X360 trees, helper named by its bundle path):
  **83 / 83** R84 vectors — source hash = research, decoded samples per channel = declared, **PCM SHA-256 =
  the research oracle's on every vector** (R84 used FFmpeg 8.1.1; no difference appeared with 8.1.3). The
  known unavailable vector (NHL 11 music at offset 298 517 360: beyond the loose stream, no research hash)
  is reported and skipped as R84 did. The same binary copied to `<dir>\helpers\` and found by `locate` with
  the variable removed decodes the first vector to the same PCM; a directory without one gives the
  missing-helper message.
- `r32-helper-runner.log`: `helper_runner` 5 / 5 — 8 MiB echo (no pipe deadlock), failing exit with its
  stderr, stderr flood beside a result, stdout flood stopped at the cap, 0.5 s timeout, pre-set
  cancellation, **cancellation raised 200 ms after the helper started and an 0.8 s timeout, each leaving
  no helper process** (checked with `tasklist`; positive control on the test process), missing file,
  non-executable file ("the decoder helper could not start"), `locate` without `PATH`.
- `r32-xma-domain.log`: `studio-domain` `gen7_sfx_banks` 6 / 6 with `--include-ignored` and this helper
  (X360 NHL 15 `000_goal.abk` record 0 previews at the declared count; without a helper the same entry
  refuses with the missing-helper text).
- Not run: R92 `.ast` XMA and R100 NHL 09 vectors — their Studio routes are not admitted (framing /
  source contracts, package 2); decoding them would not admit a listing path.

## 5. Limits

- Decoder agreement is not listening: card A-XMA-2 waits for a review build that contains this helper.
- The helper is a build output under `C:\nhlms-toolchain\out\`; Studio finds it only through the
  variable or a `helpers\` folder beside `nhl-studio.exe` (the review bundle, package 5). No installer
  integration (`tauri.conf.json` resources) was changed.

## 6. Release obligations identified (not decided; nothing published)

From FFmpeg's LGPL checklist (`ffmpeg.org/legal.html`) for this build: the corresponding source, configure
line and "no patches" are kept with the binary. Open for a release: hosting the source beside the
distributed binary (or a written offer); the "uses code of FFmpeg under the LGPLv2.1" wording in the about
box / EULA / download page; EULA terms compatible with the LGPL; whether an `ffmpeg` build should keep the
name `nhlms-xma2-helper.exe` (FFmpeg discourages obfuscating renames; a name showing the origin may be
preferable — a one-constant change, `XMA2_HELPER_FILE`); the statically linked mingw-w64 CRT / winpthreads
notices and the GCC Runtime Library Exception (texts in the bundle); Studio's own licence. The subprocess
boundary alone does not settle these.

## 7. Retained artifacts and cleanup

- **Kept (build dependency):** `C:\nhlms-toolchain\` — `msys64\` (toolchain), `downloads\` (verified
  inputs), `gnupg\` (verification keyring), `build\ffmpeg-8.1.3` (≈ 114 MB, the tree the helper was built
  from; the script recreates it, so it may be deleted), `out\` (build record + the helper bundle used by
  the next review build). Removal: delete `C:\nhlms-toolchain\`.
- Deleted: the three scratch comparison binaries and the patch scripts in the session scratchpad; test
  temp roots (`nhlms-helper-cancel-*`, `nhlms-helper-unusable-*`, `nhlms-xma-helpers-route-*`) are removed
  by their tests. No process left running.

## 8. Next

Package 2: wire the SNS EA-XMA route into the X360 speech / music listings, per measured tuple, with the
built helper.
