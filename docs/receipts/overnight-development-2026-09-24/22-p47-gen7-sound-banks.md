# Receipt 22 (P4.7) — gen7 sound banks: the `.abk` listing, the named picker tree, `.ast` stream framing and a diagnostic decode, on the research's measured populations

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing listened to; nothing
game-verified.** HEAD `8d60a6d` (unchanged). Claim: `dev-p47-gen7-sound-banks`. Clock: 15:10 → 22:07 (real clock at close)
(MDT, 25 Sep 2026). Direction: development audit 2026-09-25 ("consume ready ABKC / AST inspection and
diagnostic contracts within their measured populations"), after the ledger refresh below.

## 1. Plain-language summary

The PS3 and Xbox 360 games keep their crowd, horn, chant, player-call and effect sounds in `.abk`
banks with a sibling `.ast` stream file. Studio could not read either. The Speech catalogue page now
has a sound-bank card: it lists every bank of the title's loose `audio/sfx` folder with its record
counts and whether its stream file matches, opens one bank into its named "pickers" (the named
sound events that choose among samples, with each sample's weight, codec, rate, length and whether
its stream frames correctly), and on PS3 decodes a one- or two-channel sample into a clearly
labelled diagnostic preview. Xbox 360 banks list and frame but do not decode, because no XMA2
decoder has been chosen; loop records, four- and six-channel entries, archive-only titles and PS4
each say why they are not offered. Nothing writes, and nothing claims which sample the game plays.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/tdb-snd/src/abk.rs` (**new**) | `parse_bank` (header, R82 `S10A` records incl. ram segments / stream stubs / loop words, R89 pointer table B, R94 name directory C with the `u16 bank key \| u16 name key` split, sample lists with owners and empty slots, every-record-referenced-once check); `walk_stream` / `ast_id` (R91 block grammar, terminal flag, alignment); `sub_streams`; `trim_terminal_padding` + `eal3_diagnostic` (R92 construction: PCM frame, planar → interleaved, then decoder output from 1 152, trimmed to the declared count); named refusals `AbkRefusal` / `AstRefusal`; 3 synthetic unit tests (both platforms, every refusal, framing) |
| `crates/tdb-snd/src/lib.rs` | `pub mod abk` |
| `crates/studio-domain/src/gen7_sfx_banks.rs` (**new**), `lib.rs` | `coverage` (loose PS3 11 / 14 / 15 / Legacy; X360 10 → Legacy + Recompiled; archive-backed and PS4 refused in words), `list_banks`, `bank_tree` (pickers, per-entry framing), `preview_entry` (PS3 EALayer3, one sub-stream, non-loop) |
| `crates/studio-api/src/{dto,registry,commands,lib,bindings}.rs` | 6 DTOs, 3 routes / commands (`gen7_sfx_banks`, `gen7_sfx_bank_tree`, `gen7_sfx_preview`), export-root pin 158 → 164 |
| `app/src/bindings/Gen7Sfx*.ts` (6 **new**) | generated (`--write`; in sync) |
| `app/src/components/Gen7SfxBankCard.tsx` (**new**) + `.test.tsx` (4 cases); `app/src/pages/SpeechCatalogue.tsx`; `app/src/lib/client.ts` | the card: bank select, picker tree, "no preview" marker with the reason and the X / S words in the tooltip, request-on-button preview with an owned audio element released on every selection change |
| `crates/studio-domain/tests/gen7_sfx_banks.rs` (**new**) | coverage refusals; real PS3 NHL 15 and X360 NHL 15 pins (`--ignored`, read-only over `X:`) |

## 3. Evidence ([`p47-gen7-sfx-banks-real.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p47-gen7-sfx-banks-real.log))

