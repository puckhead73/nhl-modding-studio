# Handoff 05 (28 Sep) — residual X360 EA-XMA in-bank shapes (NHL 10 → Legacy): four-/six-channel chains, two-segment loops, and packets the header check wrongly refuses

Research lane, 28 September 2026, published 05:37 MDT (system clock). Prompt package 5 (residual audio). Claim
`research-q5-recompiled-routes` (its second half).
- Read-only on the loose X360 NHL 10 / 11 / 12 / 13 / 14 / 15 / Legacy `audio/sfx/*.abk` banks.
- Decoder: the **installed pinned helper** `C:\nhlms-toolchain\out\xma2-helper-ffmpeg-8.1.3\nhlms-xma2-helper.exe` (SHA-256 `dc3533ed…`,
  pipe protocol only), fed the unchanged R84 RIFF XMA2 construction.
- Decoder output was never kept. Nothing was listened to.

Machine-readable, in `data/`:
- [`r139-xma-residual-shapes.json`](data/r139-xma-residual-shapes.json): inventory, samples with controls, sweep;
- [`r139b-xma-multistream-segments.json`](data/r139b-xma-multistream-segments.json): chunk chains;
- [`r139c-t2-packet-positions.json`](data/r139c-t2-packet-positions.json): where the header check fails.

Probes: `r139_xma_residual_shapes.py` (`faddd545…`), `r139b_xma_multistream_segments.py` (`d14affe0…`), `r139c_t2_packet_positions.py`
(`513d27ef…`).

## Plain-language summary

Studio's Xbox sound previews refuse several kinds of in-bank sounds:
- surround (four- or six-channel) sounds;
- sounds stored as a short intro plus a looping part;
- some ordinary sounds that fail a packet check.

Research decoded every one of these on seven titles with Studio's own shipped decoder. Each piece comes out at exactly the length the bank
declares:
- A surround sound is two or three stereo streams stored back to back.
- An intro-plus-loop sound is two separately encoded pieces, with the loop starting where the intro ends.
- The packets that fail Studio's check are a normal "no new frame starts here" marker. The check is simply too strict, as it was on
  NHL 09.

This is decoder evidence only. Nobody has listened to it, and which speaker each stream feeds is not established.

## 1. Inventory (EA-XMA codec 0x03 ram records; every record's segment samples sum to its declared total)

| Title | 1 seg, no loop, mono / stereo | 1 seg, loop, stereo | **2 seg, loop, stereo** | **1 seg, 4 ch** (no loop / loop) | **1 seg, 6 ch** |
|---|---|---|---|---|---|
| NHL 10 | 1 633 / 205 | 73 | 40 | 83 / 10 | 6 |
| NHL 11 | 1 648 / 213 | 74 | 47 | 83 / 10 | 4 |
| NHL 12 | 896 / 205 | 73 | 49 | 153 / 12 | 4 |
| NHL 13 | 1 342 / 200 | 63 | 52 | 152 / 12 | 1 |
| NHL 14 | 1 345 / 202 | 63 | 55 | 152 / 12 | 0 |
| NHL 15 | 1 345 / 202 | 63 | 55 | 152 / 12 | 0 |
| Legacy | 1 375 / 176 | 72 | 42 | 152 / 12 | 0 |

No mono record is looping or multi-segment. Studio currently refuses:
- looping ram records on NHL 10+;
- every multi-segment record;
- every record above two channels;
- any record with a packet that fails the header check (§4).

## 2. Two-segment records = separately encoded intro + loop body

- Every two-segment record is looping, and on all of them **`loop_start` = segment 0's samples**:
  - NHL 10: 40 / 40;
  - NHL 11: 47 / 47;
  - NHL 12: 49 / 49;
  - NHL 13: 52 / 52;
  - NHL 14 / 15: 55 / 55;
  - Legacy: 42 / 42.
- Single-segment loops have `loop_start` = 0: the whole record loops.
- **Each segment decodes exactly to its own declared count**, every segment of every record. On 8–12 records per title that holds only with
  the §4 header rule.
- Decoding the two segments' packets **concatenated** as one stream gives *more* samples than the sum. For example, `073_chant.abk` #0:
  53 258 decoded vs 4 437 + 48 138 = 52 575 declared.
- So the segments are independent streams, each with its own start. A preview decodes segment 0 and then segment 1. It must not splice the
  packets. That matches the stub + body construction R92 measured for `.ast` loops.
- **Control:** re-declaring each RIFF with +1 000 samples does **not** add 1 000 decoded samples (`output_follows_header` false on every
  sample). The exact counts come from the data, not from the header.

## 3. Four- and six-channel segments = chains of stereo sub-streams

- A four-channel (cc 3) segment is **two** `{u32 w0 (w0 >> 2 = length incl. w0) | XMA packets}` chunks. A six-channel (cc 5) segment is
  **three**. The chunks tile the segment exactly (every refused multichannel segment on all seven titles).
