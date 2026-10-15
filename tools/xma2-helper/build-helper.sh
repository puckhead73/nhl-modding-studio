#!/usr/bin/env bash
# Reproducible build of Studio's minimal XMA2 decoder helper (nhlms-xma2-helper.exe) from a pinned,
# signature-verified FFmpeg release. Run inside the isolated MSYS2 UCRT64 environment, e.g.
#   powershell -NoProfile -File tools\xma2-helper\msys-run.ps1 -Command "bash /e/Repositories/nhl-modding-studio/tools/xma2-helper/build-helper.sh"
# Inputs (see README.md): the release tarball + detached signature in $DOWNLOADS, verified beforehand
# with the FFmpeg release signing key FCF9 86EA 15E6 E293 A564 4F10 B432 2F04 D676 58D8.
# The build is LGPL-only: no --enable-gpl, --enable-version3 or --enable-nonfree; no network; no
# external libraries (--disable-autodetect); one decoder, one demuxer, one muxer, one encoder, one protocol.
set -euo pipefail

FFMPEG_VERSION=8.1.3
FFMPEG_SHA256=7138d28c96d9d3e3af4ee3d8cad72741f8ffb40da90c1112235dea3ecd3178a3
ROOT=${NHLMS_TOOLCHAIN_ROOT:-/c/nhlms-toolchain}
DOWNLOADS=$ROOT/downloads
BUILD=$ROOT/build/ffmpeg-$FFMPEG_VERSION
OUT=$ROOT/out/xma2-helper-ffmpeg-$FFMPEG_VERSION
TARBALL=$DOWNLOADS/ffmpeg-$FFMPEG_VERSION.tar.xz
# Fixed PE time stamp (the release signature's time) so identical inputs give identical bytes.
export SOURCE_DATE_EPOCH=1789951930

[ "$MSYSTEM" = UCRT64 ] || { echo "run under MSYSTEM=UCRT64 (got '$MSYSTEM')" >&2; exit 2; }
echo "$FFMPEG_SHA256 *$TARBALL" | sha256sum -c -

rm -rf "$BUILD" "$OUT"
mkdir -p "$ROOT/build" "$OUT"
tar -xJf "$TARBALL" -C "$ROOT/build"
cd "$BUILD"

CONFIGURE_ARGS=(
  --disable-everything --disable-autodetect --disable-network --disable-doc --disable-debug
  --disable-ffplay --disable-ffprobe --disable-avdevice --disable-swscale
  --enable-static --disable-shared --enable-small --enable-w32threads
  --enable-decoder=xma2 --enable-demuxer=wav --enable-muxer=pcm_s16le --enable-encoder=pcm_s16le
  --enable-protocol=pipe --enable-swresample
  --enable-filter=aresample,aformat,anull,abuffer,abuffersink
  --extra-ldflags="-static -Wl,--no-insert-timestamp"
  --extra-version=nhlms-xma2
)
printf '%s\n' "${CONFIGURE_ARGS[@]}" > "$OUT/configure-args.txt"
./configure "${CONFIGURE_ARGS[@]}" > "$OUT/configure.log" 2>&1 || { tail -30 "$OUT/configure.log"; exit 1; }
if grep -q "did not match anything" "$OUT/configure.log"; then grep "did not match" "$OUT/configure.log"; exit 1; fi
cp ffbuild/config.mak "$OUT/config.mak"
cp config_components.h "$OUT/config_components.h"
cp config.h "$OUT/config.h"
make -j"${NUMBER_OF_PROCESSORS:-8}" ffmpeg.exe > "$OUT/make.log" 2>&1 || { tail -30 "$OUT/make.log"; exit 1; }
cp ffmpeg.exe "$OUT/nhlms-xma2-helper.exe"

{
  echo "ffmpeg $FFMPEG_VERSION sha256 $FFMPEG_SHA256"
  gcc --version | head -1
  ld --version | head -1
  nasm --version | head -1
  make --version | head -1
  uname -a
  pacman -Q
} > "$OUT/toolchain.txt" 2>&1
( cd "$OUT" && sha256sum nhlms-xma2-helper.exe > nhlms-xma2-helper.exe.sha256 )
echo "built $OUT/nhlms-xma2-helper.exe"