| Pin (research source) | PS3 NHL 15 | X360 NHL 15 |
|---|---|---|
| banks / records / stream records (R89 §2) | 217 / 5 099 / 3 682 | 217 / 5 088 / 3 259 |
| sample lists (R89 §2) | 524 | — |
| `.ast` files, id word = bank 0x2c (R89 T5) | 14 / 14 | — |
| `crowds.abk` SHA-256, records / stream / ram / lists / names (R89 §3) | `4382317d…`, 204 / 86 / 118 / 44 / 311 | `c375f2f5…`, 198 / 61 / 137 / 44 / 311 |
| loop records; owner of the ambience loops | 3 (records 0 / 80 / 88, `base_value_reset_send`) | 9 |
| every one-shot stream frames to its record's count; every loop stub matches its words (R91) | yes | yes |
| `000_goal.abk`: 1 ram record owned by `serviceShutdown_msg`, 25 names, no `.ast` | yes; decodes 44 100 Hz stereo, 298 234 samples | — |
| `playercalls.abk` record 0 = PREVIEW_REVIEW #1 | 33 207 samples, 22 050 Hz, mono; WAV exactly the declared count | refused: EA-XMA framed only |

## 4. Checks

| Run | Result |
|---|---|
| `cargo test -p tdb-snd` | 15 passed (abk 3 new) |
| `cargo test -p studio-domain --test gen7_sfx_banks -- --include-ignored` | **3 passed** (real PS3 / X360 NHL 15) |
| `cargo test -p studio-api --lib` | 31 passed (bindings drift test incl.) |
| `export-bindings --write` then `--check` | 6 added; in sync |
| `npx tsc --noEmit` / `npx vitest run` | clean / **40 files, 243 passed** |
| closing aggregate (`closeout2-*.log`) | domain lib 222 passed, 0 failed, 2 ignored (`--skip zz_diag`); operation matrix 1 passed (371 s), audit CSV status identical before and after |

Repo-wide fmt / clippy remain the recorded C0.2 baseline defects; new files formatted.

## 5. Departures and corrections found while consuming

- R89 JSON `owner_examples` list only the first eight records of a list; the lists themselves are
  longer (`hiAmbColorSamplePicker02_msg` 21–29, `boardPlaySamplePicker_msg` 38–55). Not a contradiction;
  the tests pin the prefix.
- The name-directory class is the **high** byte of the big-endian flags word (R89 "byte 0").
- A record-less bank (`100_goal.abk`) carries one empty sample list; R89 counts none and so does
  the reader now.
- PS3 terminal blocks end in zero padding (R91); the R77 granule parser refuses a zero byte, so the
  decode route trims the shortest zero tail that leaves an exactly-walked payload. Interior blocks
  and ram segments are untouched.

## 6. Limitations

- Archive-backed banks (PS3 09 / 10 / 12 / 13, X360 09 — R98 / R100) are refused, not listed.
- Channel configs 3 / 5 and loop records are listed and framed, not decoded.
- Xbox 360 decode needs an XMA2 decoder decision (R84: FFmpeg is the only known implementation).
- Only the NHL 15 trees are pinned by a real run; the other measured titles are covered by the
  research census, not by a Studio run.
- The card is not yet linked from the arena page's horn / chant rows.

## 7. Cleanup

Scratchpad `patch_sfx_api.py`, `patch_sfx_api2.py`, `ps3_15_lists.txt` deleted by literal path; no
temp roots (read-only over `X:`); no processes.

## 8. Next action

The archive-backed banks through the existing `locate_member` / chunkzip / RefPack path (R98 / R100
pins on PS3 NHL 12 / 13), then the arena page's chant wording per R97 (the current "candidate" note
predates the withdrawn "only arena chant id 61" sentence).

> **Note 2026-09-26 (receipt 24):** superseded in part. The archive-backed PS3 NHL 09 / 10 / 12 / 13 sound banks are now listed, framed and previewed from their own members; a single-sub-stream loop record is gated as "no preview" in the tree as well as the route (NHL 09 had one marked previewable); the horn probe reads PS3 12 / 13 (`nocache.big`) and PS3 10 (`audio.big`) and names members by their real path (NHL 09: `audio/sfx/ram/…`); the chant note follows R97 §1.4 and never resolves a chant id to a bank. See [receipt 24](24-p48-archive-sound-banks-chant-wording-horn-links.md).

> **Note 2026-09-26 (receipt 25):** loop records and channel configs 3 / 5 now preview (per stereo pair; stub + body) — the "not offered in this route" refusals above describe the code before receipt 25.
