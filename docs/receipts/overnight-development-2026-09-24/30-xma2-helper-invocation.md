# Receipt 30 — EA-XMA previews: Studio-side helper route delivered; the helper build is blocked on this machine

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing listened to; nothing
game-verified.** HEAD `8d60a6d` (unchanged; nothing staged or committed). Claim
`dev-xma2-helper-invocation`. Direction: [after-receipt-27 prompt](../../plans/development-20260926-after-receipt27.md),
"Audio decisions — XMA2". Not in the 26 Sep stable review build.

## 1. Plain-language summary

- **The helper itself could not be built here.** Building a pinned, minimal FFmpeg needs a Unix-style
  build environment (GNU make plus MinGW/MSYS, or MSVC driven from MSYS). None is installed, and
  installing one is a machine-wide tool install that was not authorised by surprise. The installed
  FFmpeg is a GPLv3 "full" build and was **not** adopted.
- **Everything on Studio's side is done and tested**: Studio wraps EA's Xbox 360 XMA audio into a
  standard XMA2 file, runs a helper program by an explicit path (never through a shell, never found on
  `PATH`), feeds it and reads it on separate threads so it cannot hang, limits output size and time,
  can cancel it, always stops and reaps it on any error, and hides its window on Windows.
- **Without a helper**, Studio behaves as before: the entries are listed and framed, and the preview
  button's reason now says plainly that no XMA2 helper is installed and where Studio looks for one.
- **With a helper** (tested here only by pointing Studio at the local FFmpeg as a research stand-in),
  all 83 research samples decode to exactly their declared length, and the audio matches the research
  output bit for bit. That checks Studio's plumbing, not the future helper, and not the sound.

## 2. Files

| Path | Change |
|---|---|
| `crates/tdb-snd/src/eaxma.rs` (**new**) | refusing packet extraction for SNS blocks (`w0`, one packet per stream, pad to 0x800) and raw ram segments; packet-header check; RIFF XMA2 (WAVEFORMATEX 0x0166 + the documented XMA2WAVEFORMATEX extension); label; unit tests |
| `crates/tdb-snd/src/helper.rs` (**new**) | `locate` (the `NHL_STUDIO_XMA2_HELPER` file, else `helpers\nhlms-xma2-helper.exe` beside the executable; no `PATH`), `run` (structured args, no shell, `CREATE_NO_WINDOW`, stdin writer + stdout / stderr drain threads, byte caps, timeout, cancellation flag, kill-and-reap guard), `decode_xma2` with the fixed argument list |
| `crates/tdb-snd/src/bin/nhlms-helper-stub.rs` (**new**) | test-only stub helper (echo / sleep / flood / fail / stderr modes); not a decoder, never shipped |
| `crates/tdb-snd/src/lib.rs` | modules |
| `crates/studio-domain/src/gen7_sfx_banks.rs` | `xma_gate` + `xma_preview`: Xbox 360 (not Recompiled) single-segment, non-looping ram records, cc 0 / 1, only with an installed helper; decoded count must equal the record's; every other case framed with its reason; NHL 09 wording |
| `crates/tdb-snd/tests/helper_runner.rs`, `ea_xma_real_vectors.rs` (**new**); `crates/studio-domain/tests/gen7_sfx_banks.rs` | below |

## 3. Evidence (`docs/validation/evidence/overnight-development-2026-09-24/logs/`)

- `tdb-snd` lib 20 / 20 (incl. `eaxma` extraction / refusals / RIFF layout); `helper_runner` 3 / 3:
  8 MiB echo without a pipe deadlock, failing exit with stderr text, 1 MiB stderr flood beside a normal
  result, stdout flood stopped at a 1 MiB cap, 0.5 s timeout and a pre-set cancellation both stop a
  30 s sleeper (well under 10 s), missing helper file refused, `locate` uses only the env file or the
  `helpers` folder (`r30-affected-suites.log`).
