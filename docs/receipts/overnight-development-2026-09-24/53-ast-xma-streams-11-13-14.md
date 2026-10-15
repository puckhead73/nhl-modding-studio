# Receipt 53 — X360 `.ast` stream sounds on NHL 11 / 13 / 14 (and NHL 12 loops / surround) now preview (research 2026-09-28 handoff 06)

**Developer-verified; not Codex-accepted; not listened to; nothing written.** HEAD `8d60a6d` (unchanged; nothing staged). Claim
`dev-xma-inbank-residuals` (same audio package as receipt 50, extended). Research consumed: 2026-09-28 handoff 06 (R140), published
05:44.

## 1. Plain-language summary

- **More Xbox stream previews.** On Xbox 360 NHL 11, 13 and 14, stream sounds (the long ones kept in `.ast` files) now preview. So do
  looping and four- or six-channel streams where research decoded them. Research decoded **every** stream record of those titles
  exactly, with NHL 12 as a control.
- Looping streams play their stub and body once, with the loop point shown. Surround streams play one stereo pair at a time, with no
  speaker order claimed.
- Shapes research did not sweep stay "framed only" with the reason, for example looping stereo streams on NHL Legacy.

## 2. Change

`crates/studio-domain/src/gen7_sfx_banks.rs`:
- `XMA_STREAM_TITLES` adds `nhl11`, `nhl13` and `nhl14`, for non-looping mono / stereo streams (S1).
- The new `XMA_STREAM_SHAPES` admits a shape only on the titles R140 swept:
  - looping stereo: NHL 11–14 (S2);
  - non-looping four-channel: NHL 11–14 (S3);
  - looping four-channel: NHL 11–15 (S3; NHL 15 is R92 vector H);
  - six-channel: NHL 13 / 14 (S3).
- The refusal text names R140.
- The preview route is unchanged: stub and body decoded separately, the chosen pair only, and decoded = declared or refused.

Tests:
- `tests/gen7_xma_ram_residuals.rs` gains `research_06_…`;
- `tests/gen7_sfx_banks.rs` swaps its "unmeasured" example from an NHL 11 stream (now measured) to a Legacy looping stereo stream.

## 3. Evidence (`NHL_STUDIO_XMA2_HELPER` = the pinned helper `dc3533ed…`; real loose trees, read only)

- **`research_06_stream_vectors_decode_to_their_declared_counts` 1 / 1, first run:**
  - **S1:** NHL 11 `colors1.abk` #0 mono 54 000; NHL 13 / 14 `ambientskatesandpunches.abk` #0 stereo 28 496 and #3 mono 17 068;
  - **S2:**
    - NHL 11 `crowds.abk` #6: stub 26 + body 113 266;
    - NHL 13 `ambientskatesandpunches.abk` #207: 32 + 815 616;
  - **S3:**
    - NHL 11 `crowds.abk` #13: 952 896 on each of 2 pairs;
    - #14: 24 + 691 200 per pair;
    - NHL 13 `sfx.abk` #206: 455 519 on each of 3 pairs.
- **Regression:** `gen7_sfx_banks` X360 cases 3 / 3, including the R92 pins and the new Legacy refusal example.

## 4. Limits

- Not listened to, and no speaker mapping is claimed.
- Only the named vectors were decoded here; research decoded every record.
- NHL 10 / 15 / Legacy shapes beyond R92's stay refused, and Recompiled stays refused.
