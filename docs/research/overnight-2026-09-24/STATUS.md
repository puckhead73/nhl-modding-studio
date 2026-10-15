# Research lane status — 2026-09-24 (fresh overnight session, Leagues and Teams) — closed 05:56 MDT; reopened for the coordinator review at 13:25 MDT

Brief: [overnight-research-2026-09-24-leagues-teams.md](../../plans/overnight-research-2026-09-24-leagues-teams.md). Codex coordinates
and reviews; the developer lane owns application code; the user decides runtime acceptance. Baseline recorded at 04:57 MDT: HEAD
`8d60a6d` on `feat/phase5-asset-tooling`, the working tree dirty from the developer campaigns (791 paths, preserved), `docs/research/`
and `local/` untracked. The developer lane ran its own 24 September campaign in parallel (claim `dev-p0-reviewed-texture-write`,
complete 05:22 MDT; receipts under `docs/receipts/overnight-development-2026-09-24/`) — no path or topic overlap with this lane. No
launches, no listening, no writes to any source tree, anchor, application path, `defs/`, binding or developer record; nothing staged,
committed or pushed.

Lane ownership this campaign: `docs/research/overnight-2026-09-24/**`, `local/runs/overnight-research-2026-09-24/**`, the
`research-r0-blocker-map-20260924` and `research-r82…r87` claims under `local/coordination/overnight-2026-09-24/claims/`, and dated
correction / supersession notes inside this lane's earlier campaigns (2026-09-22 pm handoff 12, queue, atlas, STATUS; 2026-09-20
handoff 02). Probes: Python 3.12, ffmpeg 8.1.1 (decoder oracle only). Final receipt: [FINAL-RECEIPT.md](FINAL-RECEIPT.md).

**Clock correction.** The first drafts of handoffs 01–05 and the R82–R87 claims carried estimated clock stamps running ahead of the
real clock; all were corrected to the files' modification times (05:47 and 05:53 MDT). Handoff 01 was emptied by a failed
text-encoding write at 05:23 and re-created in full at 05:47 (its header says so); its content matches the original draft plus the
two bank identities marked "filled".

## R0 — blocker map — handoff [00](00-blocker-map-leagues-teams.md) (05:16 MDT)

Queue reconciled against the developer's implemented work; every open League / Team item classed; boundaries carried; package order set.
Claim `research-r0-blocker-map-20260924`. No measurements.

## R1 (R82) — archive-backed gen7 audio coverage + `ABKC` record grammar — handoff [01](01-gen7-archive-audio-coverage-and-abk-records.md) (05:23, re-created 05:47)

Plain-language: the last "not surveyed" gen7 audio cells are measured from the archives (PS3 09 / 10 / 12 / 13 music and sound-effect
banks established under the R77 grammar; X360 NHL 09 archived banks are EA-XMA); the chant residual was a bank-record misreading —
loop records carry a loop-start word and one or two segments — and every one of 35 706 records on nine trees tiles under the corrected
grammar; 4- and 6-channel entries are interleaved stereo streams (named, not decoded); PS3 09 / 10 / 12 / 13 horn ids get the same
residual classes as the loose titles from their own anchors. Claim `research-r82-gen7-archive-audio-coverage`. Executed: r82a × 4
(three exposed misreadings, fixed), r82b smoke + full (≈ 35 min; every `sfx.abk` ram entry decoded — wider than planned, kept).

## R2 (R83 / R84) — `0x09` = EA-Speex; EA-XMA diagnostic rewrap — handoff [02](02-codec09-easpeex-and-ea-xma-preview.md) (05:33)

Plain-language: the unnamed speech codec is Speex ultra-wideband (one-byte-length frames of 640 samples, ten per block, 509-sample
delay) — read from the data, then found to agree with the pinned public decoder; an Ogg rewrap decodes through two separate ffmpeg
Speex decoders to the predicted counts on 96 / 96 vectors of 25 families (their outputs differ 7–29 % rms; recorded, not heard). EA-XMA:
each SNS block is one XMA2 packet; a RIFF rewrap decodes to the declared count on 91 / 91 vectors and a control shows the count is
bitstream-driven. Claims `research-r83-codec09-easpeex`, `research-r84-ea-xma-preview`. Executed: r83 smoke + full + rerun (single-block
rule); r84 smoke + full + the four-vector header control. Dated notes on the 2026-09-22 pm handoff 12, queue, atlas, STATUS and the
2026-09-20 handoff 02.

## R3 (R85) — audio consumers and editability boundaries — handoff [03](03-audio-consumers-and-editability-boundaries.md) (05:40)

