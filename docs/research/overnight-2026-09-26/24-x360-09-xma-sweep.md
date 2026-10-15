# Handoff R131 (P27) — Xbox 360 NHL 09 archive-backed EA-XMA: 471 / 471 stream entries and 1 983 / 1 985 in-bank segments decode to exactly their declared sample counts; the NHL 10+ packet-header refusal must not be applied on NHL 09

Research lane, 27 September 2026, published 05:53 MDT (system clock). Audio residual A3 of [19](19-audio-admission-residuals.md), made
actionable by developer receipt 32 (the XMA2 helper is built; 83 / 83 R84 vectors bit-identical). Claim `research-p27-x360-09-xma-sweep`.
Extends R100 (3 NHL 09 vectors) and R84 (15 archived NHL 09 vectors) to every record of the title through the **unchanged** research routes.
Read-only; decoder = the local research oracle FFmpeg `8.1.1-full_build` (gyan.dev, `ffmpeg.exe` SHA-256 `09948d4c…`), **not** Studio's
helper; every WAV / PCM deleted at once; nothing listened to.

Machine-readable: [`data/r131-x360-09-ast-xma-sweep.json`](data/r131-x360-09-ast-xma-sweep.json) (per bank: entries, classes, failures),
[`data/r131b-x360-09-abk-ram-sweep.json`](data/r131b-x360-09-abk-ram-sweep.json) (186 banks, per-bank results and failures).

## Plain-language summary

Xbox 360 NHL 09 keeps its sound effects, chants and player calls inside archives, and its audio packets use an older header layout that
the other Xbox titles do not. With Studio's XMA helper now built, the question was whether NHL 09 can be previewed with the same rule the
other titles use: "decode, and refuse if the number of samples is not the declared number". Across every record of the game the answer is
yes for all but two in-bank sounds, which come out short and would therefore be refused. One check that works on NHL 10 and later — reading
the frame count from each packet's header — would wrongly reject 47 NHL 09 sounds that decode perfectly, so it must be switched off for
this title.

## 1. Results

| Family | Route | Population | Exact | Not exact | Listed only |
|---|---|---|---|---|---|
| `.ast` stream entries of the 7 banks (`playercalls`, `crowds`, `sfx`, `colors1`, `colors2`, `createplayer`, `practicerink`) | R92 / R100 raw packets → RIFF XMA2 | 471 non-loop mono / stereo entries | **471** (cc 0: 421, cc 1: 50) | 0 | 2 loop entries (cc 1, cc 3) |
| `.abk` in-bank segments, 186 banks | R84 raw segment (`w0` \| packets → 0x800) | 1 985 mono / stereo segments (1 808 non-loop, 177 loop) | **1 983** | 2: `0293_chant.abk` entry 0 (273 945 vs 288 281, −14 336 = 28 × 512), `crowds.abk` entry 4, loop (676 416 vs 705 600, −29 184 = 57 × 512) | 9 segments with > 2 channels |
| of which: segments R84's packet-header check (T2: frame count > 0 and first-frame offset < packet bits) **refuses** | same | 48 (header word decodes to offset 16 384 — NHL 09's constant header variant, R98b) | **47** (39 non-loop, 8 loop) | 1 (the `crowds` loop above) | — |

`w0 >> 2 = segment length` (R84 T1) held on every segment. Streams and banks are the archive members (`audioboot.big` / `data0.big`,
stored; loose copies byte-identical where present, R98).

## 2. Admission contract implications (developer; not a writer, not listening acceptance)

- **Use the count rule** as the refusal on NHL 09: decoded samples = declared (Σ block samples for streams; segment samples for in-bank
  records). It admits 471 + 1 983 records and refuses exactly the two short ones by name.
- **Do not apply the packet-header frame-count / offset refusal on NHL 09** (R84 T2, and the `packet_frame_count_0` control of C1): on
  this title it refuses 47 records that decode exactly. Keep it for NHL 10 → Legacy where R84 / R91 validated it.
- Loops: the in-bank loop segments decoded exactly on 176 / 177 (168 plus 8 header-variant); the loop *stub + body* preview rule of R96
  applies to `.ast` loop entries only (2 here, listed only).
- The two short records are residuals (frame-multiple deficits — a decoder stop or a declared-count convention; unexplained); refuse them.
- The research oracle is FFmpeg 8.1.1; Studio's helper is 8.1.3 (receipt 32) — a PCM-identity rerun of a sample of these records through
  the helper is the developer's regression step, not assumed here.

## 3. Reproduce

```
python -B local/runs/overnight-research-2026-09-26/probes/r131_x360_09_ast_xma_sweep.py <out.json> --scratch <dir>     (≈ 30 s)
python -B local/runs/overnight-research-2026-09-26/probes/r131b_x360_09_abk_ram_sweep.py <out.json>                    (≈ 2 min)
```
Executed: R131 once with `--limit 5` (trial, output deleted) and once in full; R131b twice (the second run also decodes the segments the
header check refuses, labelled).
