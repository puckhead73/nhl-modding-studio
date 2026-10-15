# Receipt 24 (P4.8) — archive-backed PS3 sound banks, the arena chant wording, and horn-to-bank links

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing listened to; nothing
game-verified.** HEAD `8d60a6d` (unchanged). Claim: `dev-p48-archive-sound-banks`. Clock: 02:40 → 03:12 (real clock)
(MDT, 26 Sep 2026). Direction: [packages 20–22 review](../../validation/development-20260926-packages20-22-review.md),
items after F1 / F2.

## 1. Plain-language summary

The sound-bank card now works on PS3 NHL 09, 10, 12 and 13. Those titles keep their banks inside
their game archives rather than in a loose folder. Studio reads each bank and stream straight from
the title's own archive members (NHL 10's four loose stream files included), with the same stream
identity check as the loose titles, and records where each bank came from and in what stored form.
Every one of the research's framed streams frames here too; a handful of samples the research
actually decoded were decoded again and give the same sample counts. Everything else previews by
the same route but has not been individually witnessed, and the card says so. Xbox 360 NHL 09 stays
refused (its packets use a different header and no Xbox decoder has been chosen).

The arena page no longer calls a chant id a "candidate" bank. It now says what is measured (chant
banks are loader assets listed by number; twelve are club chants; only the Finnish arena's chant id
equals a club-bank number, and that arena's horn ids carry the same 61) and what is not (nothing
that selects a chant bank has been traced). Goal and period horns whose numbered bank was found now
link to that bank in the sound-bank browser; chant and audio-id values never link.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-domain/src/gen7_sfx_banks.rs` | per-title `Layout` (loose; PS3 09 / 10 / 12 / 13 archive members per R98 §1; X360 09 refused); `resolve_sources` (loose files + EB / BIG4 members by directory); `load_from` (unwraps RefPack / chunkzip bank members, refuses differing duplicate copies, records wrapper + stored / decoded SHA-256); `stream_for` (the single `<stem>.ast` copy, case-insensitive — NHL 10 ships `playerCalls.ast`); `frame_stream` reads the member in place, checks `S10S` + id on the same handle, refuses a wrapped stream member, bounds the walk to the member extent; per-title evidence note (R98 structure vs R100 decoded vectors) |
| `crates/tdb-snd/src/abk.rs` | `walk_stream_in` (reader walk bounded to `[base, base + len)`); unit test extended (agreement with the slice walk inside a container, block-crosses-end and no-terminal refusals) |
| `crates/tdb-archive-big/src/big4.rs` | `BoundBig4::read_directory` — bind's header / directory / sidecar checks without hashing the whole archive (the first census run read 30 GB because every lookup hashed `data0.big` / `audio.big`); `bind` unchanged (now `sha_file` + the shared parse) |
| `crates/studio-domain/src/gen7_speech_preview.rs` | `list_members` (new); `locate_member` / `list_members` use `read_directory` |
| `crates/studio-domain/src/gen7_horn_sources.rs` | chant note per R97 §1.4 (never resolved, never linked; same-numbered bank named as a number match); `bank_file` for goal / period horns only when the bank was found and the listing covers the title; unit test asserts no "candidate" / silence / fallback wording; the probe now reads PS3 12 / 13 (refusal lifted on R98 §1) and `audio.big` (PS3 10), names each member by its real path (NHL 09's are `audio/sfx/ram/…`, previously printed as `audio/sfx/…`), and lists BIG4 members with `read_directory` instead of hashing `data0.big` per arena view |
| `crates/studio-domain/src/arena_venue.rs`, `crates/studio-api/src/{dto,convert,registry}.rs` | `VenueFieldView.bank_file` / `VenueFieldDto.bank_file`; `Gen7SfxBankTreeDto.{bank_wrapper, bank_stored_sha256, bank_copies}` |
| `app/src/bindings/{VenueFieldDto,Gen7SfxBankTreeDto}.ts` | generated (`--write`) |
| `app/src/components/ArenaVenueCard.tsx` + test | horn link to `/app/database/leagues/commentary?bank=`; chant label "number only — selection not traced"; tests under `MemoryRouter` |
| `app/src/components/Gen7SfxBankCard.tsx` + test | archive-member provenance line; coverage fixture now X360 09 |
| `crates/studio-domain/tests/gen7_sfx_banks.rs` | coverage updated; `archive_backed_ps3_titles_list_frame_and_decode_from_their_own_members` |

## 3. Evidence

All runs in a separate target directory (`E:/nhlms-target-check`, owned; removed at cleanup) while the
F1 / F2 aggregate held the main one.

- [`p48-sfx-real.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p48-sfx-real.log) — `gen7_sfx_banks` 5 / 5 with `--include-ignored` (real
  trees, read-only):

  | Title | banks | present records (R82 §2) | stream entries framed / R98 | loops | stream files, id match | R100 vector |
  |---|---|---|---|---|---|---|
  | PS3 NHL 09 | 186 | 2 594 | 506 / 506 | 2 | 7 / 7 | playercalls 342 → 81 768 samples, 86 blocks, `data0.big:audio/sfx/streams/playercalls.ast` |
  | PS3 NHL 10 | 199 | 2 748 | 1 108 / 1 108 | 4 | 7 / 7 | playercalls 382 → 49 382, 51 blocks, loose `playerCalls.ast` |
  | PS3 NHL 12 | 219 | 4 697 | 3 719 / 3 719 | 9 | 15 / 15 | playercalls 0 → 33 207 (34 blocks); sfx 92 → 133 749 (114 blocks), `nocache.big` |
  | PS3 NHL 13 | 213 | 5 117 | 3 708 / 3 708 | 9 | 15 / 15 | playercalls 0 → 33 207 (34 blocks), `cacheaudio.big` |

  Every bank a single copy; stored SHA-256 prefixes of the vector banks equal R98's; the loose PS3 /
  X360 NHL 15 pins and the F1 identity regression unchanged.
