# Receipt 50 — X360 EA-XMA in-bank sounds: a false refusal fixed; intro + loop, looping and surround records preview (research 2026-09-28 handoff 05)

**Developer-verified; not Codex-accepted; not listened to; nothing launched or written.** HEAD `8d60a6d` (unchanged; nothing
staged). Claim `dev-xma-inbank-residuals`. Research consumed: 2026-09-28 handoff 05 (R139 / R139b / R139c), published 05:37 and not
yet coordinator-reviewed. Consumed under the dispatch's fallback 1 (a demonstrated defect) and 3 (a ready contract), for **diagnostic
previews only**.

## 1. Plain-language summary

- **Defect fixed.** On Xbox 360 NHL 10 → Legacy, Studio refused in-bank sounds containing a packet that marks "no new frame starts
  here" (frame count 0, offset `0x7FFF`). Research found 80–103 such sub-streams per title, each decoding exactly when the packet is kept.
  Studio now keeps that one marker on those titles' in-bank records. Every other bad packet header still refuses.
- **New previews (diagnostic, same shipped helper):**
  - **intro + loop** records, two pieces decoded separately and joined, with the loop point shown;
  - **looping** stereo and four-channel records, played once;
  - **four- and six-channel** records, one stereo pair at a time, with no claim about which speakers a pair feeds.

  Every piece must decode to exactly the length the bank declares, or the preview is refused.
- **On NHL 10 the sound-bank tree now offers a preview for all 2 050 EA-XMA in-bank entries.** Every shape class was decoded to its
  exact count through the pinned helper.
- **Unchanged:**
  - NHL 09's own rule;
  - Recompiled (not measured);
  - `.ast` stream records (NHL 11 / 13 / 14 shapes are research's current item);
  - nothing is replaceable.

## 2. Files

| Path | Change |
|---|---|
| `crates/tdb-snd/src/eaxma.rs` | `HeaderRule::CheckedNoFrameMarker` (accepts only frame count 0 with offset `0x7FFF`, counted as 0 frames), `NO_FRAME_OFFSET`, `segment_chunks` (a `w0` chain that must tile the segment); 2 unit tests (the marker kept only under its rule; count 0 with any other offset and NHL 10's (2, 16 384) header still refuse; chunks tile or refuse) |
| `crates/studio-domain/src/gen7_sfx_banks.rs` | `XMA_RAM_RESIDUAL_TITLES` (X360 NHL 10 / 11 / 12 / 13 / 14 / 15 / Legacy); `header_rule` uses the marker rule there; the gate admits exactly the measured shapes (1 segment mono / stereo / 4 / 6 ch; 1-segment looping stereo / 4 ch with loop 0; 2-segment looping stereo with the loop at segment 0's end) and names the rest; `xma_preview` decodes per segment and per chunk (`sub_stream` = stereo pair), checks each against its declared count, joins intro + body, reports the loop point and pair; empty notes removed; the old "one stream only" guard replaced by the chunk bound |
| `crates/studio-domain/tests/gen7_xma_ram_residuals.rs` (**new**) | below |

## 3. Evidence (`NHL_STUDIO_XMA2_HELPER` = the pinned bundle helper `dc3533ed…`; real loose trees, read only)

- **`gen7_xma_ram_residuals` 2 / 2 (`--ignored`), research's vectors:**
  - `000_chant.abk` #1: 48 888, one piece, played once (X3);
  - `073_chant.abk` #0: 52 575 = 4 437 + 48 138, loop at 4 437, two segments (X2);
  - `002_chant.abk` #1: two stereo pairs of 90 452 each, and pair 3 refused (X4);
  - `002_chant.abk` #0 pair 2, on NHL 10 **and** Legacy: 266 690. It has the (0, `0x7FFF`) packet and was refused before (X1);
  - `002_goal.abk` #0: 352 323 (X1 on a single-stream record);
  - **NHL 10 sweep:** the tree admits 2 050 / 2 050 EA-XMA ram entries. The first record of each class decodes exactly: mono
    99 683, stereo 435 337, four-channel 266 690 per pair, six-channel `ambientskatesandpunches.abk` #166 156 373 on each of three
    pairs, looping stereo 48 888, looping four-channel 90 452.
- **First run failed** (recorded): the old guard in `xma_preview` refused any pair but the first. It was removed; re-run green.
- **Regressions:** `tdb-snd` unit tests. The existing XMA suites (`gen7_sfx_banks`, `x360_nhl09_xma_banks`, `eaxma_speech_preview`)
  with the helper: see §5.

## 4. Limits

- **Not decoded here:**
  - every admitted record: research decoded them all, and here one per class and the named vectors were decoded;
  - NHL 11–15 records: only Legacy's `002_chant` #0 was decoded beyond NHL 10.

  A record whose decode differs from its declared count, such as NHL 10's one (2, 16 384) sub-stream, is refused at preview time with
  the counts named.
- **Not claimed:**
  - listening: counts are not audible fidelity;
  - speaker mapping for four- or six-channel pairs;
  - the meaning of the `0x7FFF` marker beyond the exact decode.

## 5. Regression run (with the helper, `--include-ignored`)

- `gen7_sfx_banks` 7 / 7 (incl. the R92 `.ast` pins, the NHL 15 gate listing and the "refuse without a helper" case);
- `x360_nhl09_xma_banks` 2 / 2 (every NHL 09 record through the unchanged NHL 09 rule; 750 s);
- `eaxma_speech_preview` 2 / 2 (the eight R84 tuples, PCM = research);
- `tdb-snd` 24 / 24 unit tests (+ 5 integration, 4 ignored cases not selected).