Plain-language: the loader's own asset table names every horn / chant bank by id on all 16 trees, so the residual arena horn ids are
unlisted in the game's load list; the four-digit chant banks are goalie chants keyed by the goalie's announcer id (44 / 45 on NHL 10);
the PS4 goal-horn patch is located but not decoded; a per-class table separates what can be read / previewed from what a replacement
would need. Claim `research-r85-audio-consumers`. Executed: r85 twice (one guard); inline goalie-chant and patch-listing probes.

## R4 (R86) — Team `artid` identity graph — handoff [04](04-team-artid-identity-graph.md) (05:44)

Plain-language: a team's art id is shared by a club and its variant rows on every title from NHL 12, so the safe re-key operation is
defined on the id (all rows) rather than on one row; the DB columns and asset families it drags are listed per title family, with the
refusals and the unknowns (runtime caches, saves, PS4 vault). Claim `research-r86-team-artid-graph`. Executed: r86 once + one inline
duplicate-row query.

## R4 (R87) — team / league row allocation constraints — handoff [05](05-team-row-allocation-constraints.md) (05:52)

Plain-language: every table of every gen7 database is full to its slot count (teams, arenas, kits, roster, schedules), so creating a
team there means growing the file — an engine question; PS4 18–21 have spare slots in most tables but their NHL stat tables are full,
so only a non-NHL team could be hosted; the rows a new team needs and the value spaces it must choose from are measured. Claim
`research-r87-team-row-allocation`. Executed: r87 once.

**R5:** no package — no independent method for the packed tail arose; the write gate stays intact. **NHL 07 labels:** no new method;
not repeated. **Players / Equipment / Misc:** not started (the League / Team packages consumed the session).

## Final checkpoint — 05:56 MDT

**State.** Six handoffs (00–05), seven claims (`research-r0-…`, `research-r82…r87`, all `ready`), atlas, queue rows 1–10, cards
A-SPX-1 / A-XMA-1 / T-ARTID-1 / H-HORN-1 rev. 2, dated notes on the 2026-09-22 pm campaign (handoff 12, queue, atlas, STATUS) and the
2026-09-20 handoff 02. No raw evidence altered anywhere.

**Checks actually run.** r82a × 4, r82b smoke + full, r83 smoke + full + rerun, r84 smoke + full + header control (inline), r85 × 2,
r86 × 1 (+ inline duplicate-row query), r87 × 1; inline inventories (archives, load file, chant / goalie join, `sfx_ptc.big` listing,
column census); curl + sha256 of five pinned reference files. No listening, no launches, no writes to sources.

**Limitations.** Nothing game-verified; every decode is decoder acceptance; the Speex 509 start and the EA-XMA route are diagnostic;
cc 3 / 5 `.abk` entries and the `.ast` stream index are undecoded; the PS4 horn patch is located, not decoded; the `artid` graph's
asset families are compiled from earlier joins (Legacy witness for coverage); the creation constraints are container facts, not an
engine capability statement.

**Retained dependencies.** ffmpeg 8.1.1 (`speex`, `libspeex`, `xma2`, `mp3float`); numpy; the trees on `X:` / `H:`; the committed
anchors; the pinned vgmstream revision `4b6a02dd` (hashes recorded in handoff 02; copies deleted).

**Cleanup (verified at closeout).** Session scratchpad empty (`r82_eal3/`, `r83_speex/`, `r84_xma/`, `ref/`, three smoke JSONs, three
run logs and the two inventory scripts removed by literal path after a containment check); `__pycache__` created by this lane's imports
under the 2026-09-18 … 2026-09-24 probe directories removed (the pre-existing `pylib/Crypto/**/__pycache__` of the 09-18 campaign left
alone); `%TEMP%` holds no `r28_` / `r80_` / `r82_` / `r83_` / `r84_` copies; the four Python processes visible at closeout predate this
session and belong to other lanes (not touched); no background job of this lane running; `docs/research/` and `local/` remain
untracked; nothing staged, committed or pushed; the developer's working tree as found.

**One exact next task.** For Teams 2.6 / 2.7 on gen7: decode the `ABKC` bank tree at 0x78 on the four loose PS3 trees (sound → sample
index, and — for stream-type records — the `.ast` block offset), using the R82 record grammar as the anchor and `crowds.abk` /
`playercalls.abk` as vectors (89 vs 86 chained streams is the first discriminating count); success gives the sfx listing its stream
previews and the horn / chant banks their sound names; failure is recorded per structure tried. In parallel for the developer: the
`0x78` tree is also the last unknown before an `.abk` same-size replacement contract can be written (handoff 03 §5).

