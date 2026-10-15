# tdb-speex — pinned libspeex for EA-Speex diagnostic previews

Decoder decision: coordinator review of receipt 27 (26 Sep 2026) — pinned upstream libspeex with a
small handwritten binding over a C shim compiled with `cc`; no libclang / bindgen requirement.
Preview only: no encoder is exposed and no audio is written anywhere.

## Upstream source

| Item | Value |
|---|---|
| Project | Speex (Xiph.Org Foundation) — libspeex |
| Release | `speex-1.2.1` (`SPEEX_VERSION` in `libspeex/arch.h`) |
| Tarball | `https://downloads.xiph.org/releases/speex/speex-1.2.1.tar.gz` |
| Tarball SHA-256 | `4b44d4f2b38a370a2d98a78329fefc56a0cf93d1c1be70029217baae6628feea` — equal to the entry in Xiph's published `https://downloads.xiph.org/releases/speex/SHA256SUMS.txt` (checked 26 Sep 2026) |
| Licence | BSD-3-Clause variant, `vendor/speex-1.2.1/COPYING` (kept verbatim with `AUTHORS`) |
| Vendored files | `vendor/speex-1.2.1/`: `COPYING`, `AUTHORS`, the 30 `.c` files of upstream's default `libspeex_la_SOURCES`, every `libspeex/*.h`, every `include/speex/*.h` — **byte-identical** to the tarball (checked file by file; per-file hashes in `vendor/SHA256SUMS-vendored.txt`) |
| Not vendored | the optional Vorbis psychoacoustic / FFT sources (`vorbis_psy.c`, `smallft.c`, `kiss_fft*.c`, `fftwrap.c`; off in upstream's default configure), the encoder test programs, build system and docs |

The `speex-sys` 0.4.0 crate's bundled copy was **not** used: it differs from the release tarball in
nine files (`cb_search.c`, `kiss_fft.c`, `lsp.c`, `nb_celp.c`, `sb_celp.c`, `modes.h`,
`cb_search_arm4.h`, `cb_search_bfin.h`, `quant_lsp_bfin.h`) and adds `fftwrap.c`. No MPL-covered
wrapper file from that crate is copied.

## Studio files (not upstream)

| File | Role |
|---|---|
| `shim/nhlms_speex_shim.{h,c}` | Opaque decoder handle owning `SpeexBits` and the decoder state; UWB init, `SPEEX_SET_ENH`, `SPEEX_GET_FRAME_SIZE`, one-packet decode with length checks (1..=2000 bytes, never reallocating or truncating the bit buffer), bits-remaining, free, version. |
| `shim/include-gen/os_support_custom.h` | Upstream's `OS_SUPPORT_CUSTOM` extension point: `speex_fatal` / `speex_assert` (upstream `exit(1)`) unwind by `longjmp` to the guarded shim call (C frames only) and poison the handle; warnings / notifications silenced. |
| `shim/include-gen/speex_config_types.h` | configure's `stdint` substitution of upstream `speex_config_types.h.in` (non-Windows only; `speex_types.h` defines the types itself on `_WIN32`). |
| `build.rs` | `cc` recipe below. |
| `src/lib.rs` | Hand-declared FFI for the shim's functions and the safe owner (`UwbDecoder`, `Drop`, named errors). |

## Build recipe

`cc` compiles the 30 upstream sources plus the shim with defines `FLOATING_POINT` (upstream
default), `USE_ALLOCA` (as upstream's Win32 configuration without SSE), `EXPORT=` and
`OS_SUPPORT_CUSTOM`; on MSVC also `inline=__inline`. No SSE / ARM / Blackfin code paths, no
upstream modification, no network access at build time, no libclang.

## Obligations for a binary that includes this crate

Reproduce `vendor/speex-1.2.1/COPYING` (copyright notice, conditions and disclaimer) in the
documentation or other materials provided with the distribution. No publication is authorised by
this package.
