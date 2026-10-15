# Draft — download-page attribution and licence audit for the release archive

**Draft for the user / coordinator; not published and not a legal review.** First written 27 Sep 2026 (receipt 43),
revised 28 Sep 2026 (receipt 46). This is wording for an eventual download page, based on the actual arrangement:
one self-contained zip archive holding Studio, its `defs`, the XMA2 helper with its exact corresponding source,
licences, notices and build record, the third-party notices and a hash manifest ([beta
decisions](../plans/beta-20261001-scope-decisions.md) 3). Studio's own licence is MIT (Q46e); no EULA is invented.

**Receipt 46 corrections to the 27 Sep draft:** the page no longer says "everything is included" or "nothing else to
install" — the WebView2 Runtime is a real prerequisite on some Windows 10 PCs (§1, §4). Studio's licence is no
longer "to be supplied": it is MIT, with only the copyright line pending. The embedded artwork is disclosed (§5).

## 1. Proposed download-page text (fill the bracketed parts at publication)

> **NHL Modding Studio [version] for Windows** — [archive name].zip ([size], SHA-256 `[archive hash]`)
>
> **Requirements:** 64-bit Windows 10 or 11 with the Microsoft Edge WebView2 Runtime. WebView2 is part of
> Windows 11 and is already installed on the vast majority of Windows 10 PCs. If Studio's window does not open,
> install the Evergreen WebView2 Runtime from
> [Microsoft](https://developer.microsoft.com/microsoft-edge/webview2/) and try again.
>
> Extract the archive anywhere and run `nhl-studio.exe` from the extracted folder. Studio's definitions and the
> helper for Xbox 360 audio previews are inside the archive; there is no installer and no other download.
>
> **Licence.** NHL Modding Studio's own source code is released under the MIT License (`LICENSE` in the
> archive). Third-party components keep their own licences, listed below and in `THIRD-PARTY-NOTICES.txt`.
> The app shows the same texts under Settings → About & Licences.
>
> **Third-party software.** This download includes an unmodified build of **FFmpeg 8.1.3**
> ([ffmpeg.org](https://ffmpeg.org)), used only as a separate helper program
> (`helpers\nhlms-xma2-helper.exe`) to decode Xbox 360 XMA audio for previews. This software uses code of
> FFmpeg licensed under the
> [LGPLv2.1](https://www.gnu.org/licenses/old-licenses/lgpl-2.1.html). The helper's complete corresponding
> source (the official FFmpeg 8.1.3 release tarball and its signature), its build scripts and configuration,
> and the licences of the runtime code linked into it are inside the archive, in
> `helpers\nhlms-xma2-helper-notices\`. FFmpeg is a trademark of Fabrice Bellard, originator of the FFmpeg
> project. The helper is not an official FFmpeg release, and the FFmpeg project does not endorse NHL
> Modding Studio.
>
> Studio also includes libspeex 1.2.1 (BSD-3-Clause), the Microsoft WebView2 SDK loader, and open-source Rust
> and JavaScript libraries. Each is listed with its licence in `THIRD-PARTY-NOTICES.txt` inside the archive.
>
> NHL Modding Studio is a fan-made tool, not affiliated with or endorsed by EA or the NHL. It includes no EA
> game data and edits files from copies of the games that you own. [Artwork statement — depends on §5.]

## 2. What the archive does to meet the FFmpeg obligations (as built)

| Item (ffmpeg.org/legal checklist, read 27 Sep, and the LGPL 2.1 text) | This archive | Status |
|---|---|---|
| Compile without `--enable-gpl` / `--enable-nonfree` | `configure-args.txt`; the helper's `-L` reports LGPL 2.1+; `CONFIG_GPL / VERSION3 / NONFREE = 0` (receipt 32; re-read 28 Sep in the packaged `config.h`) | done |
| Distribute the source of the FFmpeg you use, including modifications | the exact 8.1.3 release tarball + signature in the same archive (no patches applied) | done (same-archive delivery, as the user chose) |
| State the FFmpeg version, configuration and build scripts | `build\configure-args.txt`, `build-helper.sh`, `msys-run.ps1`, `toolchain.txt`, `config.h`, `configure.log` | done |
| Licence text with the binary | `licenses\ffmpeg\COPYING.LGPLv2.1`, `LICENSE.md`, `CREDITS` | done |
| Attribution ("This software uses code of FFmpeg …") in an accessible place | `THIRD-PARTY-NOTICES.txt`; helper `README.txt`; **in-app Settings → About & Licences** (receipt 46); page §1 | done (the About page is automated-tested, not desktop-observed) |
| Notices of runtime code linked into the helper | see §3 | done |
| No misrepresentation of origin / no trademark use as endorsement | "not an official FFmpeg release", trademark sentence | done |
| Library-specific items (dynamic linking, DLL names, relinking) | the helper is a statically linked **executable** that Studio runs as a separate process; Studio links no FFmpeg library. Every part linked into the helper is either FFmpeg (LGPL, full source shipped) or runtime code under its own terms (§3) | reviewed; the helper name is kept, its FFmpeg origin disclosed wherever it appears |
| EULA must not forbid reverse engineering of the LGPL part | no EULA exists; nothing in the archive restricts the helper | no conflict found |

This records how the archive was put together. It is not a claim of complete legal compliance.

## 3. What is linked into the helper (checked 28 Sep, receipt 46)

Read from the packaged `config.h`, `configure-args.txt`, `toolchain.txt`, the MSYS2 package records and the
helper's own import table (`tools/release-archive/pe_info.py`):

| Component | How | Terms | In the archive |
|---|---|---|---|
| FFmpeg 8.1.3 (libavcodec / format / util / swresample / filter, fftools) | static; only the XMA2 decoder, WAV demuxer, pcm_s16le, pipe and five audio filters | LGPL-2.1-or-later (GPL, version 3, non-free off) | source tarball + signature, configuration, scripts, licence texts |
| External libraries (zlib, bzlib, lzma, iconv, TLS, SDL, hardware acceleration) | **not linked**: `--disable-autodetect`, each `CONFIG_* 0` | — | — |
| mingw-w64 CRT (incl. mingwex) and winpthreads 14.0.0.r426 | static (`-static`) | mingw-w64 runtime licence (permissive / public domain parts) | `licenses\mingw-w64-crt`, `licenses\mingw-w64-winpthreads` |
| GCC 16.2.0 libgcc / libatomic | static | GPL-3 with the GCC Runtime Library Exception | `licenses\gcc-runtime` |
| mingw-w64 default application manifest (`default-manifest.o`, package 20260815-1) | linked by GCC; the helper carries a manifest resource | the MSYS2 package declares "custom:Public Domain" | named in the notices |
| Windows system libraries (KERNEL32, SHELL32, bcrypt, the Universal CRT) | dynamic imports | Windows components | not shipped |

libstdc++ and libquadmath are installed in the toolchain but not used (FFmpeg is C; the import table and
configuration show no C++ runtime).

## 4. Runtime prerequisites (checked 28 Sep, receipt 46)

- **`nhl-studio.exe`** imports only Windows system libraries and the Universal CRT (`api-ms-win-crt-*`, part of
  Windows 10 / 11). There is no Visual C++ redistributable dependency: the Rust build links the C runtime
  statically. The WebView2 loader is statically linked (the WebView2 SDK 1.0.3650.58 static library, byte-identical
  to the NuGet package's). The browser engine itself — the WebView2 Runtime — is not included.
- **WebView2 Runtime:** Microsoft's distribution guide (read 28 Sep 2026) states that the Evergreen Runtime is
  included as part of Windows 11 and that "a small number of Windows 10 devices don't have the WebView2 Runtime
  installed". It recommends redirecting such users to Microsoft's download page, which §1 does. The archive has no
  installer, so it cannot install the Runtime itself, and Studio does not yet check for it at startup. What happens
  when it is missing has **not been observed** (no launch was made).
- **Not checked:** Windows versions older than 10, ARM64 Windows (the executable is x64), and machines whose
  WebView2 updates are disabled by policy.

## 5. Artwork embedded in the executable (disclosed 28 Sep, receipt 46; not decided here)

A sourcemap build of the embedded frontend (identical code to the bundle `tauri build` embeds) shows these
non-code files inside `nhl-studio.exe`:

- 17 title-picker tiles: **game box art of the EA NHL titles**, user-edited to a matched set (commit `086e0f0`);
- 3 platform tiles (Xbox 360, PS3, PS4 artwork, user-edited);
- 30 line-strategy GIFs that appear to be captured from an EA NHL game's strategy screens;
- the Studio header logo, the rink backdrop and the application icon (origin not recorded).

Their rights are **unrecorded** (`tools/release-archive/embedded-resources.json`). Studio's MIT licence does not
cover them, and the notices say so. The assembler keeps any candidate with unrecorded artwork from being marked
release-ready. **A user / coordinator decision is needed:** record the rights of each group (for example
first-party artwork), or replace or remove the EA-derived images before a public release. Nothing was changed here.

## 6. Licence audit status (28 Sep 2026)

| Item | Status |
|---|---|
| Studio licence (Q46e MIT) | package metadata `MIT` across the workspace (`tdb-speex`: `MIT AND BSD-3-Clause`), README updated, SPDX 3.29.0 MIT text staged. **Copyright holder not recorded**: `LICENSE` withheld from archives (never a placeholder); `tools/release-archive/studio-licence.json` holds the one missing value |
| In-app About / notices | Settings → About & Licences, and a link on the start screen; reads the packaged files offline; names any missing file |
| 15 packages without packaged licence files (receipt 43) | 4 npm packages (`@mediapipe/tasks-vision`, `draco3d`, `maath`, `stats-gl`) are **not in the bundle** (measured), so they are no longer listed. The other 11 have texts retrieved from their exact published upstream (`supplemental-licenses/PROVENANCE.json`). `selectors` (MPL-2.0) has no upstream licence file, so the authoritative MPL-2.0 text and the source location are given |
| Frontend closure | measured from a sourcemap build equal to the embedded bundle: 20 npm packages, plus Tailwind CSS (its MIT banner in the stylesheet), instead of the 95-package non-dev superset |
| WebView2 SDK loader (statically linked; missed in receipt 43) | SDK licence and NOTICE included |
| Release gate | `assemble.py` records `release_ready` and plain-language blockers; `verify.py` refuses inconsistent status, broken notice links and missing licence folders; `check --release` refuses a not-ready candidate |
| Remaining before release | the copyright holder; the artwork decision (§5); final distribution review of this table; publisher authenticity (the archive is not code-signed — paid Authenticode is deferred by Q46) |
