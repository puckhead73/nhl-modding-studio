# XMA2 decoder helper (`nhlms-xma2-helper.exe`)

Studio previews Xbox 360 EA-XMA audio by wrapping it as a RIFF XMA2 file (`tdb-snd::eaxma`) and
piping it through a separately built helper program (`tdb-snd::helper`). The helper is the `ffmpeg`
command-line program from an unmodified FFmpeg release, configured down to exactly what
`helper::XMA2_ARGS` needs. Studio never runs a shell or searches `PATH`: it runs the file named by
`NHL_STUDIO_XMA2_HELPER`, else `helpers\nhlms-xma2-helper.exe` beside `nhl-studio.exe`. Without a
helper, XMA entries stay listed and framed and say why there is no preview.

Status (27 Sep 2026): **built and developer-tested; internal review only.** Nothing is published. The
distribution arrangement for a release (hosting of the corresponding source, about-box / EULA wording,
Studio's own licence) is not decided here — see "Release obligations".

## Pinned inputs

| Input | Identity |
|---|---|
| FFmpeg release | `ffmpeg-8.1.3.tar.xz` from `https://ffmpeg.org/releases/`, SHA-256 `7138d28c96d9d3e3af4ee3d8cad72741f8ffb40da90c1112235dea3ecd3178a3`; detached signature `ffmpeg-8.1.3.tar.xz.asc` verified "Good signature" with the FFmpeg release signing key, primary fingerprint `FCF9 86EA 15E6 E293 A564 4F10 B432 2F04 D676 58D8` (the fingerprint published on `https://ffmpeg.org/download.html`) |
| Patches | none |
| Toolchain | MSYS2 base `msys2-base-x86_64-20260611.tar.xz` from `https://repo.msys2.org/distrib/x86_64/`, SHA-256 `a2d047e8ee213c3c6a49a8de427eb1069df12207c0422ff1b3cbb5c905c34221`, signature verified with the MSYS2 installer key, primary fingerprint `0EBF 782C 5D53 F7E5 FB02 A667 46BD 761F 7A49 B0EC` (as published on `https://www.msys2.org/docs/installer/`); packages then installed with `pacman` (packages verified by pacman against the MSYS2 keyring) |
| Packages used | `mingw-w64-ucrt-x86_64-gcc 16.2.0-4`, `mingw-w64-ucrt-x86_64-binutils 2.47-3`, `mingw-w64-ucrt-x86_64-crt 14.0.0.r426.g4564ee4b5-1`, `mingw-w64-ucrt-x86_64-winpthreads` / `libwinpthread 14.0.0.r426.g4564ee4b5-1`, `mingw-w64-ucrt-x86_64-nasm 3.02-1`, `make 4.4.1-3`, `diffutils`; the full package list of the build environment is written to `toolchain.txt` by the build |

## Installed toolchain (a kept build dependency, not scratch)

- Location: `C:\nhlms-toolchain\` — `msys64\` (the MSYS2 installation), `downloads\` (the verified
  inputs above), `gnupg\` (the keyring used to verify them), `build\` (the extracted build tree) and
  `out\` (build outputs and the helper bundle).
- No machine or user `PATH` change, no service, no registry entry, no Start-menu entry, no restart:
  the base archive was extracted, not run as an installer. `msys-run.ps1` starts the MSYS2 shell with a
  process-local minimal environment (`MSYSTEM=UCRT64`, `PATH` = System32 only on entry).
- **Removal:** delete `C:\nhlms-toolchain\` (nothing else was installed). Keep a copy of the helper
  bundle first if a review build still uses it.

## Build

```powershell
# one-time: extract the verified MSYS2 base to C:\nhlms-toolchain\msys64, then
powershell -NoProfile -File tools\xma2-helper\msys-run.ps1 -Command "pacman -Syuu --noconfirm"
powershell -NoProfile -File tools\xma2-helper\msys-run.ps1 -Command "pacman -S --needed --noconfirm make diffutils mingw-w64-ucrt-x86_64-gcc mingw-w64-ucrt-x86_64-nasm"
# each build (checks the tarball hash, configures, builds ffmpeg.exe only, records the toolchain):
powershell -NoProfile -File tools\xma2-helper\msys-run.ps1 -Command "bash /e/Repositories/nhl-modding-studio/tools/xma2-helper/build-helper.sh"
# assemble helpers\ + notices + corresponding source:
powershell -NoProfile -File tools\xma2-helper\msys-run.ps1 -Command "bash /e/Repositories/nhl-modding-studio/tools/xma2-helper/package-helper.sh"
```

Configure line (`build-helper.sh`; LGPL only — no `--enable-gpl`, `--enable-version3` or
`--enable-nonfree`; the script fails if any `--enable-*` name matches nothing):

```
--disable-everything --disable-autodetect --disable-network --disable-doc --disable-debug
--disable-ffplay --disable-ffprobe --disable-avdevice --disable-swscale
--enable-static --disable-shared --enable-small --enable-w32threads
--enable-decoder=xma2 --enable-demuxer=wav --enable-muxer=pcm_s16le --enable-encoder=pcm_s16le
--enable-protocol=pipe --enable-swresample
--enable-filter=aresample,aformat,anull,abuffer,abuffersink
--extra-ldflags=-static --extra-version=nhlms-xma2
```

Differences from receipt 30's proposed recipe, found by building it: `--disable-postproc` is not an
option in FFmpeg 8.x (dropped); `--enable-w32threads` is needed because `--disable-autodetect` would
otherwise leave the `ffmpeg` program without a threading backend; `--extra-ldflags=-static` keeps the
GCC / winpthreads runtime inside the executable instead of beside it. The muxer's configure name is
`pcm_s16le` (its format name is `s16le`, which is what `XMA2_ARGS` passes to `-f`).

## What the built binary contains (its own inspection commands)

`nhlms-xma2-helper.exe -L` reports LGPL 2.1 or later; `-version` prints the configure line above;
`-decoders` lists `xma2` and `wmapro` (the XMA2 decoder is implemented inside the WMA Pro decoder and
configure selects it); `-encoders` `pcm_s16le`; `-demuxers` `wav`; `-muxers` `s16le`; `-protocols`
`pipe` only; `-filters` the five requested plus `atrim`, `crop`, `format`, `hflip`, `null`, `rotate`,
`transpose`, `trim`, `vflip`, `buffer`, `buffersink` (forced by the `ffmpeg` program's own
`ffmpeg_select`; audio-only input never instantiates the video ones); no hardware acceleration, no
devices, no parsers, no bitstream filters, no network. `config.h`: `CONFIG_GPL 0`, `CONFIG_VERSION3 0`,
`CONFIG_NONFREE 0`, `CONFIG_NETWORK 0`.

Imports (`objdump -p`): `KERNEL32.dll`, `SHELL32.dll`, `bcrypt.dll` and the Universal CRT
`api-ms-win-crt-*` set — all Windows system components on Windows 10 and later. No compiler DLL is
needed beside it.

### Runtime code in the binary

Statically linked from the toolchain: the mingw-w64 CRT start-up code (ZPL 2.1 with public-domain
parts), mingw-w64 winpthreads (MIT-style), GCC `libgcc` / `libatomic` (GPLv3 **with the GCC Runtime
Library Exception**, which permits this use). Their licence texts are copied into the bundle.

## Release obligations (identified, not decided)

From FFmpeg's licence-compliance checklist (`https://ffmpeg.org/legal.html`) applied to this build:
the build is LGPL-only and uses no external library; the corresponding source (the unmodified
signed tarball), the configure line and "no patches" are kept with the binary. Open before any
release: hosting the corresponding source next to the distributed binary (or an equivalent written
offer), the "uses code of FFmpeg licensed under the LGPLv2.1" wording in Studio's about box / EULA /
download page, EULA terms compatible with LGPL (no reverse-engineering prohibition for this part),
whether to keep the file name `nhlms-xma2-helper.exe` for an `ffmpeg` build (FFmpeg's checklist
discourages obfuscated renames of its DLLs; a name that shows the origin may be preferable), and
Studio's own licence. The subprocess boundary does not by itself settle these.
