#!/usr/bin/env bash
# Assemble the internal review bundle for the XMA2 helper built by build-helper.sh:
#   <BUNDLE>/helpers/nhlms-xma2-helper.exe
#   <BUNDLE>/helpers/nhlms-xma2-helper-notices/  (licences, build record, corresponding source)
# Run inside the isolated MSYS2 UCRT64 environment (msys-run.ps1). Nothing is published or uploaded.
set -euo pipefail

FFMPEG_VERSION=8.1.3
ROOT=${NHLMS_TOOLCHAIN_ROOT:-/c/nhlms-toolchain}
OUT=$ROOT/out/xma2-helper-ffmpeg-$FFMPEG_VERSION
BUNDLE=${1:-$ROOT/out/xma2-helper-bundle-$FFMPEG_VERSION}
HERE=$(cd "$(dirname "$0")" && pwd)
N=$BUNDLE/helpers/nhlms-xma2-helper-notices

[ -f "$OUT/nhlms-xma2-helper.exe" ] || { echo "build first: $OUT/nhlms-xma2-helper.exe missing" >&2; exit 2; }
rm -rf "$BUNDLE"
mkdir -p "$N/licenses/ffmpeg" "$N/licenses/mingw-w64-crt" "$N/licenses/mingw-w64-winpthreads" "$N/licenses/gcc-runtime" "$N/source" "$N/build"

cp "$OUT/nhlms-xma2-helper.exe" "$BUNDLE/helpers/"
# FFmpeg: the licence of this build (LGPL 2.1 or later) and the project's licence overview.
tar -xJf "$ROOT/downloads/ffmpeg-$FFMPEG_VERSION.tar.xz" -C "$N/licenses/ffmpeg" --strip-components=1 \
  "ffmpeg-$FFMPEG_VERSION/COPYING.LGPLv2.1" "ffmpeg-$FFMPEG_VERSION/LICENSE.md" "ffmpeg-$FFMPEG_VERSION/CREDITS"
# Statically linked runtime pieces from the toolchain (see README.md "Runtime code in the binary").
cp /ucrt64/share/licenses/crt/* "$N/licenses/mingw-w64-crt/"
cp /ucrt64/share/licenses/winpthreads/COPYING "$N/licenses/mingw-w64-winpthreads/"
cp /ucrt64/share/licenses/libgcc/COPYING.RUNTIME /ucrt64/share/licenses/libgcc/COPYING3 "$N/licenses/gcc-runtime/"
# Corresponding source: the unmodified release tarball and its detached signature (no patches).
cp "$ROOT/downloads/ffmpeg-$FFMPEG_VERSION.tar.xz" "$ROOT/downloads/ffmpeg-$FFMPEG_VERSION.tar.xz.asc" "$N/source/"
# How this exact binary was built.
cp "$HERE/build-helper.sh" "$HERE/msys-run.ps1" "$HERE/README.md" "$N/build/"
cp "$OUT/configure-args.txt" "$OUT/configure.log" "$OUT/config.h" "$OUT/config_components.h" "$OUT/toolchain.txt" "$N/build/"
cat > "$N/README.txt" <<EOF
nhlms-xma2-helper.exe is an unmodified build of the ffmpeg command-line program from FFmpeg
$FFMPEG_VERSION (https://ffmpeg.org), configured with only the XMA2 decoder, the WAV demuxer, the
pcm_s16le encoder / muxer and the pipe protocol. This software uses code of FFmpeg licensed under the
LGPLv2.1 (or later); see licenses/ffmpeg. FFmpeg is a trademark of Fabrice Bellard; this helper is not
an official FFmpeg release.

Corresponding source: source/ffmpeg-$FFMPEG_VERSION.tar.xz (signature source/*.asc, FFmpeg release
signing key FCF9 86EA 15E6 E293 A564 4F10 B432 2F04 D676 58D8). No patches were applied.
Build: build/build-helper.sh with build/configure-args.txt; compiler and packages in build/toolchain.txt.
Statically linked runtime code: mingw-w64 CRT (licenses/mingw-w64-crt), mingw-w64 winpthreads
(licenses/mingw-w64-winpthreads), GCC libgcc / libatomic under the GCC Runtime Library Exception
(licenses/gcc-runtime). The binary imports only Windows system DLLs (KERNEL32, SHELL32, bcrypt and the
Universal CRT api-ms-win-crt-* set).
EOF
( cd "$BUNDLE" && find helpers -type f -print0 | sort -z | xargs -0 sha256sum > MANIFEST.sha256 )
echo "bundle: $BUNDLE"
