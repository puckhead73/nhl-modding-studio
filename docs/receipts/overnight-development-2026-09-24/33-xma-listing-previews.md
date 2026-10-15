# Receipt 33 — Xbox 360 XMA previews reachable: speech / music banks and `.ast` sound-bank streams

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing listened to; nothing
game-verified.** HEAD `8d60a6d` (unchanged; nothing staged or committed). Claim `dev-xma-listing-previews`
(written at the first checkpoint rather than before the first edit — a process slip, recorded here). Direction:
[overnight prompt after receipt 31](../../plans/development-20260927-overnight-after-receipt31.md) §2. Uses the
helper of [receipt 32](32-xma2-helper-build.md).

## 1. Plain-language summary

- **Speech and music banks.** The sample-bank card (Speech page) now lists and previews the Xbox 360 banks
  that are XMA: announcer and play-by-play on NHL 09 and NHL 10, and the music bank on NHL 09, 10, 11 and 12.
  Each was admitted on its own research vectors. The first sample of each decodes to exactly its declared
  length and **byte-identical** to the research decode.
- **Sound-bank streams (`.ast`).** Xbox 360 stream sounds that were "framed only" now preview where research
  decoded that shape on that title: ordinary mono / stereo streams on NHL 10, 12, 15 and Legacy, and on NHL 15
  the looping four-channel crowd record, one stereo pair at a time (loop start and body joined once). Every
  research vector matches byte for byte. Other shapes and titles stay framed with the reason shown.
- **Without a helper**, the card says "Listed and framed only — no XMA2 decoder helper is installed …" and
  nothing is previewable. If the helper disappears after the list was loaded, the preview refuses with that
  reason instead of reusing an old result.
- **Cancel.** A running XMA preview can be cancelled; changing the selection or leaving the card cancels it
  too. Studio stops the helper process on the server side.
- Still not offered: Xbox NHL 09 sound banks (their own header variant and archive source — next bounded
  item), looping or multi-segment ram records, Legacy Recompiled, NHL 11 / 13 / 14 `.ast` streams (no research
  vector of that shape there).

## 2. Files

| Path | Change |
|---|---|
| `crates/tdb-snd/src/eaxma.rs` | `MEASURED` SNS tuples (R84) + `measured()`; `block_packets` (per-stream pick for stereo pairs; terminal zero padding allowed only for `.ast`, R91 §1); unit test |
| `crates/studio-domain/src/gen7_speech_preview.rs` | coverage admits the R84 tuples (never Recompiled); listing `preview_unavailable` (helper lookup) and row gating (loops refused by name); `xma_preview` (helper looked up again at preview time, helper SHA-256 in the notes, count check); `preview_sample_cancellable` |
| `crates/studio-domain/src/gen7_sfx_banks.rs` | `xma_gate` by title and shape (`XMA_STREAM_TITLES`, `XMA_LOOP_PAIR_TITLES`); `xma_stream_preview` (framing on the checked handle, pair selection, loop stub + body each to its own count) |
| `crates/studio-api/src/{registry,commands,dto,lib}.rs` | `preview_unavailable` on the listing DTO; per-project in-flight preview flag (a new request cancels the previous); command `gen7_speech_preview_cancel` |
| `app/src/bindings/Gen7BankListingDto.ts` | generated (`export-bindings --write`, 1 file; `--check` in sync) |
| `app/src/lib/client.ts`, `app/src/components/Gen7BankCard.tsx`, `Gen7SpeechPreview.tsx` | no-helper banner; unavailable-row reason instead of a request; Cancel button; cancel on selection change / unmount |
| tests | `studio-domain/tests/eaxma_speech_preview.rs` (**new**), `gen7_sfx_banks.rs` (new R92 case; X360 NHL 15 gate case rewritten), `gen7_speech_preview.rs` / `easpeex_preview.rs` / unit test (old "EA-XMA is refused" assertions updated); `Gen7SpeechPreview.test.tsx` (+3), `Gen7BankCard.test.tsx` (+2) |

## 3. Evidence (`docs/validation/evidence/overnight-development-2026-09-24/logs/`)