## Correction checkpoint (R88) — handoff [06](06-correction-checkpoint-campaign-review.md) (13:25 MDT)

Plain-language: the coordinator's review found four over-reaches; each is corrected in place with dated notes and a new handoff. Two
tables had quoted a "NHL 20" database that is a copy of NHL 19 — re-measured on the real NHL 20 install and NHL 19's patch 56, every
input hashed. The creation analysis had turned shipped-season stat rows into requirements — withdrawn after a membership audit (NHL and
AHL participants; European teams in the PS4 career tables). The art-id re-key is downgraded to an inspection plus an unfinished contract
(156 "free" ids already carry files on Legacy; sharing, atomic restore and PS4 references open). The audio closeout now says the bank
tree unblocks indexing, not replacement; the NHL 21 decoder outlier (0.46 correlation) is carried; the frame-length domain is 1..255;
509 is a count rule, not a game trace. Claim `research-r88-correction-checkpoint` (`ready`); claims r83 / r86 / r87 annotated. Executed:
r88 × 2 (second run excludes arena-keyed families from the occupancy union); two `eb_list.py` extractions (deleted).

**Next (authorized by the review):** R89 — `ABKC` offset 0x78 as a read / selection / stream-address contract (`crowds.abk` 86 records
vs 89 `.ast` chains first), then the highest-impact ready League / Team blocker from the corrected ledger.

## R89 — `ABKC` bank tree = read / selection / stream-address contract — handoff [07](07-abk-bank-tree-read-selection-stream-address.md) (13:47 MDT)

Plain-language: the bank's own table of contents is decoded — a pointer-fixup table (8-byte slots on PS3, 4-byte on X360), a name
directory (`_snd` / `h_` / `_msg`) and sample lists that name every one of 48 751 sample records exactly once on 2 355 banks of 11
loose trees, with the `.ast` stream offset for every streamed record (29 962 / 29 962 walk to the end-flagged block with the right
sample sum); the crowd file's 89-vs-86 count is three looping streams (stub + unreferenced body, 146 / 146 across the trees); three
corruption controls refuse on every tree. Open: stream payload decode (the `.ast` blocks do not tile under the ram-segment grammar),
the S / X words, the name hash, node classes 1 / 2 / 4, archived trees. Claim `research-r89-abk-bank-tree` (`ready`). Executed: smoke
× 1 (3 trees), full × 2 (10 then 11 trees). No writer; replacement unchanged.

## R90 — gen7 team-chant binding — handoff [08](08-gen7-team-chant-binding.md) (13:54 MDT)

Plain-language: with the bank names readable (R89), twelve of the 73 crowd-chant banks turn out to be Swedish club chants named
inside the bank (Brynäs … Timrå, frozen since NHL 10) and known by name to the crowd script on every title, yet no arena chant id and
no team / arena audio field ever addresses them (every SHL club's arena carries the generic bank 45) — the selection key is inside the
compiled crowd module, not in the database. Arena-keyed join re-measured per title; one counterexample (a Finnish arena's chant id 61
= the Linköping bank number) kept; card T-CHANT-1. Claim `research-r90-team-chant-binding` (`ready`). Executed: r90 × 2.

> **Note 2026-09-25 14:20 (R97, [overnight-2026-09-25/10-correction-checkpoint-direction-audit.md](../overnight-2026-09-25/10-correction-checkpoint-direction-audit.md)):** "no arena chant id … ever addresses them" and "the selection key is inside the compiled crowd
> module, not in the database" are withdrawn as conclusions (the Finnish arena's chant, goal-horn and period-horn ids are all 61; no
> selecting consumer traced). Claim r90 carries a `revision` note.

**Session state at 13:54 MDT:** claims r88 / r89 / r90 `ready`; scratchpad emptied (the two publish scripts deleted after use); no
background job; nothing staged, committed or pushed. **Next task (not started):** Q-R89-1 — the `.ast` block payload framing
(per-block padding / EA-frame grouping; XMA packets on X360) so the located streams can be previewed through the existing diagnostic
routes; first vector PS3 NHL 15 `playercalls.ast` stream 0 (34 blocks, 33 207 samples) versus the R77 tiling of its concatenated payload.

> **Note 2026-09-25:** Q-R89-1 is closed by the next campaign ([overnight-2026-09-25/STATUS.md](../overnight-2026-09-25/STATUS.md),
> handoff 09): framing measured on every loose `.ast` stream (R91), diagnostic decode + local previews (R92), the NHL 21 Speex outlier
> resolved as silence (R93), the name word split into two u16 keys (R94), club-chant selection narrowed to a loader-by-number gate (R95).
