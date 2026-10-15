# Decoder recommendation — EA-Speex (codec `0x09`) and EA-XMA (codec `0x03`) previews

Developer, 26 September 2026 (review of packages 23–26, task 3). A recommendation for the coordinator's decision;
**nothing is added to Studio** and no dependency is accepted by this document. No audio writer is proposed.
Evidence: [`t3-decoder-evaluation.log`](evidence/overnight-development-2026-09-24/logs/t3-decoder-evaluation.log).

## 1. Plain-language summary

Two groups of in-game sound still cannot be previewed in Studio. The Xbox 360 and PS3 NHL 14 announcer and
play-by-play lines use EA's version of Speex; every Xbox 360 sound-bank stream uses XMA. For Speex, the original
reference library (libspeex, a small permissively licensed C library) can be built into Studio. I tested it on
the research's own sample lines: it reproduces the reference output to within one unit in the last bit, on
every sample tested. **Recommended: adopt libspeex for the Speex preview.** For XMA there is exactly one
maintained open decoder, inside FFmpeg, under the LGPL. Shipping it is a licensing and packaging decision,
not a technical one, and Studio's own licence is still "to be determined". **Recommended: do not adopt an XMA
decoder until the project licence is set; if the LGPL is acceptable, ship a minimal FFmpeg build as a separate
helper program.** Until then XMA stays "framed, no preview".

## 2. Current audio architecture (what a decoder must fit)

- `tdb-snd` decodes MPEG Layer III through Symphonia (`symphonia-bundle-mp3` 0.6, MPL-2.0, pure Rust), fed by
  Studio's own EALayer3 → MPEG rebuild; PS4 EA-MP3 likewise. No C code and no external process in the audio path today.
- Previews are **diagnostic**, decoded on request, returned as an in-memory WAV. Windows is the required desktop
  platform; builds use MSVC (present for Tauri).
- Research vectors: R83 (EA-Speex: UWB 32 kHz, `u8 length | frame` chains, 509-sample delay; 96 / 96 vectors
  structurally verified; the ffmpeg `libspeex` PCM hashes recorded), R84 / R92 / R100 (EA-XMA: XMA2 packets, ffmpeg
  `xma2` exact counts; X360 NHL 09 header variant).

## 3. Speex (ultra-wideband, 32 kHz)

| Candidate | Licence | Build / packaging | Fidelity against the R83 vectors | Maintenance |
|---|---|---|---|---|
| **libspeex C source (Xiph), via `speex-sys` 0.4.0** (vendored, unmodified upstream `.c` / `.h`, compiled by `cc`) | libspeex **BSD-3-Clause**; binding crate MPL-2.0 (file-level copyleft on its own files only) | static, offline, no runtime files; release `speex.lib` 432 KB (MSVC); **build needs libclang** (bindgen 0.65) — present on this machine (`C:\Program Files\LLVM`), not guaranteed on CI | **measured:** 4 / 4 R83 picks on X360 NHL 15 `ann_eng` decode to frames × 640 with R83's trims (216 / 246 / 332 / 452); against ffmpeg's `libspeex` on **pick 16 only** (the one sample-by-sample comparison; the other three were checked for length and trims): **max \|diff\| 1 LSB, corr 1.00000, 95.5 % samples identical** (enhancer on; with it off, corr 0.998 — the reference path uses the enhancer); not bit-identical by hash (float → int rounding of a FLOATING_POINT build) | libspeex 1.2.x is the reference and stable; `speex-sys` last released July 2023 (thin) |
| same C source with a **hand-written FFI** (≈ 8 functions) and `cc`, no bindgen | BSD-3-Clause only | as above without the libclang build requirement | same library, same result expected | vendoring becomes Studio's responsibility |
| `oxideav-speex` (pure Rust) | MIT | no C toolchain | its own reports: UWB corr 0.9973 on tone fixtures, **not bit-exact**, "reference-tracking"; not run here | pre-1.0, 1 star |
| FFmpeg native `speex` decoder | LGPL-2.1+ | a whole FFmpeg for one codec | R83: a different implementation from libspeex (corr 0.96–1.00; here 0.982, max diff 5 266) | maintained |

**Recommendation (Speex):** libspeex. Prefer the existing compatible crate `speex-sys` =0.4.0 **if** the
coordinator accepts libclang as a build requirement. Otherwise the same vendored C source with a hand-written
FFI (bounded: `speex_decoder_init(&speex_uwb_mode)`, `speex_decoder_ctl` for `SET_ENH` / `GET_FRAME_SIZE`,
`speex_bits_init / read_from / destroy`, `speex_decode_int`, `speex_decoder_destroy`). Enhancer **on**, as the
reference path. The preview keeps R83's contract and label: drop 509 samples and the tail trim, "diagnostic
preview — not game-verified". The 1-LSB difference from the ffmpeg path is stated, not hidden. Scope when
accepted: X360 11–Legacy `ann_eng` / `pbp_eng`, PS3 14 `pbp_eng`, PS4 15–21 `rink_ann` (R83 families). PS4 NHL 21's
single-block outlier stays labelled as R93 describes ("near-silent output under both decoders").

## 4. XMA2

| Candidate | Licence | Build / packaging | Fidelity | Notes |
|---|---|---|---|---|
| **FFmpeg `xma2` decoder** (in `libavcodec/wmaprodec.c`) | **LGPL-2.1+** as built without `--enable-gpl` | (a) **separate helper executable**: a minimal FFmpeg (only `xma2` decoder, the WAV demuxer and `pcm_s16le`) shipped beside Studio and fed a synthesised RIFF over a pipe — cleanest LGPL boundary, a few MB; (b) dynamic `avcodec` DLL via `ffmpeg-sys-next` — needs FFmpeg headers / import libraries at build and the same LGPL duties | research: exact sample counts on R84 / R92 vectors and R100's three X360 NHL 09 vectors (header-independent) | the FFmpeg on this machine is a **GPLv3 "full" build** (`--enable-gpl --enable-version3`) and **must not** be what Studio ships |
| Rust port of the XMA2 path of `wmaprodec.c` | a derivative of LGPL code (LGPL obligations follow) | large (the WMA Pro decoder core) | unknown until built | not recommended |
| none (status quo) | — | — | — | X360 entries stay "framed, no preview" |

No suitable standalone XMA2 decoder outside FFmpeg was found in the recorded search (crates.io / docs.rs search, 26 Sep; results listed in the
evidence log). **Recommendation (XMA2):** a decision is required that this lane cannot make — the project's
licence is "to be determined" (`README.md`), and shipping an LGPL component brings source-offer, relinking /
replacement and notice obligations. If the coordinator accepts LGPL for an optional component: option (a), a
minimal FFmpeg helper built from a pinned release with only the listed components, with its build script and
source offer in the repository; Studio calls it only for a diagnostic preview and works fully without it. If
not: keep the status quo. The X360 NHL 09 header variant needs no special handling for decoding (R100),
but Studio's own framing of that variant remains a separate listing task.

## 5. Blockers and questions for the coordinator

1. **Speex:** accept libspeex (BSD-3) with `speex-sys` (libclang at build) or with a hand-written FFI (no libclang)?
2. **XMA2:** is an LGPL helper executable acceptable while the project licence is undetermined? If yes, the
   developer builds and pins the minimal FFmpeg in an isolated package. If no, XMA previews stay deferred.
3. Neither choice is "accepted" by this document; ffmpeg having decoded a research vector is not an acceptance.

## 6. Cleanup

The evaluation harness (an isolated scratch crate outside the repository; its target directory and dumped
PCM / Ogg files) is deleted after this document is published; only the evidence log above is kept.
