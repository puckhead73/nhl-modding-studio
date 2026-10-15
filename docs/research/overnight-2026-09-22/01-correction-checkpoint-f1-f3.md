# Handoff R65 — Correction checkpoint F1–F3: the Xbox slot-extent contract is withdrawn to what it measures, the EA-MP3 reconstruction is restated as a hypothesis, and the GNF coverage wording is reconciled

Research lane, 22 September 2026, first package of this campaign. Claim `research-r65-corrections-f1-f3`. Applies
[the coordinator's delivery review of 2026-09-22](../../validation/research-20260921-pm-delivery-review.md) (F1, F2, F3) to the
2026-09-21 pm campaign. **No raw evidence is changed**: every measurement, table and JSON stays exactly as published; the corrections are
dated notes, restated conclusions and rewritten user-facing text. No application file, matrix or developer receipt is touched.

## Plain-language summary

Three things published yesterday said more than the measurements support, and they are corrected here before any new work.

1. The Xbox 360 texture measurement (handoff 17) found the last non-zero byte inside each texture's reserved slot. That is a *lower*
   bound on the bytes a texture uses — valid texture data can be zero, and the measurement was taken inside a region that is already
   8 KiB-aligned. It therefore cannot prove that a resized texture will fit, that the biggest observed tail is what to allocate, or
   anything at all about *where* each mip level sits. Those claims are withdrawn. The distributions themselves stay, relabelled.
2. The speech-format measurement (handoff 18) established the block sizes, the prefixes, the one walkable frame per block and the failed
   decode. It did **not** establish that eight frame headers were "removed", and it does not follow that synthesising headers will decode
   the class, nor that the same routine solves the last-generation EALayer3 class. The structure stands; the explanation becomes a
   candidate hypothesis, and the user-facing text becomes "not supported yet / the tested conversion failed".
3. The PS4 texture census (handoff 03) is complete on all seven titles, but its own reproduction footer and the queue still described a
   run in progress with only NHL 20 done and only NHL 15 carrying multi-stream files. Those are reconciled, and the qualification is
   narrowed: the rules were checked on the *first* stream of each file, so a multi-stream file is not resize-qualified by that pass.

## F1 — Xbox 360 `RW4xb2` slot extent (handoff 17 / R60)

**What R60 measured, restated exactly.** For each descriptor whose format is modelled, the probe read the pixel slot named by the file
table and recorded (a) the slot length, (b) the offset of the last non-zero byte inside that slot, (c) that offset rounded up to 4 KiB.
The slot lengths are all multiples of 8 KiB (89 267 / 89 267 descriptors over NHL 09 / 10 / 13 / Legacy). The non-zero extent never
exceeded its own slot.

**Why that last sentence is not evidence.** The extent is computed *inside* the slot: it is bounded by the slot by construction, so
`slot_lt_extent4k == 0` is a property of the measurement, not of the data. Independently, a texel whose bytes are zero is a valid texel,
so the last non-zero byte is a **lower** bound on the addressed region, and the maximum over a shape's population is a lower bound on
that shape's allocation — never an upper bound and never a sufficient allocation formula. And a byte count, however exact, says nothing
about sub-level positions: an allocation-size oracle records allocations.

**Withdrawn** (dated 2026-09-22; the original sentences stay in place in handoff 17 under a correction note):

| Withdrawn statement | Where | Replacement |
|---|---|---|
| "the developer's resize rule — fit the existing slot or regrow it — is safe as written" | 17 plain-language, §1 | The measurement neither supports nor refutes the rule. The rule needs an independent per-level layout contract. |
| "no shipped slot is smaller than its content, so keeping the slot length never truncates" | 17 §1 | The non-zero extent of shipped content never exceeded its own slot, which is a measurement boundary. Truncation safety is unestablished. |
| "the **max** is the value to allocate" (§3 tail-page table) | 17 §3 | The maxima are the largest *observed non-zero extents* per shape class: lower bounds on the allocation, usable as a floor for a candidate, not as an allocation rule. |
| "`M_tile` exact on the single-level population" → "write single-level textures freely" | 17 §2 / §4, queue row 17 | Aggregate extent agreement for single-level textures is *consistent with* the model; it does not establish the layout, because a larger prediction may contain valid zero bytes or padding. |
| The five models' "over" counts read as refutations | 17 §2 | A model predicting **less** than the observed non-zero extent is genuinely contradicted (content exists past its prediction). A model predicting **more** is **not** contradicted by this measurement. Only the "under" columns are evidence; the "over" columns are neutral. |

**Preserved, with accurate labels:** the slot-length distribution per shape (handoff 02's oracle — an allocation record), the 8 KiB
granularity, the per-shape non-zero-extent distributions of §3, the identity of those distributions across NHL 09 / 10 / 13 / Legacy, and
the 369 descriptors whose extent falls below base + large-level tiles. `data/r60-rw4xb2-slot-data-extent.json` is unchanged.

**Consequence for the developer.** Do not gate, enable or qualify changed mipped output on R60. Before mipped output is written for any
shape — including shapes EA ships — an independent per-level address / layout contract and a test that distinguishes *misplaced* mips
from *equally sized* mips are required. Presence in a size oracle is not that test. Audit the existing resize route against that contract
once it exists; do **not** disable independently established layouts (the base-level `DW12` tile rule of handoff 02 §1, measured
separately) on the strength of this withdrawal. Establishing that contract is this campaign's first priority.

**Addendum, found while building the successor probe (R66).** r60 read each slot from the file start instead of from `hdr_size + off`
(pixel-data entry offsets are relative to the end of the RX2 header, `hdr_size` = u32 BE at `0x44`, 0x1000 on every file checked). Its
windows were therefore displaced by 4 KiB into the header and descriptors, so the §1 / §3 extent tables are not measurements of the slot
at all. This is recorded as a dated defect note in handoff 17; the JSON stays as the record of what ran. r42 / r44 (descriptor reads,
header-resident) are unaffected, and Studio's own reader adds `hdr_size` (`texlib.rs`, `buf_start = hdr_size + data_offset`).

## F2 — EA-MP3 (handoff 18 / R61)

**Established and kept:** codec `0x0b` on NHL 15–17 speech, 48 000 Hz mono, `samples-in-block` = 10 368 on every block but the last and
Σ = SNR `num_samples` (24 / 24 vectors); a 6-byte prefix on the first data block and a 2-byte prefix on later blocks, each followed by a
valid MPEG-1 Layer III header; exactly one conventionally walkable frame per block; the attempted ffmpeg conversion failed or returned
2 304 samples (24 / 24); 115–242 false `FF Ex` syncs per sample, so a sync scan is not a frame finder.

**Withdrawn as stated:** that the other eight frames "have had their headers removed"; that 10 368 = 9 × 1 152 plus one walkable frame
identifies nine frames or their boundaries; that synthesising eight headers from the first is *the* next step rather than *a* candidate;
and that the same routine will also solve gen7 EALayer3 v1. The probe has not identified any granule past the first, its side information
or its payload boundary.

**Restated:** the block declares 10 368 samples, which is consistent with nine Layer III granule pairs, and only one conventional frame
header is present. Whether the remaining audio is stored as headerless frames, as a re-packed granule container, or in a form that is not
frame-structured at all **is not established**.

**User-facing and queue text becomes:** "EA-MP3 (codec `0x0b`, NHL 15–17): this speech format is not supported by Studio yet; the tested
conversion failed. Duration, sample rate, block count, per-block bitrate and raw export are available." Not "the stored stream omits the
per-frame MPEG headers" — that is the hypothesis, not the observation.

**Method requirements carried into this campaign's audio work:** a decoder exit code is not a result — declared duration / sample count,
error text, channel and rate, and an independent reference implementation where one is available must all be checked; each codec class is
tested on its own vectors, and the gen7 EALayer3 relationship is established before it is assumed. No audio writer without an
encoding / alias / container contract. Listening review stays with the user.

## F3 — GNF coverage reconciliation (handoff 03 / R43, queue row 14)

- Handoff 03's reproduction footer said the full run was in progress with NHL 20 complete. **Corrected:** all seven titles completed
  (JSONs `r43-gnf-nhl15/16/17/18/19/20/21.json`); the footer now states the finished run.
- Queue row 14 named only NHL 15's 1 115 multi-stream files. **Corrected:** NHL 15 (1 115) **and NHL 19 (5 342)**, 6 457 files; the other
  five titles ship none.
- **Narrowed qualification.** The census walked each file's first `GNF ` stream. A pass is therefore a *first-stream* rule check. A
  multi-stream file is **not** resize-qualified by it: a writer must preserve every stream or refuse the file by name. First-stream
  success is not whole-file qualification.
- **Shipped maxima are observations.** 8 192 × 4 096 and 14 levels are the largest shipped values seen, not runtime limits; the field
  bounds and the hardware limits are the separate quantities of handoff 02 §4.
- **BC6H.** Byte-size agreement for format `0x28` establishes the 16-byte block size for size arithmetic. It is not evidence that a BC6H
  payload decodes correctly, that an encoder is correct, or that pixels round-trip.

## Changed paths

Dated notes only, no deletions: `docs/research/overnight-2026-09-21-pm/` handoffs 03, 17, 18 (top note + in-place annotations),
`DEVELOPER-QUEUE.md` rows 14 / 17 / 18, `00-ATLAS.md` rows 03 / 17 / 18 + Q-R60-1 / Q-R28-1 / Q-R43-1, `STATUS.md` (closing section).
New: this campaign's `00-ATLAS.md`, `STATUS.md`, `DEVELOPER-QUEUE.md`, this handoff, claim `research-r65-corrections-f1-f3`.
Unchanged: every `data/*.json`, every probe, every other handoff.
