# Handoff R100 / R101 — (R100) representative diagnostic decode of the archive-backed `.ast` streams: PS3 NHL 09 / 10 / 12 / 13 decode through the R92 EALayer3 route with exact counts on 15 / 15 sub-streams from member bytes, and **X360 NHL 09 decodes through the unchanged R92 XMA route with exact counts on 3 / 3** — the ffmpeg decoder does not read the constant packet-header word, so handoff 11's "not expected to apply unchanged" is withdrawn; (R101) the kit and nameplate member names are `{style}_{teamartid}_{variant}` on jersey, pant, sock and nameplate on six titles (position-by-position value-set match against `stockteamjerseys`), the rink logo is `texlib_{artid}_{style}_{variant}`, and PS3 NHL 12's socks follow a four-number grammar whose id position is not established

Research lane, 25 September 2026, 15:05 → 15:30 MDT (real clock). Tasks 11 and 12 of [TASKS.md](TASKS.md). Claims
`research-r100-archived-decode`, `research-r101-kit-name-key-positions`. Read-only over the archives on `X:` (members decoded in memory),
`anchors/` and the session scratch (temporary `.mp3` / `.xma` / `.wav` deleted by the decode routines; the scratch directory removed).
Decoder acceptance only; nothing listened to; no WAV kept; nothing game-verified; no writer.

Machine-readable: [`data/r100-archived-decode.json`](data/r100-archived-decode.json) (probe `r100_archived_decode.py`, 18 vectors
requested, 16 decoded, 2 "no such entry" recorded), [`data/r101-nameplate-kit-key-positions.json`](data/r101-nameplate-kit-key-positions.json)
(probe `r101_nameplate_kit_key_positions.py`, six titles).

> **Note 2026-09-25 22:07 (R102 / R103, [14-correction-checkpoint-r97-r101-review.md](14-correction-checkpoint-r97-r101-review.md) §F2 / §F3):** (R100) "the ffmpeg decoder does not read the constant packet-header word"
> is narrowed to **"on these three vectors this decoder's output did not depend on the header's frame-count / metadata bits"**
> (R100b: byte-identical PCM under shipped, synthesised and zeroed header words; the frame-offset field was already 0 and stays
> untested). (R101) "settled" is withdrawn: position-set matches do not select. Per-row joins (R103): nameplate `(style, teamartid,
> variant)` resolves every row on six trees and every discriminating row — a naming association; **rink-logo positions 2–3 remain
> candidates** (16–119 rows unresolved, ~1 000+ unmatched tuples, `jerseyindex` competitive on NHL 13). Jersey / pant / sock keys are
> R56's per-row result, not this probe's. The PS3 NHL 12 "four-number sock grammar" is the two-file sock form of R56 / R74, already
> implemented by the developer's resolver — not an open mapping.

## Plain-language summary

Two follow-ups. First, a handful of streams from each archive-only title were actually decoded through the same diagnostic routes as
the loose titles, from the archive members themselves: every PlayStation 3 sample came out with exactly the predicted number of
samples, and — contrary to what the previous handoff expected — so did the Xbox 360 NHL 09 samples, because the decoder used for
previews walks the frames itself and never reads the header word that NHL 09 leaves constant. Second, the question of which number in
a jersey, pant, sock or nameplate file name is the team's art id was settled by matching each number's set of values against the
database's jersey table: the middle number is the team art id, the first is the jersey style and the last the variant, the same on
all six titles, and nameplates use exactly the same triple as jerseys. The rink-logo names also carry a style and a variant after the
art id. One title, PlayStation 3 NHL 12, names its socks with four numbers, and which of them is the team id was not established.

## 1. R100 — decode from member bytes (diagnostic)