- `r33-xma-speech-domain.log` (`--include-ignored`, helper `dc3533ed…`): 8 / 8 tuples — X360 NHL 09 `ann_eng`
  (14 132 samples) / `pbp_eng` (45 589), NHL 10 `ann_eng` (13 496) / `pbp_eng` (53 276), music NHL 09 (166) /
  10 (177) / 11 (195) / 12 (183): first sample by offset previews at the SNR count, **SNS SHA-256 and PCM
  SHA-256 = R84**; first 5 000 rows of every bank are all `ea-xma` (per-row codec gating stays in force). NHL 11
  music: 50 of 195 rows lie beyond the truncated loose stream and refuse by name ("no SNS header block").
  Disposable copies: no stream → named refusal; 1 MiB stream prefix → rows beyond refuse, rows inside preview;
  one flipped sub-block length bit → "EA-XMA packets refused: block 0: 1 bytes after the stream". Helper named
  as a missing file at listing time → `preview_unavailable` and no previewable row; helper removed **after** a
  listing that offered previews → the preview refuses ("not a file"); pre-set cancellation → "cancelled".
- `r33-xma-ast-domain.log` (`gen7_sfx_banks` 7 / 7 with the helper): R92 vectors F (NHL 15 `playercalls` 0,
  mono), G (NHL 15 `crowds` 58, stereo, 159 blocks), I2 (NHL 10 `playercalls` 382), I4 (Legacy `playercalls` 0),
  I6 (NHL 12 `sfx` 27): count = declared and **WAV SHA-256 = R92** (R92 hashed its 44-byte-header WAV; Studio's
  WAV is byte-identical); H (NHL 15 `crowds` 0, loop, channel config 3): both pairs, stub 24 + body 563 232,
  each part's WAV = R92; no pair → "choose one (1–2)"; pair 3 refused; an NHL 11 stream entry stays framed with
  "no measured EA-XMA stream vector of this shape on nhl11". NHL 15 `crowds.abk`: 54 stream entries previewable,
  7 refused by shape. The same gate case without a helper: stream entries refuse "no XMA2 decoder helper".
- `r33-affected-suites.log`: studio-domain lib 223 / 2 ignored; `easpeex_preview` 3; `gen7_speech_preview` 1
  (after its update); `tdb-snd` lib 21 + `helper_runner` 5; studio-api lib 31; bindings in sync; vitest 255
  tests; `tsc` exit 0.

## 4. Limits

- Decoder agreement, not listening. Card A-XMA-2 extended below; nothing heard.
- A sample bank carries no fingerprint of its stream: a *different* stream file whose bytes happen to hold an
  SNS header at a sample's offset would preview that other sound. The preview shows the SNS hash and the
  stream identity; a truncated or corrupted stream refuses.
- Cancellation is wired for the speech / music card; sound-bank (`.ast` / ram) XMA previews are bounded by the
  helper's 60 s limit but have no Cancel button yet.
- A stereo pair of a multichannel record is a diagnostic selection, not the console's mix.
- Research counts cover the named vectors; other entries of an admitted shape use the same route by class.

## 5. Cards (for a build that contains receipts 32–33 and the helper)

- **A-XMA-2** (listening): X360 NHL 15 → Sound banks → `000_goal.abk` record 0 (ram, stereo, ~6.8 s) and
  `playercalls.abk` record 0 (`.ast` stream, mono) → Decode diagnostic preview: clean audio, no clicks.
- **A-XMA-3** (listening): X360 NHL 10 → Speech → sample banks → announcer, sample 2 → decode: a short
  announcer call at 24 kHz; music bank sample 1: stereo music.
- **V-75** (desktop): with the helper folder renamed away, the same card shows "Listed and framed only — no
  XMA2 decoder helper is installed" and a selected row shows the reason, no Decode button; restore the folder
  and reload: rows previewable. During a long music decode, Cancel shows "Decode cancelled".

## 6. Cleanup

Temporary roots `nhlms-xma-speech-*` are removed by the test's guard; scratch patch scripts deleted. No
process left running.

## 7. Next

Stable review bundle with the helper (prompt §5), then League / Team contracts (§3).
