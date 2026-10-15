# Receipt 56 — X360 NHL 10 / 15 / Legacy stream sounds of every shape preview

**Developer-verified; not Codex-accepted; nothing launched or listened to.** HEAD `8d60a6d` (unchanged; nothing staged). Claim
`dev-xma-inbank-residuals` (extended). Research consumed: 2026-09-28 handoff 06 §4 addendum (R140b), which answers receipt 53's
"shapes research did not sweep".

## 1. Plain-language summary

- **Every Xbox 360 stream sound on NHL 10, 15 and Legacy now previews.** Before, only mono and stereo sounds without a loop
  previewed on NHL 10 and Legacy. NHL 15 also had its looping four-channel sounds.
- Newly admitted:
  - looping stereo sounds (NHL 10, 15, Legacy);
  - four-channel sounds, with and without a loop (NHL 10, 15, Legacy);
  - six-channel sounds (NHL 15, Legacy).
- Four- and six-channel sounds play one stereo pair at a time. No speaker order is claimed.
- The preview still refuses a sound whose decoded length differs from the length the bank declares.

## 2. Files

| Path | Change |
|---|---|
| `crates/studio-domain/src/gen7_sfx_banks.rs` | `XMA_STREAM_SHAPES` adds NHL 10 (stereo loop, 4 ch, 4 ch loop) and NHL 15 / Legacy (stereo loop, 4 ch, 6 ch); the refusal text now cites R140 / R140b |
| `crates/studio-domain/tests/gen7_xma_ram_residuals.rs` | new `research_06_addendum_stream_vectors_decode_to_their_declared_counts` |
| `crates/studio-domain/tests/gen7_sfx_banks.rs` | the Legacy looping-stereo stream entry, asserted refused since receipt 33, is now asserted previewable |

## 3. Evidence (`NHL_STUDIO_XMA2_HELPER` = the pinned helper `dc3533ed…`; real loose trees, read only)

- **New case 1 / 1.** Every addendum regression input decodes to its declared count:
  - NHL 10 `crowds` #198 → stub 23 + body 100 681; #121 → 21 + 500 650 per pair; #144 → 847 018 per pair;
  - NHL 15 and Legacy `ambientskatesandpunches` #132 → 32 + 815 616; `crowds` #0 → 24 + 563 232 per pair; #83 → 952 896 per pair;
  - NHL 15 and Legacy `sfx` #441 → 455 519 on each of three pairs.
- `gen7_xma_ram_residuals` 4 / 4 and `gen7_sfx_banks` 7 / 7 (`--include-ignored`); `studio-domain --lib` 226 passed, 2 ignored.

## 4. Limits

- This is decoder evidence, not a listening check. The speaker mapping of four- and six-channel sounds is not established.
- Recompiled stays refused. Its archive audio route is unmeasured.
- NHL 09 keeps the R131 shapes only.
