# Receipt 29 — EA-Speex diagnostic preview through pinned libspeex (no libclang)

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing listened to; nothing
game-verified.** HEAD `8d60a6d` (unchanged; nothing staged or committed). Claim `dev-speex-preview`.
Direction: [after-receipt-27 prompt](../../plans/development-20260926-after-receipt27.md), "Audio decisions —
Speex". **Not in the 26 Sep stable review build** (built before this package).

## 1. Plain-language summary

- Studio can now play a **diagnostic preview** of EA's Speex speech (codec `0x09`): the Xbox 360
  announcer and play-by-play banks of NHL 11 → Legacy, PS3 NHL 14 play-by-play, and the PS4 NHL 15 → 21
  rink-announcer banks. Each preview says "not game-verified"; nothing is written and there is no
  encoder.
- The decoder is the official Speex library, version 1.2.1, copied **unchanged** from Xiph's release
  (its checksum matches Xiph's published list) and built with a small C wrapper — no libclang needed.
  The copy bundled in the `speex-sys` crate was not used: it differs from the release in nine files.
- On every research sample (96 streams, 25 families) Studio's output is within **1 unit** of the
  reference decoder on every sample (94–96 % identical; the NHL 21 single-record outlier is
  bit-identical). That is agreement between decoders, not a listening result.
- One family is **not** offered: Xbox 360 NHL 14 announcer. Its audio decodes, but Studio's existing
  bank-table reader cannot read that title's announcer bank, so the listing has no passing test there.
- The library's internal-error path (which would normally terminate the program) is contained: a
  corrupt frame refuses with a message instead of closing Studio.

## 2. Files

| Path | Change |
|---|---|
| `Cargo.toml` | workspace member `crates/tdb-speex` |
| `crates/tdb-speex/` (**new**) | `vendor/speex-1.2.1/` (upstream `COPYING`, `AUTHORS`, 30 default `.c`, all `.h`; byte-identical to the tarball; `vendor/SHA256SUMS-vendored.txt`), `shim/nhlms_speex_shim.{h,c}`, `shim/include-gen/{os_support_custom.h,speex_config_types.h}`, `build.rs` (`cc`), `src/lib.rs` (hand-declared FFI, `UwbDecoder` with `Drop`, named `SpeexError`), `README.md` (provenance, recipe, obligations) |
| `crates/tdb-snd/Cargo.toml`, `src/lib.rs`, **`src/easpeex.rs` (new)** | refusing grammar (u8 length 1..=255 chain consuming each block; first / middle / last / single-block counts; Σ = header), decode (enhancer on, drop 509 + tail), `MEASURED` admitted gen7 tuples (a `static`), label |
| `crates/tdb-snd/src/sns.rs` | codec label `0x09` = `ea-speex` |
| `crates/studio-domain/src/gen7_speech_preview.rs` | `coverage` admits the EA-Speex tuples (never Legacy Recompiled); listing `previewable` by the tuple's codec; `speex_preview` route; `route_detail` field |
| `crates/studio-domain/src/ps4_speech_bank.rs` | rink-announcer directory and the two bank names; listing includes them; EA-Speex preview branch with the R93 disclosure on NHL 21 `rink_ann_samples` |
| `crates/studio-api/src/{dto,registry}.rs`; `app/src/bindings/Gen7SpeechPreviewDto.ts` (generated) | `route_detail` |
| `app/src/components/Gen7SpeechPreview.tsx` + test; `Gen7BankCard.test.tsx` | the Speex diagnostic line replaces the granule line when present |
| `crates/tdb-snd/tests/ea_speex_real_vectors.rs` (**new**), `crates/studio-domain/tests/easpeex_preview.rs` (**new**) | below |
| `crates/studio-domain/src/gen7_speech_preview.rs` (unit test), `crates/studio-domain/tests/gen7_speech_preview.rs` | assertions that X360 NHL 11 / PS3 NHL 14 `pbp_eng` had no route updated to the R83 admission |

## 3. Evidence (`docs/validation/evidence/overnight-development-2026-09-24/logs/`)

- **Unit** (`r29-affected-suites.log`): `tdb-speex` 3 / 3 — version `speex-1.2.1`, frame size 640,
  packet bounds refused before the call, 3 000 deterministic arbitrary packets: 828 decoded, 2 172
  refused by name, 0 internal errors reached (the fatal path is contained by construction; no
  vector reached it). `tdb-snd` lib 18 / 18 incl. the grammar's accept shape and every refusal
  (length 0, overrun, first / middle / last / single-block counts, total, no blocks, codec).
- **Real vectors** (`r29-speex-real-vectors.log`, `--ignored`, read-only X: / H: trees, FFmpeg 8.1.1
  full build as an **external research oracle only**): 96 R83 vectors (X360 NHL 14 `pbp_eng` offset
  820 125 120 has no research vector — R83 skipped it): SNS SHA-256 = research, frames and tail trim
  = research, the four R83 mutations refuse, decode = frames × 640 and trimmed = `num_samples`.
  Per-family comparison with FFmpeg `libspeex` on a lossless Ogg rewrap: **max |diff| 1 LSB on every
  family, identical 94.1–95.9 %, correlation 1.000000 (6 d.p.)**; NHL 21 base `rink_ann_samples`
  (1 vector, 12 frames) 0 LSB / 100 %. Tolerance asserted: ≤ 2 LSB and corr ≥ 0.9999.
- **Domain** (`r29-speex-domain.log`): `easpeex_preview` 3 / 3 with `--include-ignored` — 10 gen7
  tuples list their first row as `ea-speex` / previewable and preview it to exactly the SNR count
  (e.g. X360 NHL 11 `ann_eng` sample 45: 143 frames, 91 520 decoded, tail 216 → 90 795); all 14 PS4
  rink-announcer banks (NHL 15–21) list and preview; the R93 note appears only on NHL 21
  `rink_ann_samples`; Recompiled and X360 NHL 14 `ann_eng` refuse by name.
- **Affected suites**: studio-domain lib 222 + the updated coverage test (`-2.log`: 1 / 1),
  `ps4_speech_bank` 5, `gen7_speech_preview` 1 (after the update), `gen7_sfx_banks` 2, studio-api lib
  31; `export-bindings --write` (1 file) then `--check` exit 0; `tsc` exit 0; vitest 40 files / 250.

## 4. Decisions and limits

- **Admission is per tuple**, each backed by the two regressions above; the four X360 announcer
  picks of receipt 27 did not stand in for any other family.
- **X360 NHL 14 `ann_eng` not admitted** — Studio's `tdb-speech` SBKR reader refuses that bank
  (`SBKR: file is 450160 bytes, shorter than the 4 needed at offset 1263665424`). The stream itself
  passes the tdb-snd vectors. Next check: the SBKR table variant of that one bank (developer, bounded).
  **Addendum (receipt 31):** the bank ends with 8 all-zero id-table slots; the reader now skips them
  and the tuple is admitted with its regression (sample 45 → 90 795 samples).
- **Legacy Recompiled not admitted** (no R83 vector; its registered source is not measured here).
- The **X360 NHL 14 `pbp_eng` loose stream is truncated** (R80); the listing reads the loose stream
  first, so samples beyond its end refuse by name rather than falling back to the archive member.
- **R93 outlier** (PS4 NHL 21 base `rink_ann_samples`, 1 record): previewed with the disclosure; here
  both decoders agree exactly, which says nothing about what the game plays.
- 509 / tail trims are the count rule (R83 / R88 §F4), not a heard result. Card below.
- PS4 preview payload reuses the EA-MP3 fields (frames, frames per block, trim; embedded PCM 0; the
  rendering toggle has no effect on Speex) — cosmetic, noted for the UI.
- **Obligations:** a distributed binary must reproduce libspeex's `COPYING`; no publication authorised.

## 5. Cards

- **A-SPX-2** (listening, after a build that includes receipt 29): X360 NHL 15 `ann_eng` sample 45 and
  PS4 NHL 15 `rink_ann_samples_inxp` sample 45 → Decode diagnostic preview. Expected: clean speech at
  32 kHz, no click at the start. A click or clipped onset questions the 509 rule; garbled audio is a
  decoder defect (report title, bank, sample). Supersedes the ffmpeg-based A-SPX-1 for Studio's route.

## 6. Cleanup

The upstream tarball and extracted tree in the session scratchpad were deleted after vendoring (hashes
recorded in `crates/tdb-speex/README.md` and `vendor/SHA256SUMS-vendored.txt`). The oracle test's
temporary directory (`nhlms-easpeex-oracle-*`) is removed by its guard. No build cache was added
beyond the ordinary `target/` compile of the new crate.

## 7. Next

XMA2: the helper build is **blocked** on this machine (no GNU make / MinGW / MSYS to run a pinned
FFmpeg `configure`; installing one is a global tool install, not done). The Studio-side invocation
(controlled path, structured arguments, bounded I/O and time, pipe draining, cleanup, missing-helper
behaviour) is independent and is the next bounded step; then goalie gear 4.5.