- **Each chunk decodes as a stereo stream exactly to the segment's declared count:**
  - four-channel: 93 / 93 on NHL 10, 93 / 93 on NHL 11, 165 / 165 on NHL 12, and 164 / 164 on each of NHL 13, 14, 15 and Legacy;
  - six-channel: 6 / 6, 4 / 4, 4 / 4 and 1 / 1 on NHL 10–13.
- Some of these need the §4 rule:
  - four-channel: 17–31 non-looping and 1–2 looping records per title;
  - six-channel: 1 record per title on NHL 10–13.

This is the same per-stream layout research measured inside X360 `.ast` blocks (C1), here inside one ram segment.

**Not established:**
- which chunk feeds which speakers (front / rear / centre-LFE);
- whether the console mixes chunk k as channels 2k, 2k + 1.

A preview may play chunk 0 alone (labelled), or all chunks as separate stereo pairs. A downmix that assumes a speaker order is a
listening decision.

## 4. The packet-header check (R84 T2; Studio `check_packet`) refuses a valid marker packet

- Studio's `tdb-snd::eaxma::check_packet` (Checked rule) refuses any packet with frame count 0 or offset ≥ packet bits.
- On NHL 10 → Legacy, the packets it refuses inside otherwise valid sub-streams are almost all **frame count 0, frame offset `0x7FFF`**
  (80–103 per title).
- One NHL 10 sub-stream has a different flagged header: frame count 2, offset 16 384 bits (= the packet size).

| Title | Sub-streams with a refused packet | 1 packet, at the tail | 1 packet, mid-stream | 2 packets |
|---|---|---|---|---|
| NHL 10 | 82 | 67 | 13 | 2 |
| NHL 11 | 79 | 65 | 13 | 1 |
| NHL 12 | 81 | 56 | 22 | 3 |
| NHL 13 | 101 | 78 | 21 | 2 |
| NHL 14 | 100 | 78 | 20 | 2 |
| NHL 15 | 101 | 79 | 20 | 2 |
| Legacy | 98 | 76 | 20 | 2 |

Every such sub-stream decodes to exactly its declared count when the packet is kept (§2, §3, and the single-segment rows below). The
`0x7FFF` value is what public XMA descriptions call "no frame begins in this packet". That meaning is not verified here beyond the exact
decode.

**Consequence for records Studio already admits by shape.** Single-segment, non-looping records refused only by this check decode exactly:

| Title | Mono | Stereo |
|---|---|---|
| NHL 10 | 30 | 10 |
| NHL 11 | 27 | 8 |
| NHL 12 | 14 | 8 |
| NHL 13 / 14 / 15 / Legacy | 36 | 7 |

That is the same false-refusal class as C26 found on NHL 09.

## 5. Contract for the developer (diagnostic previews; tuple-specific; same helper)

| # | Change | Scope measured | Regression |
|---|---|---|---|
| X1 | Checked header rule: accept a packet with frame count 0 **and** offset `0x7FFF` anywhere in a sub-stream. Keep refusing every other invalid header; the NHL 10 (2, 16 384) sub-stream stays refused by name. Admission still requires decoded = declared. | NHL 10 → Legacy ram records | a mono record with a mid-stream (0, `0x7FFF`) packet, e.g. from R139c's list; a synthetic packet with frame count 0 and offset ≠ `0x7FFF` still refuses |
| X2 | Two-segment looping stereo records: decode each segment through the raw route, then join as intro + body; loop point = segment 0's samples (label "loop start"). Refuse if `loop_start` ≠ segment 0's samples or any segment ≠ declared. | NHL 10 → Legacy (40–55 records per title) | `073_chant.abk` #0: 4 437 + 48 138; the concatenated-packets decode must not be used |
| X3 | Single-segment looping stereo / four-channel records: decode once (play once, as NHL 09). The loop covers the whole record (`loop_start` 0). | NHL 10 → Legacy | `000_chant.abk` #1 (48 888) |
| X4 | Four- / six-channel ram segments: parse the `w0` chain (chunks must tile the segment; 2 chunks for cc 3, 3 for cc 5). Decode each chunk as a stereo stream; each must equal the declared count. Present chunk 0 by default and the others as selectable pairs. **No speaker mapping claimed.** | NHL 10 → Legacy (4-ch 93–165, 6-ch 1–6 per title) | `002_chant.abk` #1 (cc 3: 90 452 × 2); `002_chant.abk` #0 (second chunk needs X1) |

**Unchanged:**
- the two short NHL 09 records stay refused by name (C26);
- Recompiled stays refused until its archive route exists (handoff 04 §6);
- NHL 11 / 13 / 14 `.ast` stream shapes are not measured here (next audio item).

## 6. Reproduce

```
python -B local/runs/overnight-research-2026-09-28/probes/r139_xma_residual_shapes.py <out.json>        (≈ 1 min)
python -B local/runs/overnight-research-2026-09-28/probes/r139b_xma_multistream_segments.py <out.json>  (≈ 2 min)
python -B local/runs/overnight-research-2026-09-28/probes/r139c_t2_packet_positions.py <out.json>       (≈ 20 s)
```