- **Red found and fixed during the run:** NHL 09's cc 1 loop record was marked previewable in the tree
  while the preview route refused it (loose NHL 15's loops are all cc 3, which hid it). The loop gate
  now sits in the shared `preview_gate` after the channel gate, so tree and route agree; the test
  asserts it on every title.
- **Resource defect found and fixed:** the first census run read 30 GB — every member lookup went
  through `BoundBig4::bind`, which hashes the whole archive. `read_directory` removed that; the census
  now takes ~40 s in a debug build. The runaway test was stopped through its owning background task.
- [`p48-archive-sfx-banks.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p48-archive-sfx-banks.log) — `tdb-archive-big` 80 passed (1
  ignored, unchanged); `tdb-snd` abk 3 / 3 (incl. `walk_stream_in`).
- [`p48-lib-bindings.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p48-lib-bindings.log) — domain lib `gen7_horn_sources` / `arena_venue` /
  `gen7_speech_preview` 5 / 5; `export-bindings --write` (2 written) then `--check` in sync (164 roots,
  283 files).
- [`p48-horn-real.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p48-horn-real.log) — domain lib horn / venue 4 / 4; real horn sources 1 / 1:
  X360 NHL 15 goal horns 1–29 and period 1 link, unfound 41 and chant 61 do not, chant 61 names
  `061_chant.abk` as a club-chant number match; X360 NHL 09 archive-only horns resolve to
  `data0.big:audio/sfx/ram/…` and do not link (listing refuses X360 09); PS3 NHL 09 (`data0.big`, 31 goal /
  31 period / 105 chant members), 10 (`audio.big`, 31 / 31 / 118), 12 and 13 (`nocache.big`, 31 / 31 /
  118): goal horns 1–29 resolve to their member and link.
- [`p48-speech-real.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p48-speech-real.log) — speech preview real test 1 / 1 after the
  `locate_member` change.
- [`p48-api-app.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p48-api-app.log) — `studio-api` `song_catalogue` 2 passed (1 ignored);
  `tsc --noEmit` exit 0; vitest `ArenaVenueCard` + `Gen7SfxBankCard` 11 / 11.
- [`p48-api-lib.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p48-api-lib.log) — `studio-api --lib` on the new code 31 / 31, including
  `live_bindings_match_the_dto_contract`; `arena_venue` 4 / 4 and `arena_assets` 2 passed (1 ignored).
- [`p48-app-full.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p48-app-full.log) — full frontend: `tsc` exit 0; vitest 40 files / 246 tests passed.

## 4. Observations for research

- R98's JSON records every `.abk` member as `wrapper: "stored"` (stored SHA-256 = decoded), while
  R98 §1 prose and R100's table say RefPack (09 / 10) and chunkzip (12 / 13). Studio reports what it
  reads: `stored` on all 817 bank members of the four titles (agreeing with the JSON, not the prose).
- R98's per-member `records` totals (2 619 / 2 772 / 4 734 / 5 160) exceed R82 §2's present-record
  totals (2 594 / 2 748 / 4 697 / 5 117); the difference is presumably empty table slots (not verified here). Studio pins R82 and matches it exactly.

## 5. Limits

- Structure is R98's full census; decode is witnessed only on R100's named vectors (the evidence
  note on every card says so). Loop records and channel configs 3 / 5 stay refused.
- Horn links cover goal / period only, and only where the member is found and the listing covers the
  title. Which horn the game plays for a value stays card H-HORN-1 (runtime).
- X360 NHL 09 refused; no XMA decoder decision.
- Chant wording follows R97 §1.4; the untraced chant selector is unchanged research work.