- `r30-xma-real-vectors.log` (`--ignored`, read-only X: trees, `NHL_STUDIO_XMA2_HELPER` = the local
  FFmpeg 8.1.1 full build as a **plumbing oracle only**): **83 / 83** admitted R84 vectors — SNS speech
  X360 NHL 09 / 10 `ann_eng` / `pbp_eng` (16), stereo SNS music NHL 09–12 (11; up to 19.2 M samples),
  loose ram segments of `000_goal` / `000_period` / `000_chant` / `sfx` on NHL 10 → Legacy (56) —
  source identity = research hash, decoded samples per channel = declared, PCM hash = the research
  oracle's on every vector. One R84 vector (NHL 11 music, offset 298 517 360) has no research hash
  (beyond the loose stream) and is skipped as R84 did; NHL 09 archived banks are not read.
- `r30-xma-domain.log`: `gen7_sfx_banks` 6 / 6 with the helper named (X360 NHL 15 `000_goal.abk`
  record 0: `ea-xma`, 2 ch, 298 234 = declared; tree marks it previewable; Recompiled refused "no
  measured EA-XMA vector"; with the variable removed the same entry refuses "no XMA2 decoder helper is
  installed"), then the X360 tree case again **without** any helper: 1 / 1.
- studio-domain lib 223 / 2 ignored; studio-api lib 31. No DTO change (bindings untouched).

## 4. Blocker and the proposed (unexecuted) helper recipe

**Blocker (exact):** no `make`, `gcc` / MinGW, MSYS2 or other POSIX build environment on this machine
(`command -v make gcc` empty; no `C:\msys64`); FFmpeg's `configure` + `make` need one. LLVM / MSVC are
present but cannot run `configure` alone. Unblocking needs an authorised toolchain install (or a
separate build machine) and a network fetch of the pinned source release.

Proposed recipe for review (nothing downloaded or built): a pinned FFmpeg **release tarball**,
signature-verified, then

```
configure --disable-everything --disable-autodetect --disable-network --disable-doc --disable-debug
          --disable-ffplay --disable-ffprobe --disable-avdevice --disable-swscale --disable-postproc
          --enable-static --disable-shared --enable-small
          --enable-decoder=xma2 --enable-demuxer=wav --enable-muxer=pcm_s16le --enable-encoder=pcm_s16le
          --enable-protocol=pipe --enable-swresample
          --enable-filter=aresample,aformat,anull,abuffer,abuffersink
```

(no `--enable-gpl`, `--enable-nonfree` or `--enable-version3`). Acceptance before any use: the built
binary's `-L` reports LGPL 2.1-or-later; `-buildconf`, `-decoders`, `-demuxers`, `-muxers`,
`-protocols` and `-filters` list exactly the closure above (plus configure's automatic
dependencies, recorded); the corresponding source, the recipe, patches (none planned) and notices are
kept with the binary; the 83-vector case above passes with it; distribution arrangement reviewed
before release. The argument list Studio sends (`helper::XMA2_ARGS`) needs exactly those components.

## 5. Limits

- Admitted route: Xbox 360 NHL 10 → Legacy ram records (single segment, not looping, cc 0 / 1).
  **Not** admitted: stream (`.ast`) XMA sub-blocks and loops (R92 — framed only), multi-segment ram
  records, the NHL 09 header variant (R98b / R100 — its source path stays refused), Recompiled.
- Speech / music SNS XMA (X360 NHL 09–12) is validated at the `tdb-snd` level but **not wired** into
  the gen7 speech listing (the NHL 09 / 10 speech tuples have no bank-listing route yet); a bounded
  follow-up once a helper exists.
- Exact counts and matching hashes are decoder agreement, not audible fidelity. Card A-XMA-2 (below)
  waits for an accepted helper.
- The local GPLv3 FFmpeg was used only as a test oracle via an environment variable; nothing copies,
  bundles or references it in Studio.

## 6. Cards

- **A-XMA-2** (after an accepted helper is built and installed; not now): X360 NHL 15 → Sound banks →
  `000_goal.abk` record 0 → Decode diagnostic preview. Expected: a clean goal horn, stereo, ~6.8 s.

## 7. Cleanup

No temporary directories remain (the runner test's `nhlms-helper-locate-*` is removed by the test).
The stub helper is a build product of `target/` only. No toolchain, source tarball or helper binary
was downloaded or installed.

## 8. Next

Goalie gear 4.5 (masks, pads, blocker, trapper, goalie stick) on the equipment sweep pattern, including
the genuine NHL 20.
