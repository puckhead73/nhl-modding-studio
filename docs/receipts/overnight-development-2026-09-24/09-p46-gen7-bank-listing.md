# Receipt 09 (P4.6) — gen7 sample banks listed with header-measured codecs, music included

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing listened to.** HEAD
`8d60a6d` (unchanged). Claim: `dev-p46-gen7-bank-listing`. Clock: 14:26 → 14:38 (MDT; the system clock as the checks log records it).

## 1. Plain-language summary

The gen7 titles (Xbox 360 / PS3) now have the same kind of bank card the PS4 titles got earlier
tonight: pick the play-by-play, announcer or music bank, see every sample the bank's table names
with the codec / rate / channels / length read from its own header in the stream (never assumed
from the file name), and decode a selected one as the diagnostic EALayer3 v1 preview. The music
bank is included because its `.sbr` turned out to be the same SBKR table the speech reader already
parses (measured tonight on the X360 and PS3 NHL 15 files: 263 entries, the R80 count); the real
NHL 15 music listing reads 263 / 263 rows as 44.1 kHz stereo EALayer3 v1. Titles and banks outside
the research's measured set refuse with the tuple named. Nothing is written and no music-box or
selection semantics are claimed.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-domain/src/gen7_speech_preview.rs` | `bank_files` (speech vs music directory / stem), `resolve_stream` (shared by preview and listing), `Gen7SampleRow` / `Gen7BankListing`, **new** `list_bank_samples` (offset-sorted rows, id filter, cap, codec census, per-row `previewable` on the tuple's class); `coverage` admits `music_samples` on the R80 stereo tuples; the module tail was rewritten as one piece after a botched splice (unit test + real-tree test green) |
| `crates/studio-domain/tests/gen7_speech_preview.rs` | music previews on X360 / PS3 NHL 15 (sample 634, 2 ch, 495 blocks, 7 914 granules); the X360 NHL 15 music listing (263 rows, all EALayer3 v1, no header problems; id filter) |
| `crates/studio-api/src/{dto,registry,commands,lib,bindings}.rs` | `Gen7BankListingDto`, `Gen7SampleRowDto`, `gen7_bank_samples`; export roots 156 → 158 |
| `app/src/bindings/Gen7BankListingDto.ts`, `Gen7SampleRowDto.ts` (generated); `app/src/lib/client.ts` | shapes (`bankStem` admits `music_samples`) |
| `app/src/components/Gen7BankCard.tsx` (**new**) + `.test.tsx`; `Gen7SpeechPreview.tsx` (stem type; archive-member sources shown in full on the identity line); `app/src/pages/SpeechCatalogue.tsx` | the card on the speech page |

## 3. Evidence

- [`p46-gen7-bank-listing-real.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p46-gen7-bank-listing-real.log):
  X360 NHL 15 `music_samples` — 263 total, census `ealayer3-v1 263`, first rows (634 @ 16, 635 @
  2 071 168, 636 @ 3 602 944; 2 ch; previewable); the six preview cases of receipts 05 / 08 (loose
  and archived banks, mono 44.1 k / 24 k, stereo music) still pass in the same run.
- Music previews: X360 and PS3 NHL 15 sample 634, 44 100 Hz, 2 ch, 495 blocks, 7 914 granules
  (18 then 16 per block), stereo side info carried as read, decoded 7 914 × 576 per channel.

## 4. Checks ([`p46-checks.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p46-checks.log))

| Run | Result |
|---|---|
| `cargo test -p studio-domain --lib -- gen7_speech_preview` | 1 passed |
| `cargo test -p studio-domain --test gen7_speech_preview -- --ignored` | **1 passed** (6 previews + the listing, 37 s) |
| `cargo test -p studio-api --lib export_root` | 1 passed (pin 158) |
| `export-bindings --check` | in sync |
| `npx vitest run` (full) | **36 files, 231 passed** (Gen7BankCard 2) |
| `npx tsc --noEmit` | clean |

## 5. Limitations

- The music preview decodes the **whole** clip (a 100-second stereo track is a ~18 MB WAV in the
  reply); no streaming or range decode. Acceptable for a diagnostic; recorded.
- Which music sample plays where (the music box, research handoff 10) is not joined; the card lists
  the bank only. `music_samples` on X360 09–12 and PS3 09 / 10 / 12 / 13 refuses (EA-XMA or
  unsurveyed, R80).
- The identity line shows an archive-member bank as its full source description (archive, member,
  offset, stored form) — long but exact.

## 6. Cleanup

Scratchpad scripts deleted; no temp roots; no processes.

## 7. Next action

With P4.5 / P4.6 closed, the audio queue's remaining rows all need contracts (PS4 announcer codec
census; `.abk` S10A reading for horn banks; any replacement / mutation). The next ready roadmap
item is **Phase 3 players** in the parity roadmap's order, starting from the existing player
inspection foundations — unless the morning review returns a League / Team defect.
