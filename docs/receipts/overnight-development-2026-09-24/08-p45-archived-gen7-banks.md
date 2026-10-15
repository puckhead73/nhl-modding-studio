# Receipt 08 (P4.5 iii) — gen7 speech banks and streams that ship only inside archives

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing listened to.** HEAD
`8d60a6d` (unchanged). Domain-only change; the API and UI contracts are unchanged. Claim:
`dev-p45-archived-gen7-banks`. Clock: 14:00 → 14:26 (MDT, system clock).

## 1. Plain-language summary

On PS3 NHL 09, 10, 12 and 13 the commentary sample bank is not a loose file: it sits inside one of
the audio archives (`data0.big`, `audio.big`, `audioboot.big`, `nocache.big`), compressed with
RefPack or chunkzip, while the sample streams are loose. On Xbox 360 NHL 15 / Legacy and PS3 NHL 11
the reverse can hold (the stream also exists as an archive member). The gen7 preview route now
resolves both the bank and the stream **loose first, else from the archive** — reading the archive's
file table, decoding the bank member from its stored form, and reading a streamed member in place at
its offset — instead of refusing. Nothing is copied or written; the source of each byte is named in
the preview's identity line. Verified read-only on the real PS3 NHL 13 (`audioboot.big`, chunkzip)
and PS3 NHL 09 (`data0.big`, RefPack) trees alongside the loose NHL 15 cases.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-domain/src/gen7_speech_preview.rs` | `SourceLocation` (loose / archive member with offset, size, stored form); `locate_member` (EB table or BIG4 + `.bh`); `bank_from_archives` (chunkzip / RefPack / stored); `stream_from_archives` (member read in place); **new** `bank_entries` (the resolved bank + its sample table, shared by the preview and by callers that want to list ids); `preview_sample` uses them; `bank_path` / `stream_path` in the reply now describe the source |
| `crates/studio-domain/tests/gen7_speech_preview.rs` | `--ignored` real-tree case extended with PS3 NHL 13 `pbp_eng` (archived bank) and PS3 NHL 09 `ann_eng` (archived bank); the first sample by stream offset is chosen through `bank_entries` |

## 3. Evidence ([`p45-gen7-archived-banks-real.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p45-gen7-archived-banks-real.log))

| Tree | Bank source | Stream | Sample | Result |
|---|---|---|---|---|
| X360 NHL 15 `pbp_eng` | loose (684 592 B, 45 442 samples) | loose | 58 | 144 granules → 72 frames, 82 944 decoded |
| PS3 NHL 15 `ann_eng` | loose (482 612 B, 24 876 samples) | loose | 45 | 124 granules → 124 frames (MPEG-2), 71 424 decoded |
| **PS3 NHL 13 `pbp_eng`** | `audioboot.big` member at 758 224, 708 824 B, **chunkzip** → 83 453 samples | loose (2.09 GB) | 77269 | 249 granules → 249 frames, 143 424 decoded |
| **PS3 NHL 09 `ann_eng`** | `data0.big` member at 456 104 192, 197 788 B, **RefPack** → 14 132 samples | loose (209 MB) | 2 | 60 granules → 60 frames, 34 560 decoded |

The archived-stream branch (X360 15 / Legacy `nocache.big`, PS3 11 `audiostream.big`) is
exercised by the `tdb-snd` real-vector test at the recorded member offsets (receipt 05); on those
trees the loose stream exists too, so the domain route reads it loose. No tree on this machine has
an archive-only stream, so that branch has **no real-tree domain evidence** here — recorded.

## 4. Checks

| Run | Result |
|---|---|
| `cargo test -p studio-domain --lib -- gen7_speech_preview` | 1 passed |
| `cargo test -p studio-domain --test gen7_speech_preview -- --ignored` | **1 passed** (4 trees, 31 s) |
| `cargo build -p studio-api` | clean (contracts unchanged; bindings untouched) |

## 5. Limitations

- Bank members compressed as `chunklzx` would refuse (the store route decodes it elsewhere, but no
  measured speech bank uses it — R80 sources are chunkzip / RefPack / stored).
- PS3 12 / 13 `ann_eng` banks in `nocache.big` are covered by the same code path but were not run
  tonight (the R80 census names them chunkzip; the NHL 13 `pbp_eng` case exercised the same decoder).
- The UI still shows the raw bank / stream descriptions in the identity line; no new card element.

## 6. Cleanup

Scratchpad `patch_p45_archives.py`, `patch_p45_bank_entries.py` deleted after this receipt; no temp roots; no processes.

## 7. Next action

P4.5 (ii) needs a measured codec census for the PS4 announcer / other banks (a research row) before
any route; otherwise the next ready item is the gen7 **music** bank index (measured stereo tuples,
bank table not read) — a bounded reader package if its `.sbr` form is the SND10 `SBbe` layout the
PS4 reader already parses (to be measured first, not assumed).

## 8. Addendum — music banks (same package, 14:45 MDT)

The gen7 `music_samples.sbr` was measured on the X360 and PS3 NHL 15 trees: `SBKR`, 263 entries (the R80 entry count) — the same table the speech reader parses, under `audio/music/`. `coverage` now admits `music_samples` on the R80 stereo tuples (X360 13–Legacy, PS3 11 / 14 / 15 / Legacy) and the route reads `audio/music/music_samples.{sbr,sbs}`; the real X360 and PS3 NHL 15 music sample 634 previews at 44.1 kHz **2 ch** (495 blocks, 7 914 granules, stereo side info carried as read). No UI lists music samples yet (no gen7 music catalogue exists), so the route is API-reachable only; the client type admits the stem. Receipt §7's "next action" (measure the music bank form) is thereby closed.