| Tree | vector (bank, record, cc, loop) | source members | blocks / Σ samples | decode | count check |
|---|---|---|---|---|---|
| PS3 NHL 09 | `playercalls` 342, cc 0 | `data0.big` (abk + ast, stored) | 86 / 81 768 | EAL3 → `mp3float`: 146 granules → 84 096 | D1 = granules × 576 ✔; D3 (decoded + 47 − 1152 − trim = Σ) ✔ |
| PS3 NHL 09 | `sfx` 132, cc 1 | `audioboot.big` abk (~~RefPack~~ stored — corrected 2026-09-26 03:42, R110b) + `data0.big` ast | 3 207 / 3 704 400 | 6 436 granules → 3 707 136 | ✔ / ✔ |
| PS3 NHL 09 | `crowds` 0, cc 3 **loop stub** | `audioboot.big` + `data0.big` | 1 / 29 | 2 sub-streams × 6 granules → 3 456 each (warm-up only, rms 0) | D1 ✔ (stub rule: output = PCM only, as R96 V6) |
| PS3 NHL 10 | `playercalls` 382, cc 0 | loose `.ast` (title ships it loose) + `audio.big` abk | 51 / 49 382 | 90 → 51 840 | ✔ / ✔ |
| PS3 NHL 10 | `sfx` 449, cc 1 | `audio.big` | 3 207 / 3 704 400 | 6 436 → 3 707 136 | ✔ / ✔ |
| PS3 NHL 10 | `crowds` 144, cc 3 | loose `.ast` | 1 102 / 1 270 528 | 2 × 2 210 → 1 272 960 each | ✔ / ✔ (per sub-stream PCM = 47; the probe's first print applied the stream-level 94 and printed "False" — arithmetic, not data) |
| PS3 NHL 10 | `crowds` 121, cc 3 loop stub | loose | 1 / 32 | 2 × 6 → 3 456 | ✔ |
| PS3 NHL 12 | `playercalls` 0, cc 0 | `nocache.big` (~~chunkzip~~ stored abk — corrected 2026-09-26 03:42, R110b; stored ast) | 34 / 33 207 | 62 → 35 712 | ✔ / ✔ |
| PS3 NHL 12 | `sfx` 92, cc 1 | `nocache.big` | 114 / 133 749 | 238 → 137 088 | ✔ / ✔ |
| PS3 NHL 12 | `crowds` 6, cc 3 | `nocache.big` | 1 102 / 1 270 528 | 2 × 2 210 → 1 272 960 | ✔ / ✔ |
| PS3 NHL 12 | `crowds_winterclassic` 9, cc 3 loop stub | `nocache.big` | 1 / 29 | 2 × 6 → 3 456 | ✔ |
| PS3 NHL 13 | `playercalls` 0, cc 0 | `cacheaudio.big` ast + `nocache.big` abk | 34 / 33 207 | 62 → 35 712 | ✔ / ✔ |
| PS3 NHL 13 | `sfx` 0, cc 1 | `cacheaudio.big` | 1 091 / 1 260 672 | 2 194 → 1 263 744 | ✔ / ✔ |
| PS3 NHL 13 | `crowds` 18, cc 3 | `cacheaudio.big` | 197 / 228 096 | 2 × 400 → 230 400 | ✔ / ✔ |
| PS3 NHL 13 | `crowds` 113, cc 3 loop stub | `cacheaudio.big` | 1 / 29 | 2 × 6 → 3 456 | ✔ |
| **X360 NHL 09** | `playercalls` 254, cc 0 | `data0.big` (= loose, byte-identical) | 10 / 49 382 | **raw packets** → `xma2`: **49 382** (rms 6 048, peak 32 160) | decoded = Σ samples ✔ |
| X360 NHL 09 | `crowds` 41, cc 1 | `audioboot.big` abk + `data0.big` ast | 88 / 450 000 | raw → **450 000** (rms 9 081) | ✔ |
| X360 NHL 09 | `colors1` 69, cc 0 | `data0.big` | 14 / 69 662 | raw → **69 662** (rms 3 221) | ✔ |
| not decoded | PS3 NHL 09 `crowds` cc 3 one-shot; X360 NHL 09 `sfx` cc 3 one-shot | — | — | no such entry in those banks (recorded) | — |

**X360 NHL 09, second reading (diagnostic construction, labelled):** every 0x800 packet's header word rewritten to the XMA2 layout with
the R98b walked frame count (offset bits kept, metadata 1) → identical decoded counts and identical rms / peak to the raw reading on
3 / 3 (rewrites: `2 → 10` on 110 packets, `2 → 8` / `2 → 7` on one each). Consequence: the ffmpeg `xma2` decoder locates frames by their
15-bit lengths and does not depend on the header's frame-count / metadata fields — the same header-independence R92 measured with the
+1000 RIFF control. **Handoff 11 §3's "the R92 route is not expected to apply unchanged" is withdrawn**: the route applies unchanged
for decoding; the header variant still matters for any *encoder* (REPLACEMENT_READINESS note) and for any parser that trusts the header
count (R91's sample-word rule needs the frame walk on NHL 09, as R98b established). The 87 residual middle blocks of R98b are unaffected
by this decode (not re-tested per block).

Not established by R100: audible correctness (nothing heard), the console's start / trim behaviour, anything about replacement.

## 2. R101 — key positions in kit / nameplate / rink-logo member names

Method: for each family the set of values at each name position is compared with every integer column of `stockteamjerseys`
(+ `customteamjerseys` where present) and with `exhibitionteams.artid`; two fractions per (position, column): the share of the name
values that occur in the column, and the share of the column's values that occur in the names.

| Family (member name) | position 1 | position 2 | position 3 | titles |
|---|---|---|---|---|
| jersey `rendering/jersey/texlib_{1}_{2}_{3}.rx2` | = `stockteamjerseys.style` (1.00 / 1.00; 2 values) | ⊂ `stockteamjerseys.teamartid` (names → column 0.925–0.995; column → names **1.00**) | ⊂ `stockteamjerseys.variant` (0.67–1.00; column → names 1.00; 19–21 values) | Legacy X360 / PS3, 15 X360 / PS3, PS3 12, PS3 13 |
| pant `rendering/pant/texlib_{1}_{2}_{3}.rx2` | same | same | same | same 6 |
| sock `rendering/sock/sock_{1}_{2}_{3}_cm.rx2` | same | same | same | Legacy X360 / PS3, 15 X360 / PS3, PS3 13 (5) |
| **nameplate** `rendering/jersey/name_{1}_{2}_{3}_cm.rx2` | same | same | same | same 6 — **the candidate of handoff 12 §3 is settled: `name_{style}_{teamartid}_{variant}_cm`**; the nameplate tuple set equals the jersey tuple set exactly on NHL 15 (both platforms) and differs by 1–3 tuples on Legacy / 12 / 13 |
| rink logo `rendering/logo/texlib_{1}_{2}_{3}.rx2` | ⊂ `exhibitionteams.artid` (column → names 0.97–1.00; names → column 0.61–0.63 — 128–133 asset-only ids per title, as handoff 04) | = `stockteamjerseys.style` (1.00 / 1.00) | ⊂ `stockteamjerseys.variant` (0.67–1.00 / 1.00) | 6 — the rink logo is **per style and variant**, not one file per art id (1 700–1 928 members for 337–348 ids) |
| PS3 NHL 12 sock `rendering/sock/sock_{1}_{2}_{3}_{4}_cm.rpsgl` (1 946) + `_am` (18) + no-suffix (40) + `sock_{1}_{2}_nm` (4) | **not established** — a four-number grammar (the NHL 11 / 12 mirror-pair titles, R74 / R75); which number is the team art id was not tested (the three-number regex does not match) | | | PS3 12 only (PS3 13 is back to three numbers) |

The DB triple (`style`, `teamartid`, `variant`) reproduces 657 / 683 jersey name tuples on NHL 15 (both platforms); the 26 others and the
"names → column < 1.00" remainders are kit files for art ids without a jersey row on that title (asset-only ids), not a grammar
exception. Column names are the metadata names of the title's `stockteamjerseys` (`style`, `teamartid`, `variant`; `fevariant` matches
`variant`'s value set on NHL 15).

## 3. Consequence for the ledger

- Handoff 12 §3: nameplate id position **established** (position 2 = `teamartid`); the rink-logo grammar is `texlib_{artid}_{style}_{variant}`;
  the sock grammar on PS3 NHL 12 (and, untested, NHL 11) is a separate four-number family — residual.
- Handoff 11 §3: X360 NHL 09 decodes through the existing diagnostic XMA route (counts exact, 3 / 3); the header variant is an
  encoder / parser concern only.
- Queue rows 9 / 10 updated by dated note; nothing here is a writer, a replacement or runtime evidence.

```
python local/runs/overnight-research-2026-09-25/probes/r100_archived_decode.py docs/research/overnight-2026-09-25/data/r100-archived-decode.json --scratch <scratch dir>
python local/runs/overnight-research-2026-09-25/probes/r101_nameplate_kit_key_positions.py docs/research/overnight-2026-09-25/data/r101-nameplate-kit-key-positions.json
```
Executed once each; one scratch listing of the PS3 12 / 13 sock stems (deleted).
